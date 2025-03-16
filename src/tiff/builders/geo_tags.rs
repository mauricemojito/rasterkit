//! GeoTIFF tag strategies
//!
//! This module handles the specialized tags that turn a regular TIFF into a GeoTIFF -
//! a georeferenced image that can be accurately placed on a map. These functions
//! handle coordinate systems, transformations, and other geo-spatial metadata.

use crate::tiff::ifd::IFD;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::constants::{tags, field_types};
use crate::extractor::Region;
use log::{debug, info, warn};
use std::io::{Read, Seek, SeekFrom};
use crate::utils::tiff_utils;
use std::collections::HashMap;

/// Handles GeoTIFF tags and transformations
pub struct GeoTagsBuilder;

impl GeoTagsBuilder {
    /// Copy GeoTIFF tags from source IFD to destination IFD
    ///
    /// GeoTIFF files contain special tags that define their geographic properties.
    /// This function preserves those tags when manipulating images, so we don't
    /// lose the spatial reference information.
    pub fn copy_geotiff_tags(
        dest_ifd: &mut IFD,
        external_data: &mut HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        source_ifd: &IFD,
        reader: &mut crate::tiff::TiffReader
    ) -> TiffResult<()> {
        info!("Copying GeoTIFF tags");

        // These are the three main GeoTIFF tags we need to preserve:
        let geotiff_tags = [
            tags::GEO_KEY_DIRECTORY_TAG,  // Contains the GeoTIFF keys structure
            tags::GEO_DOUBLE_PARAMS_TAG,  // Double-precision parameters referenced by the directory
            tags::GEO_ASCII_PARAMS_TAG,   // ASCII parameters referenced by the directory
        ];

        // We need to open the file to read the actual tag data
        // if it's stored externally (which is common for GeoTIFF tags)
        let mut file = match reader.create_reader() {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to create reader for GeoTIFF tag data: {:?}", e);
                return Err(e);
            }
        };

