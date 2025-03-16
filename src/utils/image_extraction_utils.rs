//! Extraction utilities
//!
//! Utilities for working with image extraction, including
//! coordinate transformation, region calculation, and pixel operations.
//! This module provides common utilities for extracting image data from
//! different TIFF organization formats (strips and tiles).

use log::{info, debug};
use std::cmp::min;
use image::{ImageBuffer, Rgb};

use crate::tiff::errors::{TiffResult, TiffError};
use crate::utils::logger::Logger;
use crate::extractor::Region;
use crate::coordinate::BoundingBox;
use crate::tiff::TiffReader;
use crate::tiff::is_geotiff_tag;
use crate::tiff::geo_key_parser::GeoKeyParser;
use crate::tiff::types::TIFF;
use crate::tiff::ifd::IFD;
use crate::io::byte_order::ByteOrderHandler;

/// Parse bounding box from string
///
/// Converts the bounding box string to a BoundingBox object.
///
/// # Arguments
/// * `bbox_str` - String representation of the bounding box
///
/// # Returns
/// A BoundingBox object or an error
pub fn parse_bbox(bbox_str: &str) -> TiffResult<BoundingBox> {
    BoundingBox::from_string(bbox_str)
        .map_err(|e| TiffError::GenericError(e))
}

/// Calculate geotransform from GeoTIFF information
///
/// Extracts pixel scale and tiepoint information from GeoTIFF tags
/// and constructs a geotransform array.
///
/// # Arguments
/// * `ifd` - The IFD containing GeoTIFF information
/// * `byte_order_handler` - Handler for interpreting byte order
/// * `file_path` - Path to the TIFF file
///
/// # Returns
/// A 6-element geotransform array [origin_x, pixel_width, 0, origin_y, 0, pixel_height]
pub fn calculate_geotransform(
    ifd: &IFD,
    byte_order_handler: &Box<dyn ByteOrderHandler>,
    file_path: &str
) -> TiffResult<[f64; 6]> {
    // Get pixel scale and tiepoint values
    let pixel_scale = GeoKeyParser::read_model_pixel_scale_values(ifd, byte_order_handler, file_path)?;
    let tiepoint = GeoKeyParser::read_model_tiepoint_values(ifd, byte_order_handler, file_path)?;

    // Verify we have enough values
    if pixel_scale.len() < 2 || tiepoint.len() < 6 {
        return Err(TiffError::GenericError("Incomplete GeoTIFF information".to_string()));
    }

    // Calculate geotransform components
    let pixel_width = pixel_scale[0];
    let pixel_height = -pixel_scale[1]; // Y scale is negative in geotransform
    let origin_x = tiepoint[3] - tiepoint[0] * pixel_width;
    let origin_y = tiepoint[4] + tiepoint[1] * -pixel_height;

    // Construct geotransform array
    let geotransform = [
        origin_x,
        pixel_width,
        0.0,
        origin_y,
        0.0,
        pixel_height
    ];

    debug!("Calculated geotransform: [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}]",
           geotransform[0], geotransform[1], geotransform[2],
           geotransform[3], geotransform[4], geotransform[5]);

    Ok(geotransform)
}

