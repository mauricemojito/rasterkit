//! Colormap utilities
//!
//! Utilities for working with colormaps in TIFF files, including
//! color interpolation, mapping pixel values to colors, and colormap
//! application to images.

use log::{info, warn, debug};
use std::path::Path;

use crate::tiff::errors::{TiffResult, TiffError};
use crate::tiff::colormap::{ColorMap, ColorMapReader, RgbColor, ColorMapEntry};
use crate::utils::logger::Logger;
use crate::extractor::Region;
use crate::tiff::TiffReader;
use crate::tiff::geo_key_parser::GeoKeyParser;
use crate::utils::reference_utils::add_georeferencing_to_builder;

/// Find the appropriate color for a pixel value using a colormap
///
/// # Arguments
/// * `colormap` - The colormap to use
/// * `value` - The pixel value to map
///
/// # Returns
/// The RGB color for this value
pub fn find_color_for_value(colormap: &ColorMap, value: u16) -> RgbColor {
    if colormap.entries.is_empty() {
        // Default to black if no entries
        return RgbColor::new(0, 0, 0);
    }

    // Check for exact match first
    for entry in &colormap.entries {
        if entry.value == value {
            return entry.color;
        }
    }

    // Handle ramp type colormap with interpolation
    if colormap.map_type == "ramp" && colormap.entries.len() > 1 {
        return interpolate_color(colormap, value);
    }

    // For non-ramp colormaps, find the nearest entry
    find_nearest_color(colormap, value)
}

/// Interpolate color for a value using a ramp colormap
///
/// # Arguments
/// * `colormap` - The colormap to use
/// * `value` - The pixel value to interpolate
///
/// # Returns
/// The interpolated RGB color
pub fn interpolate_color(colormap: &ColorMap, value: u16) -> RgbColor {
    // Find the bracketing entries
    let (lower_entry, upper_entry) = find_bracketing_entries(colormap, value);

    // Handle edge cases
    if value <= lower_entry.value {
        return lower_entry.color;
    }

    if value >= upper_entry.value {
        return upper_entry.color;
    }

    // Interpolate between colors
    let range = upper_entry.value as f32 - lower_entry.value as f32;
    let t = (value as f32 - lower_entry.value as f32) / range;

    // Linear interpolation between colors
    let r = (lower_entry.color.r as f32 * (1.0 - t) + upper_entry.color.r as f32 * t) as u8;
    let g = (lower_entry.color.g as f32 * (1.0 - t) + upper_entry.color.g as f32 * t) as u8;
    let b = (lower_entry.color.b as f32 * (1.0 - t) + upper_entry.color.b as f32 * t) as u8;

    RgbColor::new(r, g, b)
}

/// Find the entries that bracket a value in the colormap
///
/// # Arguments
/// * `colormap` - The colormap to search in
/// * `value` - The value to find bracketing entries for
///
/// # Returns
/// A tuple of (lower_entry, upper_entry) that bracket the value
pub fn find_bracketing_entries<'a>(colormap: &'a ColorMap, value: u16) -> (&'a ColorMapEntry, &'a ColorMapEntry) {
    let mut lower_entry = &colormap.entries[0];
    let mut upper_entry = &colormap.entries[colormap.entries.len()-1];

    // Find the entries that bracket this value
    for i in 0..colormap.entries.len()-1 {
        if colormap.entries[i].value <= value && colormap.entries[i+1].value > value {
            lower_entry = &colormap.entries[i];
            upper_entry = &colormap.entries[i+1];
            break;
        }
    }

    (lower_entry, upper_entry)
}

/// Find the nearest color in the colormap
///
/// # Arguments
/// * `colormap` - The colormap to search in
/// * `value` - The value to find the nearest color for
///
/// # Returns
/// The nearest RGB color
pub fn find_nearest_color(colormap: &ColorMap, value: u16) -> RgbColor {
    let mut nearest_entry = &colormap.entries[0];
    let mut min_distance = u16::MAX;

    for entry in &colormap.entries {
        let distance = if entry.value > value {
            entry.value - value
        } else {
            value - entry.value
        };

        if distance < min_distance {
            min_distance = distance;
            nearest_entry = entry;
        }
    }

    nearest_entry.color
}