        // Process each GeoTIFF tag
        for &tag in &geotiff_tags {
            // Skip tags that don't exist in the source
            let entry = match source_ifd.get_entry(tag) {
                Some(e) => e,
                None => continue,
            };

            debug!("Copying GeoTIFF tag {} (count: {})", tag, entry.count);

            // Figure out if the data is stored inline or external to the IFD
            // TIFF has a trick where small data can be stored directly in the tag's
            // value field, but larger data needs to be stored elsewhere in the file
            let type_size = tiff_utils::get_field_type_size(entry.field_type);
            let data_size = type_size * entry.count as usize;

            // The size threshold depends on whether this is a BigTIFF or not
            let entry_size = if reader.is_big_tiff() { 8 } else { 4 };

            // For inline values, just copy the entry directly
            // These are simpler because the data is already in the tag
            if data_size <= entry_size || data_size == 0 {
                tiff_utils::update_ifd_tag(dest_ifd, tag, entry.clone());
                continue;
            }

            // For externally stored data, we need to read it from the file
            let mut data = vec![0u8; data_size];

            // Seek to the location in the file where this data is stored
            // and read the actual bytes
            match file.seek(SeekFrom::Start(entry.value_offset))
                .and_then(|_| file.read_exact(&mut data))
            {
                Ok(_) => {
                    // Replace any existing tag with the same ID and store the external data
                    tiff_utils::create_external_tag(
                        dest_ifd,
                        external_data,
                        ifd_index,
                        tag,
                        entry.field_type,
                        entry.count,
                        data
                    );
                },
                Err(e) => {
                    // If we can't read this tag, log it but continue with others
                    warn!("Failed to read data for GeoTIFF tag {}: {:?}", tag, e);
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Adjust GeoTIFF tags for an extracted region
    ///
    /// When we extract a sub-region from a GeoTIFF, we need to update the
    /// geospatial references so the new file still aligns correctly with
    /// the real-world coordinates. This is critical for operations like
    /// cropping or tiling a larger georeferenced image.
    pub fn adjust_geotiff_for_region(
        ifd: &mut IFD,
        external_data: &mut HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        region: &Region,
        pixel_scale: &[f64],
        tiepoint: &[f64]
    ) -> TiffResult<()> {
        info!("Adjusting GeoTIFF tags for region: {:?}", region);

        // We need at least 2 values for pixel scale (x,y) and 6 for tiepoint
        // (raster x,y,z and map x,y,z)
        if pixel_scale.len() < 2 || tiepoint.len() < 6 {
            return Err(TiffError::GenericError(
                "Invalid pixel scale or tiepoint data".to_string()));
        }

        // Extract original map coordinates from tiepoint
        // These tell us where the source image is anchored in the real world
        let orig_map_x = tiepoint[3];
        let orig_map_y = tiepoint[4];

        // Get pixel dimensions in map units (e.g., meters per pixel)
        let pixel_width = pixel_scale[0];
        // We take abs() here because Y scale is typically negative in GeoTIFFs
        // (since Y increases going south in pixel space but north in map space)
        let pixel_height = pixel_scale[1].abs();

        // Now calculate where our extracted region should be placed on the map
        // This is the heart of the transformation - we're offsetting the map
        // coordinates based on the region's position in the original image
        let new_map_x = orig_map_x + (region.x as f64 * pixel_width);
        let new_map_y = orig_map_y - (region.y as f64 * pixel_height);

        // Create new tiepoint data - 6 doubles (8 bytes each)
        // A tiepoint links a pixel position to a map position
        let mut new_tiepoint_data = Vec::with_capacity(6 * 8);

        // For our extracted region, (0,0) in the new image corresponds to
        // the region's origin in the original image
        for _ in 0..3 {
            new_tiepoint_data.extend_from_slice(&0.0f64.to_le_bytes()); // Raster X, Y, Z
        }

        // These are the matching map coordinates for that pixel
        new_tiepoint_data.extend_from_slice(&new_map_x.to_le_bytes()); // Map X
        new_tiepoint_data.extend_from_slice(&new_map_y.to_le_bytes()); // Map Y
        new_tiepoint_data.extend_from_slice(&0.0f64.to_le_bytes());    // Map Z (usually 0)

        // Update the ModelTiepointTag with our new values
        tiff_utils::create_external_tag(
            ifd,
            external_data,
            ifd_index,
            tags::MODEL_TIEPOINT_TAG,
            field_types::DOUBLE,
            6,
            new_tiepoint_data
        );

        // Now handle the pixel scale - this doesn't change for the extracted region
        // but we need to preserve it in the new file
        let mut pixel_scale_data = Vec::with_capacity(3 * 8);

        // The X scale (map units per pixel in X direction)
        pixel_scale_data.extend_from_slice(&pixel_scale[0].to_le_bytes());

        // The Y scale (map units per pixel in Y direction)
        // Keep the original sign (typically negative)
        pixel_scale_data.extend_from_slice(&pixel_scale[1].to_le_bytes());

        // The Z scale if available (usually 0 or 1)
        let z_scale = pixel_scale.get(2).copied().unwrap_or(0.0);
        pixel_scale_data.extend_from_slice(&z_scale.to_le_bytes());

        // Update the ModelPixelScaleTag
        tiff_utils::create_external_tag(
            ifd,
            external_data,
            ifd_index,
            tags::MODEL_PIXEL_SCALE_TAG,
            field_types::DOUBLE,
            3,
            pixel_scale_data
        );

        Ok(())
    }

    /// Copy appearance-related tags from source IFD
    ///
    /// Some tags affect how image data is visually interpreted.
    /// This function preserves those so the output looks like the input.
    pub fn copy_appearance_tags(
        dest_ifd: &mut IFD,
        source_ifd: &IFD
    ) {
        info!("Copying appearance-related tags");

        // These tags influence how the pixels are visually presented
        // They're important for maintaining the correct appearance
        let appearance_tags = [
            tags::MIN_SAMPLE_VALUE,       // Used for contrast stretching
            tags::MAX_SAMPLE_VALUE,       // Used for contrast stretching
            tags::RESOLUTION_UNIT,        // Affects physical print size interpretation
            tags::TRANSFER_FUNCTION,      // For color correction
            tags::SAMPLE_FORMAT,          // How to interpret pixel values (signed/unsigned/float)
            tags::PLANAR_CONFIGURATION,   // How multi-channel data is organized
            tags::COLOR_MAP,              // For indexed color images
            tags::GDAL_NODATA,            // GDAL_NODATA - Custom tag used by GDAL to mark no-data values
        ];

        // Copy each tag if it exists in the source
        tiff_utils::copy_tags(dest_ifd, source_ifd, &appearance_tags);
    }

    /// Copy tags from source IFD, excluding specified ones
    ///
    /// This is a utility function that lets us selectively copy tags,
    /// which is useful when we want to preserve most metadata but
    /// override specific tags in the new file.
    pub fn copy_tags_from(
        dest_ifd: &mut IFD,
        source_ifd: &IFD,
        exclude_tags: &[u16]
    ) {
        info!("Copying tags (excluding {} tags)", exclude_tags.len());

        // Loop through all entries in the source IFD
        tiff_utils::copy_tags_except(dest_ifd, source_ifd, exclude_tags);
    }
}