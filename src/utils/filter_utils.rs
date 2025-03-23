//! Value filtering utilities
//!
//! This module provides functions for filtering image data based on value ranges.
//! It allows users to extract only specific ranges of pixel values, making
//! other values transparent or setting them to a background value.

use image::{DynamicImage, GrayImage, Luma, RgbaImage, Rgba};
use log::{debug, info};

/// Filter grayscale values to show only pixels within a specific range
///
/// This function takes a grayscale image and replaces values outside
/// the specified range with a background value (default 0). The resulting
/// image is still grayscale but contains only the values of interest.
///
/// # Arguments
/// * `image` - The grayscale image to filter
/// * `min_value` - The minimum value to keep (inclusive)
/// * `max_value` - The maximum value to keep (inclusive)
/// * `background` - The value to use for pixels outside the range (default: 0)
///
/// # Returns
/// A new grayscale image with filtered values
pub fn filter_grayscale_values(
    image: &GrayImage,
    min_value: u8,
    max_value: u8,
    background: u8
) -> GrayImage {
    info!("Filtering grayscale values: min={}, max={}, background={}",
          min_value, max_value, background);

    // Create a new grayscale image with the same dimensions
    let width = image.width();
    let height = image.height();
    let mut filtered = GrayImage::new(width, height);

    // Process each pixel
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            let value = pixel[0];

            // Keep values within range, replace others with background
            if value >= min_value && value <= max_value {
                filtered.put_pixel(x, y, *pixel);
            } else {
                filtered.put_pixel(x, y, Luma([background]));
            }
        }
    }

    filtered
}

/// Filter a DynamicImage based on grayscale values
///
/// Converts the image to grayscale if needed, applies the filter,
/// and returns a new DynamicImage.
///
/// # Arguments
/// * `image` - The image to filter
/// * `min_value` - The minimum value to keep (inclusive)
/// * `max_value` - The maximum value to keep (inclusive)
/// * `background` - The value to use for pixels outside the range (default: 0)
/// * `transparency` - Whether to make filtered pixels transparent instead of using background value
///
/// # Returns
/// A filtered image
pub fn filter_image_values(
    image: &DynamicImage,
    min_value: u8,
    max_value: u8,
    background: u8,
    transparency: bool
) -> DynamicImage {
    info!("Filtering image values: min={}, max={}, background={}, transparency={}",
          min_value, max_value, background, transparency);

    // Convert to grayscale for filtering
    let gray_image = image.to_luma8();
    let width = gray_image.width();
    let height = gray_image.height();

    if transparency {
        // When using transparency, create an RGBA image
        let mut rgba = RgbaImage::new(width, height);

        // Process each pixel
        for y in 0..height {
            for x in 0..width {
                let value = gray_image.get_pixel(x, y)[0];

                if value >= min_value && value <= max_value {
                    // Keep original pixel but make it fully opaque
                    // We use the grayscale value for R, G, B channels
                    rgba.put_pixel(x, y, Rgba([value, value, value, 255]));
                } else {
                    // Make pixel transparent
                    rgba.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }

        DynamicImage::ImageRgba8(rgba)
    } else {
        // Basic grayscale filtering without transparency
        let filtered = filter_grayscale_values(&gray_image, min_value, max_value, background);
        DynamicImage::ImageLuma8(filtered)
    }
}

/// Parse a filter range string in the format "min,max"
///
/// # Arguments
/// * `filter_str` - String in the format "min,max" (e.g., "15,160")
///
/// # Returns
/// A tuple of (min_value, max_value) or an error if parsing fails
pub fn parse_filter_range(filter_str: &str) -> Result<(u8, u8), String> {
    // Split the string at the comma
    let parts: Vec<&str> = filter_str.split(',').collect();

    if parts.len() != 2 {
        return Err(format!("Invalid filter range format '{}'. Expected 'min,max'", filter_str));
    }

    // Parse min value
    let min_value = match parts[0].trim().parse::<u8>() {
        Ok(value) => value,
        Err(_) => return Err(format!("Invalid minimum value '{}'. Expected a number between 0-255", parts[0]))
    };

    // Parse max value
    let max_value = match parts[1].trim().parse::<u8>() {
        Ok(value) => value,
        Err(_) => return Err(format!("Invalid maximum value '{}'. Expected a number between 0-255", parts[1]))
    };

    // Validate range
    if min_value > max_value {
        return Err(format!("Invalid range: min ({}) is greater than max ({})", min_value, max_value));
    }

    Ok((min_value, max_value))
}