/// Apply colormap to transform grayscale image to RGB
///
/// # Arguments
/// * `grayscale` - The grayscale image to colorize
/// * `colormap` - The colormap to apply
///
/// # Returns
/// A new RGB image with the colormap applied
pub fn apply_colormap_to_image(
    grayscale: &image::GrayImage,
    colormap: &ColorMap
) -> image::RgbImage {
    let width = grayscale.width();
    let height = grayscale.height();
    let mut rgb_image = image::RgbImage::new(width, height);

    // Apply the colormap to each pixel
    for y in 0..height {
        for x in 0..width {
            let pixel = grayscale.get_pixel(x, y);
            let value = pixel[0] as u16; // Value is in the first channel

            // Find the right color for this value
            let color = find_color_for_value(colormap, value);

            // Set the pixel in the output image
            rgb_image.put_pixel(x, y, image::Rgb([color.r, color.g, color.b]));
        }
    }

    rgb_image
}

/// Extract colormap from TIFF file and save to output
///
/// # Arguments
/// * `tiff_path` - Path to the TIFF file
/// * `output_path` - Path where to save the colormap
/// * `logger` - Logger for recording operations
///
/// # Returns
/// Result indicating success or an error
pub fn extract_colormap(tiff_path: &str, output_path: &str, logger: &Logger) -> TiffResult<()> {
    info!("Extracting colormap from {} to {}", tiff_path, output_path);

    let colormap_reader = ColorMapReader::new(logger);
    let colormap = colormap_reader.read_from_tiff(tiff_path)?;

    // Determine output format and layer name
    let extension = Path::new(output_path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let layer_name = Path::new(tiff_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "layer".to_string());

    // Handle different output formats
    if extension != "sld" {
        warn!("Unknown colormap format '{}', defaulting to SLD", extension);
    }

    // Save as SLD (default format)
    colormap.to_sld_file(output_path, &layer_name)?;

    info!("Colormap extracted and saved to {}", output_path);
    colormap.print();

    Ok(())
}

/// Save colorized image as a TIFF file with preserved georeferencing
///
/// # Arguments
/// * `rgb_image` - The RGB image to save
/// * `output_path` - Path where to save the output
/// * `input_path` - Path to the input file (for georeference info)
/// * `region` - Optional region that was extracted
/// * `logger` - Logger for recording operations
///
/// # Returns
/// Result indicating success or an error
pub fn save_colorized_tiff(
    rgb_image: image::RgbImage,
    output_path: &str,
    input_path: &str,
    region: Option<Region>,
    logger: &Logger
) -> TiffResult<()> {
    let width = rgb_image.width();
    let height = rgb_image.height();

    // Create a new TIFF builder for an RGB image
    let mut builder = crate::tiff::TiffBuilder::new(logger, false);

    // Add a new IFD
    let ifd_index = builder.add_ifd(crate::tiff::ifd::IFD::new(0, 0));

    // Set basic RGB tags
    builder.add_basic_rgb_tags(ifd_index, width, height);

    // Convert RGB image to raw data (R,G,B interleaved)
    let rgb_data = rgb_image.into_raw();

    // Set up the strip data
    builder.setup_single_strip(ifd_index, rgb_data);

    // If we have a region, add geotransform for it
    if let Some(extract_region) = region {
        add_georeferencing_to_builder(&mut builder, ifd_index, &extract_region, input_path, logger)?;
    }

    // Write the file
    info!("Writing RGB TIFF with applied colormap to {}", output_path);
    builder.write(output_path)?;

    Ok(())
}

/// Load a colormap from a file
///
/// # Arguments
/// * `colormap_path` - Path to the colormap file
/// * `logger` - Logger for recording operations
///
/// # Returns
/// Result containing the ColorMap or an error
pub fn load_colormap(colormap_path: &str, logger: &Logger) -> TiffResult<ColorMap> {
    let colormap_reader = ColorMapReader::new(logger);
    colormap_reader.read_file(colormap_path)
}