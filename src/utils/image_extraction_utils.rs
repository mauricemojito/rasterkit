//! Extraction utilities
//!
//! Utilities for working with image extraction, including
//! coordinate transformation, region calculation, and pixel operations.
//! This module provides common utilities for extracting image data from
//! different TIFF organization formats (strips and tiles).

use log::{info, debug, warn};
use std::cmp::min;
use std::path::Path;
use image::{DynamicImage, ImageBuffer, Rgb};

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
use crate::utils::coordinate_transformer;

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

/// Convert coordinates from any CRS to pixel coordinates using geotransform
///
/// This is a more generic function that handles coordinate transformation for
/// any CRS, not just specific cases like WGS84 to Web Mercator.
///
/// # Arguments
/// * `bbox` - Bounding box in source CRS
/// * `geotransform` - Geotransform array from the GeoTIFF
/// * `img_width` - Image width in pixels
/// * `img_height` - Image height in pixels
/// * `source_epsg` - Source CRS EPSG code
/// * `target_epsg` - Target CRS EPSG code (from the image)
/// * `radius_meters` - Optional radius in meters for fallback sizing
///
/// # Returns
/// A Region for extraction
pub fn generic_crs_to_pixel_region(
    bbox: &BoundingBox,
    geotransform: &[f64],
    img_width: u32,
    img_height: u32,
    source_epsg: u32,
    target_epsg: u32,
    radius_meters: Option<f64>
) -> Region {
    info!("Converting coordinates from EPSG:{} to EPSG:{}", source_epsg, target_epsg);

    // Special case for WGS84 to Web Mercator (EPSG:4326 to EPSG:3857)
    // This is a common case and we have optimized code for it
    if source_epsg == 4326 && target_epsg == 3857 {
        return convert_wgs84_to_web_mercator(bbox, geotransform, img_width, img_height);
    }

    // For same CRS, simple conversion
    if source_epsg == target_epsg {
        return convert_same_crs_to_pixels(bbox, geotransform, img_width, img_height);
    }

    // For other CRS combinations, we need more sophisticated transformation
    // This could be implemented with PROJ.4 or similar library
    // For now, we do our best with what we have

    // Different CRSes but we'll do our best to transform
    let transformed_bbox = try_transform_bbox(bbox, source_epsg, target_epsg);
    let region = convert_same_crs_to_pixels(&transformed_bbox, geotransform, img_width, img_height);

    // Check if region is reasonable and adjust if necessary
    let adjusted_region = adjust_region_to_image_bounds(
        region,
        img_width,
        img_height,
        radius_meters,
        geotransform
    );

    info!("Generic CRS conversion result: ({}, {}) with size {}x{}",
        adjusted_region.x, adjusted_region.y, adjusted_region.width, adjusted_region.height);

    adjusted_region
}

/// Try to transform a bounding box between coordinate systems
///
/// # Arguments
/// * `bbox` - Source bounding box
/// * `source_epsg` - Source CRS EPSG code
/// * `target_epsg` - Target CRS EPSG code
///
/// # Returns
/// A transformed bounding box
fn try_transform_bbox(bbox: &BoundingBox, source_epsg: u32, target_epsg: u32) -> BoundingBox {
    // In a real implementation, we'd use PROJ.4 or similar library
    // For now, we do a basic transformation for common cases

    let mut transformed = bbox.clone();

    // Case: WGS84 (EPSG:4326) to any projected system
    if source_epsg == 4326 {
        // For arbitrary projected CRS, scale the coordinates
        // This is very approximate and only works for small areas
        let center_lat = (bbox.min_y + bbox.max_y) / 2.0;
        let meters_per_degree_lat = 111_320.0; // approx meters per degree latitude
        let meters_per_degree_lon = 111_320.0 * f64::cos(center_lat * std::f64::consts::PI / 180.0);

        // Scale to meters (very approximate)
        transformed.min_x = bbox.min_x * meters_per_degree_lon;
        transformed.max_x = bbox.max_x * meters_per_degree_lon;
        transformed.min_y = bbox.min_y * meters_per_degree_lat;
        transformed.max_y = bbox.max_y * meters_per_degree_lat;
    }

    // Return our best attempt at transformation
    transformed
}