/// Determine extraction region
///
/// Based on the bounding box and GeoTIFF information, determines
/// the region to extract in pixel coordinates.
///
/// # Arguments
/// * `bbox` - The bounding box in geographic or pixel coordinates
/// * `tiff` - The TIFF file structure
/// * `reader` - TIFF reader for accessing data
/// * `input_file` - Path to the input file (fallback for file path)
/// * `logger` - Logger for recording operations
///
/// # Returns
/// A Region for extraction or an error
pub fn determine_extraction_region(
    bbox: BoundingBox,
    tiff: &TIFF,
    reader: &TiffReader,
    input_file: &str,
    logger: &Logger
) -> TiffResult<Region> {
    // Create a direct conversion region as fallback
    let direct_region = Region::new(
        bbox.min_x as u32,
        bbox.min_y as u32,
        (bbox.max_x - bbox.min_x) as u32,
        (bbox.max_y - bbox.min_y) as u32
    );

    // Check for necessary conditions for geotransform
    let has_geotiff_tags = tiff.ifds.iter().any(|ifd|
    ifd.entries.iter().any(|entry| is_geotiff_tag(entry.tag)));

    if !has_geotiff_tags || tiff.ifds.is_empty() {
        info!("No GeoTIFF tags found, using bounding box as pixel coordinates");
        return Ok(direct_region);
    }

    let ifd = &tiff.ifds[0];

    // Get byte order handler
    let byte_order_handler = match reader.get_byte_order_handler() {
        Some(handler) => handler,
        None => {
            info!("No byte order handler available, using direct coordinate conversion");
            return Ok(direct_region);
        }
    };

    let file_path = reader.get_file_path().unwrap_or(input_file);

    // Try to calculate geotransform
    match calculate_geotransform(ifd, byte_order_handler, file_path) {
        Ok(geotransform) => {
            info!("Converting geographic coordinates to pixel coordinates");
            let region = bbox.to_pixel_region(&geotransform);
            info!("Pixel region: x={}, y={}, width={}, height={}",
              region.x, region.y, region.width, region.height);
            Ok(region)
        },
        Err(e) => {
            info!("GeoTIFF conversion failed: {}, using direct coordinate conversion", e);
            Ok(direct_region)
        }
    }
}

/// Apply horizontal differencing predictor
///
/// Reverses the horizontal differencing applied during compression,
/// where each pixel value is the difference from the previous one.
/// This is common in TIFF files using Deflate or LZW compression with predictor.
///
/// # Arguments
/// * `data` - Image data to modify in-place
/// * `width` - Width in pixels
/// * `height` - Height in pixels
pub fn apply_horizontal_predictor(data: &mut [u8], width: usize, height: usize) {
    for row in 0..height {
        let start = row * width;
        let end = min(start + width, data.len());

        for i in (start + 1)..end {
            data[i] = data[i].wrapping_add(data[i - 1]);
        }
    }
}

/// Copy pixel data to the output image buffer
///
/// Maps a single pixel from the source data to the output image,
/// handling region offsets. This function performs all necessary bounds checking.
///
/// # Arguments
/// * `data` - Source image data
/// * `image` - Output image buffer
/// * `global_x` - Global X coordinate in the original image
/// * `global_y` - Global Y coordinate in the original image
/// * `data_idx` - Index in the data array for this pixel
/// * `region` - Region being extracted
///
/// # Returns
/// `true` if the pixel was copied, `false` if it was outside the region or data
pub fn copy_pixel(
    data: &[u8],
    image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    global_x: u32,
    global_y: u32,
    data_idx: usize,
    region: Region
) -> bool {
    // Skip pixels outside our region
    if global_x < region.x || global_x >= region.end_x() ||
        global_y < region.y || global_y >= region.end_y() {
        return false;
    }

    // Skip if data index is out of bounds
    if data_idx >= data.len() {
        return false;
    }

    // Calculate buffer coordinates
    let buf_x = global_x - region.x;
    let buf_y = global_y - region.y;

    // Copy the value (grayscale to RGB)
    let value = data[data_idx];
    image.put_pixel(buf_x, buf_y, Rgb([value, value, value]));

    true
}

/// Check if a given point is within an extraction region
///
/// A simple utility to check if a pixel is within the extraction region.
///
/// # Arguments
/// * `x` - X coordinate to check
/// * `y` - Y coordinate to check
/// * `region` - Region to check against
///
/// # Returns
/// `true` if the point is within the region, `false` otherwise
pub fn is_in_region(x: u32, y: u32, region: &Region) -> bool {
    x >= region.x && x < region.end_x() && y >= region.y && y < region.end_y()
}

/// Calculate image buffer coordinates from global coordinates
///
/// Converts coordinates in the original image space to coordinates in the
/// extraction buffer.
///
/// # Arguments
/// * `global_x` - X coordinate in the original image
/// * `global_y` - Y coordinate in the original image
/// * `region` - Extraction region
///
/// # Returns
/// (x, y) coordinates in the output buffer
pub fn calc_buffer_coords(global_x: u32, global_y: u32, region: &Region) -> (u32, u32) {
    (global_x - region.x, global_y - region.y)
}