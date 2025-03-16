//! TIFF extraction utility functions
//!
//! This module provides helper functions for extracting and processing
//! image data from TIFF files, focusing on clean, modular code organization.

use image::DynamicImage;
use log::{debug, info, warn};

use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::IFD;
use crate::tiff::TiffReader;
use crate::tiff::constants::{tags, field_types, photometric};
use crate::tiff::IFDEntry;
use crate::tiff::TiffBuilder;
use crate::extractor::Region;
use crate::tiff::geo_key_parser::GeoKeyParser;

/// Statistics about pixel values in an image
///
/// Contains the minimum and maximum values found in the image,
/// which are used for the MinSampleValue and MaxSampleValue tags.
pub struct ImageValueStats {
    /// Minimum pixel value found in the image
    pub min_value: u64,

    /// Maximum pixel value found in the image
    pub max_value: u64,
}

/// Calculate statistics for a grayscale image
///
/// Analyzes a grayscale image to find the minimum and maximum pixel values,
/// which are needed for proper image interpretation in TIFF files.
///
/// # Arguments
/// * `image` - The image to analyze
///
/// # Returns
/// Statistics containing min and max values
pub fn calculate_grayscale_stats(image: &DynamicImage) -> ImageValueStats {
    let gray_image = image.to_luma8();

    let mut min_value: u8 = 255;
    let mut max_value: u8 = 0;

    for pixel in gray_image.pixels() {
        let value = pixel.0[0];
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    info!("Calculated pixel value range: {} to {}", min_value, max_value);

    ImageValueStats {
        min_value: min_value as u64,
        max_value: max_value as u64,
    }
}

/// Calculate statistics for an RGB image
///
/// Analyzes an RGB image to find the minimum and maximum values across all channels.
/// The overall min/max values are used for TIFF tags.
///
/// # Arguments
/// * `image` - The RGB image to analyze
///
/// # Returns
/// Statistics containing overall min and max values
pub fn calculate_rgb_stats(image: &DynamicImage) -> ImageValueStats {
    let rgb_image = image.to_rgb8();

    let mut min_values = [255u8, 255u8, 255u8];
    let mut max_values = [0u8, 0u8, 0u8];

    for pixel in rgb_image.pixels() {
        for i in 0..3 {
            min_values[i] = min_values[i].min(pixel.0[i]);
            max_values[i] = max_values[i].max(pixel.0[i]);
        }
    }

    info!("Calculated pixel value ranges: R({} to {}), G({} to {}), B({} to {})",
          min_values[0], max_values[0], min_values[1], max_values[1], min_values[2], max_values[2]);

    // Use the min of mins and max of maxes
    let overall_min = *min_values.iter().min().unwrap() as u64;
    let overall_max = *max_values.iter().max().unwrap() as u64;

    ImageValueStats {
        min_value: overall_min,
        max_value: overall_max,
    }
}

/// Process a grayscale image and set up the appropriate TIFF structures
///
/// Handles converting the image to grayscale format if needed, calculates
/// statistics, and sets up all the necessary TIFF tags and data structures.
///
/// # Arguments
/// * `image` - The image to process
/// * `builder` - TIFF builder to configure
/// * `ifd_index` - Index of the IFD to modify
/// * `bits_per_sample` - Bit depth for each pixel
///
/// # Returns
/// Result indicating success or an error
pub fn process_grayscale_image(
    image: &DynamicImage,
    builder: &mut TiffBuilder,
    ifd_index: usize,
    bits_per_sample: u16
) -> TiffResult<()> {
    info!("Processing grayscale image data");

    // Convert to grayscale
    let gray_image = image.to_luma8();

    // Calculate statistics
    let stats = calculate_grayscale_stats(image);

    // Set min/max values
    builder.ifds[ifd_index].add_entry(IFDEntry::new(
        tags::MIN_SAMPLE_VALUE, field_types::SHORT, 1, stats.min_value));
    builder.ifds[ifd_index].add_entry(IFDEntry::new(
        tags::MAX_SAMPLE_VALUE, field_types::SHORT, 1, stats.max_value));

    // Get raw data
    let gray_data = gray_image.into_raw();

    // Add grayscale tags
    builder.add_basic_gray_tags(ifd_index, image.width(), image.height(), bits_per_sample);

    // Setup the single strip
    builder.setup_single_strip(ifd_index, gray_data);

    Ok(())
}

/// Process an RGB image and set up the appropriate TIFF structures
///
/// Handles converting the image to RGB format if needed, calculates
/// statistics, and sets up all the necessary TIFF tags and data structures.
///
/// # Arguments
/// * `image` - The image to process
/// * `builder` - TIFF builder to configure
/// * `ifd_index` - Index of the IFD to modify
///
/// # Returns
/// Result indicating success or an error
pub fn process_rgb_image(
    image: &DynamicImage,
    builder: &mut TiffBuilder,
    ifd_index: usize
) -> TiffResult<()> {
    info!("Processing RGB image data");

    // Convert to RGB
    let rgb_image = image.to_rgb8();

    // Calculate statistics
    let stats = calculate_rgb_stats(image);

    // Set min/max values
    builder.ifds[ifd_index].add_entry(IFDEntry::new(
        tags::MIN_SAMPLE_VALUE, field_types::SHORT, 1, stats.min_value));
    builder.ifds[ifd_index].add_entry(IFDEntry::new(
        tags::MAX_SAMPLE_VALUE, field_types::SHORT, 1, stats.max_value));

    // Get raw data
    let rgb_data = rgb_image.into_raw();

    // Add RGB tags
    builder.add_basic_rgb_tags(ifd_index, image.width(), image.height());

    // Setup the single strip
    builder.setup_single_strip(ifd_index, rgb_data);

    Ok(())
}

/// Extract a NoData value from a TIFF file
///
/// Reads the NoData value from a TIFF file's GDAL_NODATA tag.
/// If the tag is not present or invalid, returns a default value.
///
/// # Arguments
/// * `ifd` - The IFD containing the tag
/// * `reader` - TIFF reader to use for reading tag data
///
/// # Returns
/// The NoData value as a string
pub fn extract_nodata_value(ifd: &IFD, reader: &TiffReader) -> String {
    // Check if GDAL_NODATA tag exists
    let nodata_entry = match ifd.get_entry(tags::GDAL_NODATA) {
        Some(entry) => entry,
        None => {
            info!("No NoData tag found in original file, using 255");
            return "255".to_string();
        }
    };

    // Check if the tag is of ASCII type
    if nodata_entry.field_type != field_types::ASCII {
        warn!("NoData tag has unexpected field type {}, using default 255", nodata_entry.field_type);
        return "255".to_string();
    }

    // Try to read the NoData value
    let nodata_str = match reader.read_ascii_string_at_offset(nodata_entry.value_offset, nodata_entry.count) {
        Ok(str) => str,
        Err(e) => {
            warn!("Failed to read NoData value: {:?}, using default 255", e);
            return "255".to_string();
        }
    };

    // Process the NoData value
    let trimmed = nodata_str.trim_end_matches('\0');
    info!("Found NoData value in original file: '{}'", trimmed);

    if trimmed == ":w" || trimmed.is_empty() {
        "255".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Extract GDAL metadata from a TIFF file
///
/// Reads metadata from the GDAL_METADATA tag if present.
///
/// # Arguments
/// * `ifd` - The IFD containing the tag
/// * `reader` - TIFF reader to use for reading tag data
///
/// # Returns
/// Optional metadata string
pub fn extract_gdal_metadata(ifd: &IFD, reader: &TiffReader) -> Option<String> {
    // Check if GDAL_METADATA tag exists
    let meta_entry = match ifd.get_entry(tags::GDAL_METADATA) {
        Some(entry) => entry,
        None => return None,
    };

    // Check if the tag is of ASCII type
    if meta_entry.field_type != field_types::ASCII {
        return None;
    }

    // Try to read the metadata
    reader.read_ascii_string_at_offset(meta_entry.value_offset, meta_entry.count).ok()
}

/// Determine extraction region from input region and image dimensions
///
/// If a region is provided, validates it against image dimensions.
/// If no region is provided, uses the entire image.
///
/// # Arguments
/// * `region` - Optional region to extract
/// * `ifd` - IFD containing image dimension information
///
/// # Returns
/// The region to extract or an error if invalid
pub fn determine_extraction_region(region: Option<Region>, ifd: &IFD) -> TiffResult<Region> {
    // Get image dimensions
    let dimensions = ifd.get_dimensions()
        .ok_or_else(|| TiffError::GenericError(
            "Missing image dimensions".to_string()))?;

    let (img_width, img_height) = dimensions;
    info!("Image dimensions: {}x{}", img_width, img_height);

    // If no region provided, use full image
    let region = match region {
        Some(region) => region,
        None => return Ok(Region::new(0, 0, img_width as u32, img_height as u32)),
    };

    // Validate region is within image bounds
    if region.end_x() > img_width as u32 || region.end_y() > img_height as u32 {
        return Err(TiffError::GenericError(
            format!("Region ({},{} - {}x{}) exceeds image dimensions ({}x{})",
                    region.x, region.y, region.width, region.height,
                    img_width, img_height)
        ));
    }

    Ok(region)
}

/// Get basic information about a TIFF image
///
/// Extracts basic properties like bits per sample, photometric interpretation,
/// and samples per pixel from an IFD.
///
/// # Arguments
/// * `ifd` - The IFD to analyze
///
/// # Returns
/// The bit depth, photometric interpretation, and samples per pixel
pub fn get_tiff_image_properties(ifd: &IFD) -> (u16, u16, u16) {
    // Get samples per pixel (band count)
    let samples_per_pixel = ifd.get_samples_per_pixel() as u16;  // Convert to u16
    info!("Image has {} samples per pixel", samples_per_pixel);

    // Get bits per sample
    let bits_per_sample = ifd.get_entry(tags::BITS_PER_SAMPLE)
        .map(|e| e.value_offset as u16)
        .unwrap_or(8);
    info!("Image has {} bits per sample", bits_per_sample);

    // Get photometric interpretation
    let photometric = ifd.get_entry(tags::PHOTOMETRIC_INTERPRETATION)
        .map(|e| e.value_offset as u16)
        .unwrap_or(photometric::BLACK_IS_ZERO);
    info!("Image has photometric interpretation: {}", photometric);

    (bits_per_sample, photometric, samples_per_pixel)
}

/// Set up a common set of tags for a new TIFF file
///
/// Sets up basic dimensions and copies tags from the original IFD
/// while excluding tags that will be set separately.
///
/// # Arguments
/// * `builder` - TIFF builder to configure
/// * `ifd_index` - Index of the destination IFD
/// * `original_ifd` - Source IFD to copy tags from
/// * `image` - The image to get dimensions from
///
/// # Returns
/// Result indicating success or an error
pub fn setup_tiff_tags(
    builder: &mut TiffBuilder,
    ifd_index: usize,
    original_ifd: &IFD,
    image: &DynamicImage
) -> TiffResult<()> {
    // Define tags to exclude when copying from original IFD
    let exclude_tags = [
        tags::IMAGE_WIDTH, tags::IMAGE_LENGTH,
        tags::BITS_PER_SAMPLE, tags::COMPRESSION,
        tags::STRIP_OFFSETS, tags::ROWS_PER_STRIP,
        tags::STRIP_BYTE_COUNTS,
        tags::MIN_SAMPLE_VALUE, tags::MAX_SAMPLE_VALUE,
        tags::TILE_WIDTH, tags::TILE_LENGTH, tags::TILE_OFFSETS, tags::TILE_BYTE_COUNTS,
        tags::MODEL_PIXEL_SCALE_TAG, tags::MODEL_TIEPOINT_TAG,
        tags::GEO_KEY_DIRECTORY_TAG, tags::GEO_DOUBLE_PARAMS_TAG, tags::GEO_ASCII_PARAMS_TAG
    ];

    // Copy tags from original IFD, excluding the ones we'll handle separately
    builder.copy_tags_from(ifd_index, original_ifd, &exclude_tags);

    // Add basic image structure tags
    builder.ifds[ifd_index].add_entry(IFDEntry::new(
        tags::IMAGE_WIDTH, field_types::LONG, 1, image.width() as u64));
    builder.ifds[ifd_index].add_entry(IFDEntry::new(
        tags::IMAGE_LENGTH, field_types::LONG, 1, image.height() as u64));

    Ok(())
}

/// Read GeoTIFF information from a TIFF file
///
/// Extracts pixel scale and tiepoint information from a TIFF file.
///
/// # Arguments
/// * `ifd` - The IFD containing GeoTIFF tags
/// * `reader` - TIFF reader to use for reading tag data
/// * `file_path` - Path to the TIFF file
///
/// # Returns
/// Pixel scale and tiepoint values or default values if not found
pub fn read_geotiff_info(
    ifd: &IFD,
    reader: &TiffReader,
    file_path: &str
) -> (Vec<f64>, Vec<f64>) {
    // Get byte order handler
    let byte_order_handler = match reader.get_byte_order_handler() {
        Some(handler) => handler,
        None => {
            warn!("Byte order handler not available, using default geotransform");
            return (vec![1.0, 1.0, 0.0], vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        }
    };

    // Read pixel scale
    let pixel_scale = GeoKeyParser::read_model_pixel_scale_values(
        ifd,
        byte_order_handler,
        file_path
    ).unwrap_or_else(|_| {
        warn!("Failed to read pixel scale, using default values");
        vec![1.0, 1.0, 0.0]
    });

    // Read tiepoint
    let tiepoint = GeoKeyParser::read_model_tiepoint_values(
        ifd,
        byte_order_handler,
        file_path
    ).unwrap_or_else(|_| {
        warn!("Failed to read tiepoint, using default values");
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    });

    info!("Pixel scale: {:?}", pixel_scale);
    info!("Tiepoint: {:?}", tiepoint);

    (pixel_scale, tiepoint)
}

/// Configure photometric interpretation tag
///
/// Ensures the photometric interpretation tag is set correctly,
/// which is important for proper image display.
///
/// # Arguments
/// * `builder` - TIFF builder to configure
/// * `ifd_index` - Index of the IFD to modify
/// * `photometric_value` - Value to set (e.g., BlackIsZero)
pub fn set_photometric_interpretation(
    builder: &mut TiffBuilder,
    ifd_index: usize,
    photometric_value: u16
) {
    let existing_idx = builder.ifds[ifd_index].entries.iter().position(|e|
    e.tag == tags::PHOTOMETRIC_INTERPRETATION);

    match existing_idx {
        Some(idx) => {
            debug!("Updating existing PhotometricInterpretation to {}", photometric_value);
            builder.ifds[ifd_index].entries[idx] = IFDEntry::new(
                tags::PHOTOMETRIC_INTERPRETATION, field_types::SHORT, 1, photometric_value as u64);
        },
        None => {
            debug!("Adding PhotometricInterpretation tag with value {}", photometric_value);
            builder.ifds[ifd_index].add_entry(IFDEntry::new(
                tags::PHOTOMETRIC_INTERPRETATION, field_types::SHORT, 1, photometric_value as u64));
        }
    }
}