/// Convert coordinates in the same CRS to pixel coordinates
///
/// # Arguments
/// * `bbox` - Bounding box in the CRS
/// * `geotransform` - Geotransform array from the GeoTIFF
/// * `img_width` - Image width in pixels
/// * `img_height` - Image height in pixels
///
/// # Returns
/// A Region for extraction
fn convert_same_crs_to_pixels(
    bbox: &BoundingBox,
    geotransform: &[f64],
    img_width: u32,
    img_height: u32
) -> Region {
    debug!("Converting coordinates to pixels using direct geotransform");

    // Extract geotransform components
    let origin_x = geotransform[0];
    let pixel_width = geotransform[1];
    let origin_y = geotransform[3];
    let pixel_height = geotransform[5]; // Usually negative

    // Calculate pixel coordinates
    let min_x_pixel = ((bbox.min_x - origin_x) / pixel_width).floor() as i64;
    let max_y_pixel = ((bbox.min_y - origin_y) / pixel_height).floor() as i64;
    let max_x_pixel = ((bbox.max_x - origin_x) / pixel_width).ceil() as i64;
    let min_y_pixel = ((bbox.max_y - origin_y) / pixel_height).floor() as i64;

    debug!("Pixel region: ({}, {}) to ({}, {})",
        min_x_pixel, min_y_pixel, max_x_pixel, max_y_pixel);

    // Create a region, ensuring it's within bounds
    let x = min_x_pixel.max(0).min(img_width as i64 - 1) as u32;
    let y = min_y_pixel.max(0).min(img_height as i64 - 1) as u32;
    let width = ((max_x_pixel - min_x_pixel).max(1) as u32).min(img_width - x);
    let height = ((max_y_pixel - min_y_pixel).max(1) as u32).min(img_height - y);

    Region::new(x, y, width, height)
}

/// Convert WGS84 coordinates to Web Mercator pixels
///
/// Specialized function for the common case of transforming WGS84 (EPSG:4326)
/// to Web Mercator (EPSG:3857).
///
/// # Arguments
/// * `bbox` - The bounding box in WGS84 coordinates
/// * `geotransform` - The geotransform array from the GeoTIFF
/// * `img_width` - Image width in pixels
/// * `img_height` - Image height in pixels
///
/// # Returns
/// A Region for extraction
fn convert_wgs84_to_web_mercator(
    bbox: &BoundingBox,
    geotransform: &[f64],
    img_width: u32,
    img_height: u32
) -> Region {
    info!("Converting WGS84 coordinates to Web Mercator for extraction");

    use std::f64::consts::PI;

    // WGS84 coordinates are typically stored as longitude,latitude
    // but Web Mercator expects longitude/latitude order
    let lon_min = bbox.min_x;
    let lat_min = bbox.min_y;
    let lon_max = bbox.max_x;
    let lat_max = bbox.max_y;

    // Clamp latitude to valid range for Web Mercator (-85.06 to 85.06)
    let lat_min_clamped = lat_min.max(-85.06).min(85.06);
    let lat_max_clamped = lat_max.max(-85.06).min(85.06);

    debug!("WGS84 bbox: lon_min={}, lat_min={}, lon_max={}, lat_max={}",
           lon_min, lat_min, lon_max, lat_max);

    // Convert corners to Web Mercator
    // X coordinate: longitude to meters
    let x_min = lon_min * 20037508.34 / 180.0;
    let x_max = lon_max * 20037508.34 / 180.0;

    // Y coordinate: latitude to meters (with clamped values)
    let y_min = f64::ln(f64::tan((lat_min_clamped + 90.0) * PI / 360.0)) * 20037508.34 / PI;
    let y_max = f64::ln(f64::tan((lat_max_clamped + 90.0) * PI / 360.0)) * 20037508.34 / PI;

    debug!("Web Mercator bbox: x_min={}, y_min={}, x_max={}, y_max={}",
           x_min, y_min, x_max, y_max);

    // Calculate pixel coordinates
    let origin_x = geotransform[0];
    let pixel_width = geotransform[1];
    let origin_y = geotransform[3];
    let pixel_height = geotransform[5]; // Usually negative

    // Convert to pixel coordinates - handle min/max ordering
    let min_x_pixel = ((x_min - origin_x) / pixel_width).floor() as i64;
    let max_y_pixel = ((y_min - origin_y) / pixel_height).floor() as i64;
    let max_x_pixel = ((x_max - origin_x) / pixel_width).ceil() as i64;
    let min_y_pixel = ((y_max - origin_y) / pixel_height).floor() as i64;

    debug!("Raw pixel coordinates: ({}, {}) to ({}, {})",
           min_x_pixel, min_y_pixel, max_x_pixel, max_y_pixel);

    // Check if the region is within image bounds
    let x_in_bounds = min_x_pixel < img_width as i64 && max_x_pixel >= 0;
    let y_in_bounds = min_y_pixel < img_height as i64 && max_y_pixel >= 0;

    // If the region is completely outside the image, provide a fallback
    if !x_in_bounds || !y_in_bounds {
        // Calculate a sensible region size based on the radius if available
        let size = if let Some(radius) = bbox.radius_meters {
            // Convert radius from meters to pixels
            ((radius * 2.0) / pixel_width.abs() as f64) as u32
        } else {
            1000 // Default size
        };

        // Center of the image as fallback
        let center_x = img_width / 2;
        let center_y = img_height / 2;

        debug!("Region outside image bounds, using centered region of size {}", size);

        return Region::new(
            center_x.saturating_sub(size / 2),
            center_y.saturating_sub(size / 2),
            size.min(img_width),
            size.min(img_height)
        );
    }

    // Ensure coordinates are within bounds
    let x = min_x_pixel.max(0).min(img_width as i64 - 1) as u32;
    let y = min_y_pixel.max(0).min(img_height as i64 - 1) as u32;
    let width = ((max_x_pixel - min_x_pixel).max(1) as u32).min(img_width - x);
    let height = ((max_y_pixel - min_y_pixel).max(1) as u32).min(img_height - y);

    debug!("Adjusted pixel region: x={}, y={}, width={}, height={}",
           x, y, width, height);

    // Create and return the region
    Region::new(x, y, width, height)
}

