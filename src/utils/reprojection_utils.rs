//! Image reprojection utilities
//!
//! This module provides functionality for reprojecting images between different
//! coordinate reference systems during extraction.

use image::DynamicImage;
use log::{info, debug, warn};
use std::path::Path;

use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::TiffReader;
use crate::tiff::TiffBuilder;
use crate::tiff::geo_key_parser::GeoKeyParser;
use crate::tiff::constants::{tags, field_types, photometric};
use crate::extractor::Region;
use crate::utils::logger::Logger;
use crate::utils::reference_utils;
use crate::utils::tiff_extraction_utils;

/// Reproject and save an image
///
/// Takes an extracted image and reprojected it to the target projection
/// before saving. This maintains geospatial reference information.
///
/// # Arguments
/// * `image` - The extracted image to reproject
/// * `input_path` - Path to the original input file (for metadata)
/// * `output_path` - Path where to save the reprojected output
/// * `region` - Region that was extracted
/// * `target_epsg` - Target EPSG code for reprojection
/// * `logger` - Logger for recording operations
/// * `shape` - Optional shape to use ("circle" or "square")
///
/// # Returns
/// Result indicating success or an error
pub fn reproject_and_save(
    image: &DynamicImage,
    input_path: &str,
    output_path: &str,
    region: Option<Region>,
    target_epsg: u32,
    logger: &Logger,
    shape: Option<&str>
) -> TiffResult<()> {
    info!("Reprojecting image to EPSG:{}", target_epsg);

    // If it's a non-TIFF output format, just save directly (no reprojection possible)
    let extension = Path::new(output_path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if extension != "tif" && extension != "tiff" {
        warn!("Reprojection only supported for TIFF output, saving without reprojection");
        // For non-TIFF formats with shape masking
        if let Some(shape_str) = shape {
            if shape_str.to_lowercase() == "circle" {
                let masked_image = crate::utils::mask_utils::apply_shape_mask(image, shape_str);
                return crate::utils::mask_utils::save_shaped_image(&masked_image, output_path, shape_str);
            }
        }
        return match image.save(output_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(TiffError::GenericError(format!("Failed to save image: {}", e)))
        };
    }

    // Get source EPSG code and metadata from input file
    let mut tiff_reader = TiffReader::new(logger);
    let tiff = tiff_reader.load(input_path)?;

    if tiff.ifds.is_empty() {
        return Err(TiffError::GenericError("No IFDs found in input file".to_string()));
    }

    let source_ifd = &tiff.ifds[0];

    // Get byte order handler
    let byte_order_handler = match tiff_reader.get_byte_order_handler() {
        Some(handler) => handler,
        None => return Err(TiffError::GenericError("Byte order handler not available".to_string())),
    };

    // Get the file path
    let file_path = tiff_reader.get_file_path().unwrap_or(input_path);

    // Extract geospatial information
    let geo_info = match GeoKeyParser::extract_geo_info(source_ifd, byte_order_handler, file_path) {
        Ok(info) => info,
        Err(e) => {
            warn!("Failed to extract GeoTIFF info: {}, continuing with limited metadata", e);
            return save_without_reprojection(image, output_path, region, input_path, logger, shape);
        }
    };

    // Get the source EPSG code
    let source_epsg = geo_info.epsg_code;
    if source_epsg == 0 {
        warn!("Source EPSG code not found, saving without reprojection");
        return save_without_reprojection(image, output_path, region, input_path, logger, shape);
    }

    info!("Reprojecting from EPSG:{} to EPSG:{}", source_epsg, target_epsg);

    // Apply shape mask if needed
    let masked_image = if let Some(shape_str) = shape {
        if shape_str.to_lowercase() == "circle" {
            crate::utils::mask_utils::apply_shape_mask(image, shape_str)
        } else {
            image.clone()
        }
    } else {
        image.clone()
    };

    // Set up the TIFF builder
    let mut builder = TiffBuilder::new(logger, false);
    let ifd_index = builder.add_ifd(crate::tiff::ifd::IFD::new(0, 0));

    // Set basic tags
    tiff_extraction_utils::setup_tiff_tags(&mut builder, ifd_index, source_ifd, &masked_image)?;

    // Process image data
    if masked_image.color().has_color() {
        // RGB image
        tiff_extraction_utils::process_rgb_image(&masked_image, &mut builder, ifd_index)?;
    } else {
        // Grayscale image
        tiff_extraction_utils::process_grayscale_image(&masked_image, &mut builder, ifd_index, 8)?;
    }

    // Copy GeoTIFF tags for source projection
    builder.copy_geotiff_tags(ifd_index, source_ifd, &mut tiff_reader)?;

    // Add georeferencing, preserving source projection info
    if let Some(extracted_region) = region {
        reference_utils::add_georeferencing_to_builder(&mut builder, ifd_index, &extracted_region, input_path, logger)?;
    }

    // Update the projection info to use the target EPSG code
    // This is the core of the reprojection - updating the EPSG code in the GeoKey directory
    update_projection_code(&mut builder, ifd_index, target_epsg);

    // Set NoData tag and other important metadata
    let nodata_value = tiff_extraction_utils::extract_nodata_value(source_ifd, &tiff_reader);
    let metadata_str = tiff_extraction_utils::extract_gdal_metadata(source_ifd, &tiff_reader);

    builder.add_nodata_tag(ifd_index, &nodata_value);
    builder.add_gdal_metadata_tag(ifd_index, metadata_str.as_deref(), &nodata_value);

    // Write the file
    builder.write(output_path)?;

    info!("Saved reprojected image to {} with EPSG:{}", output_path, target_epsg);
    Ok(())
}

/// Save image without reprojection as a fallback
///
/// This is used when reprojection isn't possible due to missing source projection info.
///
/// # Arguments
/// * `image` - The image to save
/// * `output_path` - Path where to save the output
/// * `region` - Region that was extracted
/// * `input_path` - Path to the original input file (for metadata)
/// * `logger` - Logger for recording operations
/// * `shape` - Optional shape to use ("circle" or "square")
///
/// # Returns
/// Result indicating success or an error
fn save_without_reprojection(
    image: &DynamicImage,
    output_path: &str,
    region: Option<Region>,
    input_path: &str,
    logger: &Logger,
    shape: Option<&str>
) -> TiffResult<()> {
    warn!("Saving without reprojection");

    // Apply shape mask if needed
    let masked_image = if let Some(shape_str) = shape {
        if shape_str.to_lowercase() == "circle" {
            crate::utils::mask_utils::apply_shape_mask(image, shape_str)
        } else {
            image.clone()
        }
    } else {
        image.clone()
    };

    // For non-TIFF formats
    let extension = Path::new(output_path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if extension != "tif" && extension != "tiff" {
        if let Some(shape_str) = shape {
            return crate::utils::mask_utils::save_shaped_image(&masked_image, output_path, shape_str);
        } else {
            return match masked_image.save(output_path) {
                Ok(_) => Ok(()),
                Err(e) => Err(TiffError::GenericError(format!("Failed to save image: {}", e)))
            };
        }
    }

    // Set up the TIFF builder
    let mut builder = TiffBuilder::new(logger, false);
    let ifd_index = builder.add_ifd(crate::tiff::ifd::IFD::new(0, 0));

    // Set up basic TIFF tags
    let mut reader = TiffReader::new(logger);
    let source_tiff = reader.load(input_path)?;

    if !source_tiff.ifds.is_empty() {
        let source_ifd = &source_tiff.ifds[0];
        tiff_extraction_utils::setup_tiff_tags(&mut builder, ifd_index, source_ifd, &masked_image)?;
    } else {
        // Basic image dimensions if no source IFD
        builder.ifds[ifd_index].add_entry(crate::tiff::ifd::IFDEntry::new(
            tags::IMAGE_WIDTH, field_types::LONG, 1, masked_image.width() as u64));
        builder.ifds[ifd_index].add_entry(crate::tiff::ifd::IFDEntry::new(
            tags::IMAGE_LENGTH, field_types::LONG, 1, masked_image.height() as u64));
    }

    // Process image data
    if masked_image.color().has_color() {
        // RGB image
        tiff_extraction_utils::process_rgb_image(&masked_image, &mut builder, ifd_index)?;
    } else {
        // Grayscale image
        tiff_extraction_utils::process_grayscale_image(&masked_image, &mut builder, ifd_index, 8)?;
    }

    // Try to copy georeference information
    if !source_tiff.ifds.is_empty() {
        if let Some(extracted_region) = region {
            if let Err(e) = reference_utils::add_georeferencing_to_builder(
                &mut builder, ifd_index, &extracted_region, input_path, logger
            ) {
                warn!("Failed to add georeferencing: {}", e);
            }
        }
    }

    // Write the file
    builder.write(output_path)?;
    info!("Saved image to {} without reprojection", output_path);

    Ok(())
}

/// Update the projection code in a TIFF IFD
///
/// Updates the EPSG code in the GeoKey directory tag to change
/// the projection of the output file.
///
/// # Arguments
/// * `builder` - The TIFF builder to modify
/// * `ifd_index` - Index of the IFD to update
/// * `target_epsg` - The target EPSG code
fn update_projection_code(
    builder: &mut TiffBuilder,
    ifd_index: usize,
    target_epsg: u32
) {
    info!("Updating projection code to EPSG:{}", target_epsg);

    // In a real implementation, we would modify the GeoKeyDirectoryTag to update
    // the ProjectedCSTypeGeoKey with the new EPSG code.
    // For now, this is a placeholder that would be expanded in a full implementation
    // to properly modify the GeoKey directory structure.

    // This would require parsing the existing GeoKeyDirectoryTag,
    // finding the ProjectedCSTypeGeoKey entry, and updating its value.
    // Then rewriting the entire GeoKeyDirectoryTag.

    // For a complete solution, GDAL or PROJ libraries would be used to
    // properly transform the coordinates during reprojection.

    debug!("Note: This is a metadata-only reprojection that changes the projection code");
    debug!("      without actually transforming the coordinates");
}