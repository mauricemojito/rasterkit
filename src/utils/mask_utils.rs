//! Image masking utilities
//!
//! This module provides functions for applying masks to images based on
//! different shapes, like circles and squares.

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use log::info;
use std::path::Path;
use crate::tiff::errors::{TiffError, TiffResult};

/// Apply a shape mask to an image
///
/// Applies a mask based on the specified shape, making pixels outside
/// the shape transparent.
///
/// # Arguments
/// * `image` - The input image
/// * `shape` - The shape to use ("circle" or "square")
///
/// # Returns
/// A new RGBA image with the mask applied
pub fn apply_shape_mask(image: &DynamicImage, shape: &str) -> DynamicImage {
    // For square (default), no masking needed
    if shape.to_lowercase() != "circle" {
        return image.clone();
    }

    // Create the output RGBA image
    let width = image.width();
    let height = image.height();
    let mut rgba = RgbaImage::new(width, height);

    // Calculate circle parameters
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = (width.min(height) / 2) as f32;

    // Get source pixels (convert to RGB if needed)
    let rgb = image.to_rgb8();

    // Apply the mask pixel by pixel
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance_squared = dx * dx + dy * dy;

            if distance_squared <= radius * radius {
                // Inside the circle - copy with full opacity
                let pixel = rgb.get_pixel(x, y);
                rgba.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], 255]));
            } else {
                // Outside the circle - transparent
                rgba.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    DynamicImage::ImageRgba8(rgba)
}

/// Ensure a file path has PNG extension for transparency support
///
/// If the file doesn't already have a PNG extension, this function
/// creates a new path with the .png extension.
///
/// # Arguments
/// * `file_path` - The original file path
///
/// # Returns
/// A path with .png extension
pub fn ensure_png_extension(file_path: &str) -> String {
    let path = Path::new(file_path);

    // If it's already a PNG, return as is
    if let Some(ext) = path.extension() {
        if ext.to_string_lossy().to_lowercase() == "png" {
            return file_path.to_string();
        }
    }

    // Create a new path with .png extension
    let stem = path.file_stem().unwrap_or_default();
    let parent = path.parent().unwrap_or_else(|| Path::new(""));

    let new_path = parent.join(format!("{}.png", stem.to_string_lossy()));
    new_path.to_string_lossy().to_string()
}

/// Save an image with appropriate format for the shape
///
/// # Arguments
/// * `image` - The image to save
/// * `output_path` - Path where to save the output
/// * `shape` - The shape that was used ("circle" or "square")
///
/// # Returns
/// Result indicating success or an error
pub fn save_shaped_image(image: &DynamicImage, output_path: &str, shape: &str) -> TiffResult<()> {
    // For circles, we need PNG to support transparency
    let final_path = if shape.to_lowercase() == "circle" {
        let png_path = ensure_png_extension(output_path);
        if png_path != output_path {
            info!("Changed output extension to PNG for transparency support: {}", png_path);
        }
        png_path
    } else {
        output_path.to_string()
    };

    // Save the image
    match image.save(&final_path) {
        Ok(_) => Ok(()),
        Err(e) => Err(TiffError::GenericError(format!("Failed to save image: {}", e)))
    }
}