/// Adjust a region to fit within image bounds
///
/// # Arguments
/// * `region` - The original region
/// * `img_width` - Image width in pixels
/// * `img_height` - Image height in pixels
/// * `radius_meters` - Optional radius in meters for fallback sizing
/// * `geotransform` - Geotransform array for converting meters to pixels
///
/// # Returns
/// An adjusted region that fits within the image bounds
fn adjust_region_to_image_bounds(
    region: Region,
    img_width: u32,
    img_height: u32,
    radius_meters: Option<f64>,
    geotransform: &[f64]
) -> Region {
    // If region is completely outside the image, return a reasonable default
    if region.x >= img_width || region.y >= img_height || region.width == 0 || region.height == 0 {
        warn!("Region is completely outside image bounds or has zero size");

        // For better diagnostics
        debug!("Region: x={}, y={}, w={}, h={}, Image: {}x{}",
              region.x, region.y, region.width, region.height, img_width, img_height);

        // Return a region in the center of the image
        let center_x = img_width / 2;
        let center_y = img_height / 2;

        // Calculate size based on radius in meters if provided
        let size = if let Some(radius) = radius_meters {
            // Convert radius from meters to pixels using the geotransform
            let pixel_width = geotransform[1].abs();  // Pixel width in map units (meters)

            // Calculate radius in pixels (each side of the square is 2*radius)
            let size_in_pixels = ((radius * 2.0) / pixel_width as f64).ceil() as u32;
            debug!("Calculated fallback size of {} pixels from radius {} meters (pixel width {} meters)",
                  size_in_pixels, radius, pixel_width);

            // Ensure the size is reasonable
            size_in_pixels.min(5000).max(100) // Min 100px, max 5000px
        } else {
            // Default size of 100 pixels if no radius specified
            100
        };

        // Create a centered region of the calculated size
        let half_size = size / 2;
        return Region::new(
            center_x.saturating_sub(half_size),
            center_y.saturating_sub(half_size),
            size.min(img_width - center_x.saturating_sub(half_size)),
            size.min(img_height - center_y.saturating_sub(half_size))
        );
    }

    // Ensure region doesn't extend beyond image boundaries
    let mut x = region.x;
    let mut y = region.y;
    let mut width = region.width;
    let mut height = region.height;

    if x >= img_width {
        x = img_width - 1;
    }

    if y >= img_height {
        y = img_height - 1;
    }

    if x + width > img_width {
        width = img_width - x;
    }

    if y + height > img_height {
        height = img_height - y;
    }

    // Ensure we have at least a reasonable region size
    if width == 0 { width = 1; }
    if height == 0 { height = 1; }

    Region::new(x, y, width, height)
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
    info!("Determining extraction region");

    // Create a direct conversion region as fallback
    let direct_region = Region::new(
        bbox.min_x as u32,
        bbox.min_y as u32,
        (bbox.max_x - bbox.min_x) as u32,
        (bbox.max_y - bbox.min_y) as u32
    );

    // Save the radius for potential fallback use
    let radius_meters = bbox.radius_meters;

    // Check if bbox has the EPSG code specified
    let source_epsg = if let Some(epsg_code) = bbox.epsg {
        info!("Using source EPSG:{} coordinates", epsg_code);
        epsg_code
    } else {
        info!("No source EPSG code specified, assuming direct pixel coordinates");
        return Ok(direct_region);
    };

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

    // Get image dimensions
    let (img_width, img_height) = match ifd.get_dimensions() {
        Some((w, h)) => (w as u32, h as u32),
        None => {
            warn!("Could not determine image dimensions");
            return Ok(direct_region);
        }
    };

    debug!("Image dimensions from IFD #0: {}x{}", img_width, img_height);

    // Try to calculate geotransform
    match calculate_geotransform(ifd, byte_order_handler, file_path) {
        Ok(geotransform) => {
            info!("Converting geographic coordinates to pixel coordinates");

            // Extract geospatial metadata to determine the coordinate system of the image
            let geo_info = match GeoKeyParser::extract_geo_info(ifd, byte_order_handler, file_path) {
                Ok(info) => {
                    info!("Found projection information: EPSG:{}", info.epsg_code);
                    info
                },
                Err(e) => {
                    warn!("Failed to extract GeoTIFF info: {}, using fallback", e);
                    return Ok(direct_region);
                }
            };

            let target_epsg = geo_info.epsg_code;
            info!("Image CRS is EPSG:{}", target_epsg);

            // Use our more generic coordinate conversion function
            let region = generic_crs_to_pixel_region(
                &bbox,
                &geotransform,
                img_width,
                img_height,
                source_epsg,
                target_epsg,
                radius_meters
            );

            info!("Final extraction region: x={}, y={}, width={}, height={}",
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

/// Apply a circular mask to an image
///
/// Takes an image and applies a circular mask if the shape is "circle",
/// making pixels outside the circle transparent.
///
/// # Arguments
/// * `image` - The image to mask
/// * `shape` - The shape to apply ("circle" or other)
///
/// # Returns
/// A new image with the mask applied (RGBA format)
pub fn apply_shape_mask(image: &DynamicImage, shape: &str) -> DynamicImage {
    // If not a circle, return the original image
    if shape.to_lowercase() != "circle" {
        return image.clone();
    }

    // Create an RGBA image with transparency
    let width = image.width();
    let height = image.height();
    let mut rgba = image::RgbaImage::new(width, height);

    // Define the circle
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = (width.min(height) / 2) as f32;

    // For normal RGB images
    let rgb = image.to_rgb8();

    // Transfer pixels, making those outside the circle transparent
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance_squared = dx*dx + dy*dy;

            if distance_squared <= radius*radius {
                // Inside circle - copy the pixel
                let pixel = rgb.get_pixel(x, y);
                rgba.put_pixel(x, y, image::Rgba([pixel[0], pixel[1], pixel[2], 255]));
            } else {
                // Outside circle - transparent
                rgba.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    DynamicImage::ImageRgba8(rgba)
}

/// Ensure a file path has a PNG extension for transparency support
///
/// # Arguments
/// * `path` - The original file path
///
/// # Returns
/// A String with a .png extension
pub fn ensure_png_extension(path: &str) -> String {
    let path = Path::new(path);
    if let Some(ext) = path.extension() {
        if ext.to_string_lossy().to_lowercase() == "png" {
            return path.to_string_lossy().to_string();
        }
    }

    // Replace or add .png extension
    let stem = path.file_stem().unwrap_or_default();
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    parent.join(format!("{}.png", stem.to_string_lossy())).to_string_lossy().to_string()
}