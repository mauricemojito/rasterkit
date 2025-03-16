//! Metadata tag strategies (Nodata actually fails and should be rewritten)
//!
//! This module handles special metadata tags in TIFF files, with a focus on
//! GDAL-specific extensions. GDAL is a popular geospatial library that adds
//! custom tags to store important information like no-data values and stats.

use crate::tiff::ifd::{IFD, IFDEntry};
use crate::tiff::constants::{tags, field_types};
use log::{debug, info, warn};
use crate::utils::tiff_utils;
use crate::utils::xml_utils;
use std::collections::HashMap;

/// Handles metadata tags in TIFF files
pub struct MetadataBuilder;

impl MetadataBuilder {
    /// Add a GDAL NoData tag to an IFD
    ///
    /// The NoData tag tells GIS software which pixel value should be treated as
    /// "no data" or transparent. This is critical for things like satellite imagery
    /// or elevation data where some areas have no valid measurements.
    pub fn add_nodata_tag(
        ifd: &mut IFD,
        external_data: &mut HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        nodata_value: &str
    ) {
        // Clean up the input value - sometimes these come with extra whitespace
        let trimmed_nodata = nodata_value.trim();

        // GDAL has some quirks with nodata values - handle them gracefully
        let final_nodata = match trimmed_nodata {
            ":w" | "" => {
                warn!("Invalid NoData value '{}', falling back to 255", trimmed_nodata);
                "255"  // Use 255 (typically max value for 8-bit data) as fallback
            },
            _ => trimmed_nodata
        };

        info!("Adding GDAL NoData tag: {}", final_nodata);

        // Add the string exactly as GDAL expects - with null termination
        // This is known to work with most GDAL/TIFF readers
        let mut nodata_bytes = final_nodata.as_bytes().to_vec();
        nodata_bytes.push(0);  // Add NULL terminator - required for ASCII tags in TIFF

        debug!("NoData bytes: {:?}", nodata_bytes);

        // Add the tag - note that count should include the NULL terminator
        tiff_utils::create_external_tag(
            ifd,
            external_data,
            ifd_index,
            tags::GDAL_NODATA,
            field_types::ASCII,
            nodata_bytes.len() as u64,
            nodata_bytes
        );

        // Also add the standard TIFF NODATA tag if possible
        // Some applications look for this instead of the GDAL-specific tag
        if let Ok(value) = final_nodata.parse::<u8>() {
            // Set the standard TIFF tag for NoData if it's a simple numeric value
            // This improves compatibility with non-GDAL software
            ifd.add_entry(IFDEntry::new(
                tags::GDAL_NODATA,
                field_types::BYTE,
                1,
                value as u64
            ));
        }
    }

    /// Add or update GDAL metadata tag
    ///
    /// GDAL stores metadata in a custom XML format inside a TIFF tag.
    /// This function either creates new metadata or updates existing metadata
    /// to include the NoData value. This helps ensure that the NoData setting
    /// is preserved across different GIS applications.
    pub fn add_gdal_metadata_tag(
        ifd: &mut IFD,
        external_data: &mut HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        existing_metadata: Option<&str>,
        nodata_value: &str
    ) {
        info!("Adding/updating GDAL metadata tag");

        // Create the NoData item that we'll be adding or replacing
        let nodata_item = format!("<Item name=\"NODATA_VALUES\">{}</Item>", nodata_value);

        // Generate the appropriate metadata XML
        let metadata = match existing_metadata {
            // No existing metadata - create new XML structure
            None => {
                info!("Creating new metadata with NODATA_VALUES");
                format!("<GDALMetadata>\n  {}\n</GDALMetadata>", nodata_item)
            },
            // Update existing metadata
            Some(existing) => {
                if existing.contains("<Item name=\"NODATA_VALUES\"") {
                    info!("Updating existing NODATA_VALUES in metadata");
                    xml_utils::replace_xml_tag(existing, "NODATA_VALUES", nodata_value)
                } else {
                    info!("Adding NODATA_VALUES to existing metadata");
                    xml_utils::add_to_gdal_metadata(existing, &nodata_item)
                }
            }
        };

        // Add the new or updated metadata as an ASCII tag
        let metadata_bytes = metadata.as_bytes().to_vec();
        tiff_utils::create_external_tag(
            ifd,
            external_data,
            ifd_index,
            tags::GDAL_METADATA,
            field_types::ASCII,
            metadata_bytes.len() as u64,
            metadata_bytes
        );
    }

    /// Copy statistics tags from source IFD
    ///
    /// This preserves GDAL-specific metadata between files, which includes
    /// important statistical information about the raster data. This is useful
    /// for operations that don't change the data values but repackage them,
    /// like format conversions or spatial subsetting.
    pub fn copy_statistics_tags(
        dest_ifd: &mut IFD,
        source_ifd: &IFD
    ) {
        info!("Copying statistics tags");

        // The two main GDAL-specific tags we want to preserve
        let stats_tags = [
            tags::GDAL_METADATA, // Contains statistics and other metadata in XML format
            tags::GDAL_NODATA,   // Indicates which value should be treated as "no data"
        ];

        // Copy each tag if it exists
        tiff_utils::copy_tags(dest_ifd, source_ifd, &stats_tags);
    }
}