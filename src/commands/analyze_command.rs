//! TIFF/GeoTIFF structure analysis command
//!
//! This module implements the command for analyzing and displaying
//! the structure of TIFF and GeoTIFF files.

use clap::ArgMatches;
use log::{debug, info};

use crate::commands::command_traits::Command;
use crate::tiff::TiffReader;
use crate::tiff::errors::{TiffResult, TiffError};
use crate::utils::logger::Logger;
use crate::tiff::{is_geotiff_tag, get_tag_name, get_projected_cs_description};
use crate::tiff::geo_key_parser::GeoKeyParser;
use crate::utils::tiff_code_translators::compression_code_to_name;
use crate::compression::CompressionFactory;
use crate::tiff::ifd::IFD;
use crate::tiff::constants::{tags, geo_keys};
use crate::tiff::types::TIFF;

/// Command for analyzing TIFF file structure
pub struct AnalyzeCommand<'a> {
    /// Path to the input file
    input_file: String,
    /// Whether to enable verbose output
    verbose: bool,
    /// Logger for recording operations
    logger: &'a Logger,
}

impl<'a> AnalyzeCommand<'a> {
    /// Create a new analyze command
    ///
    /// # Arguments
    /// * `args` - CLI argument matches from clap
    /// * `logger` - Logger for recording operations
    ///
    /// # Returns
    /// A new AnalyzeCommand instance or an error
    pub fn new(args: &ArgMatches, logger: &'a Logger) -> TiffResult<Self> {
        let input_file = args.get_one::<String>("input")
            .ok_or_else(|| TiffError::GenericError("Missing input file".to_string()))?
            .clone();

        let verbose = args.get_flag("verbose");

        Ok(AnalyzeCommand {
            input_file,
            verbose,
            logger,
        })
    }

    /// Display basic TIFF information
    ///
    /// Shows the TIFF format (standard or BigTIFF) and number of IFDs.
    ///
    /// # Arguments
    /// * `tiff` - The TIFF structure to analyze
    fn display_tiff_summary(&self, tiff: &TIFF) {
        info!("TIFF Analysis Results:");
        info!("  Format: {}", if tiff.is_big_tiff { "BigTIFF" } else { "TIFF" });
        info!("  Number of IFDs: {}", tiff.ifd_count());
    }

    /// Display basic IFD information
    ///
    /// Shows IFD offset, number of entries, dimensions, and samples per pixel.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to analyze
    /// * `index` - Index of the IFD in the TIFF file
    fn display_ifd_summary(&self, ifd: &IFD, index: usize) {
        info!("\nIFD #{} (offset: {})", index, ifd.offset);
        info!("  Number of entries: {}", ifd.entries.len());

        if let Some((width, height)) = ifd.get_dimensions() {
            info!("  Dimensions: {}x{}", width, height);
        } else {
            info!("  Dimensions: Not available");
        }

        info!("  Samples per pixel: {}", ifd.get_samples_per_pixel());
    }

    /// Display compression information
    ///
    /// Shows the compression method used and whether it's supported for extraction.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to analyze for compression info
    fn display_compression_info(&self, ifd: &IFD) {
        if let Some(entry) = ifd.get_entry(tags::COMPRESSION) {
            info!("  Compression: {} ({})",
                  entry.value_offset,
                  compression_code_to_name(entry.value_offset));

            // Show if the compression method is supported
            match CompressionFactory::create_handler(entry.value_offset) {
                Ok(_) => info!("    (Compression supported for extraction)"),
                Err(_) => info!("    (Compression not supported for extraction)"),
            }
        }
    }

    /// Display subfile type information
    ///
    /// Shows the NewSubfileType tag value and interprets any relevant flags.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to analyze for subfile type info
    fn display_subfile_type(&self, ifd: &IFD) {
        if let Some(entry) = ifd.get_entry(tags::NEW_SUBFILE_TYPE) {
            info!("  NewSubfileType: {}", entry.value_offset);
            if entry.value_offset & 1 == 1 {
                info!("    (Reduced resolution version)");
            }
        }
    }

    /// Check if IFD has GeoTIFF tags and display them
    ///
    /// Lists all GeoTIFF tags found in the IFD.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to analyze for GeoTIFF tags
    ///
    /// # Returns
    /// Boolean indicating whether GeoTIFF tags were found
    fn display_geotiff_tags(&self, ifd: &IFD) -> bool {
        let has_geotiff = ifd.entries.iter().any(|entry| is_geotiff_tag(entry.tag));

        if has_geotiff {
            info!("  GeoTIFF tags found:");
            for entry in &ifd.entries {
                if is_geotiff_tag(entry.tag) {
                    info!("    Tag {} ({}): count={}, value/offset={}",
                          entry.tag, get_tag_name(entry.tag), entry.count, entry.value_offset);
                }
            }
        }

        has_geotiff
    }

    /// Display detailed GeoTIFF information using the GeoKeyParser
    ///
    /// Extracts and displays detailed GeoTIFF metadata including
    /// pixel scale, tiepoints, GeoKey directory, and PROJ.4 string.
    ///
    /// # Arguments
    /// * `reader` - TIFF reader for accessing tag data
    /// * `ifd` - The IFD containing GeoTIFF information
    fn display_geotiff_details(&self, reader: &TiffReader, ifd: &IFD) {
        if let Some(byte_order_handler) = reader.get_byte_order_handler() {
            let file_path = reader.get_file_path().unwrap_or(&self.input_file);

            // We need to pass the Box<dyn ByteOrderHandler> directly
            self.display_pixel_scale(ifd, byte_order_handler, file_path);
            self.display_tiepoint(ifd, byte_order_handler, file_path);
            self.display_geokey_directory(ifd, byte_order_handler, file_path);
            self.display_proj_string(ifd, byte_order_handler, file_path);
        }
    }

    /// Display pixel scale information
    ///
    /// Shows the pixel dimensions in map units from ModelPixelScale tag.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing GeoTIFF information
    /// * `byte_order_handler` - Handler for interpreting byte order
    /// * `file_path` - Path to the TIFF file
    fn display_pixel_scale(&self, ifd: &IFD,
                           byte_order_handler: &Box<dyn crate::io::byte_order::ByteOrderHandler>,
                           file_path: &str) {
        if let Ok(pixel_scale) = GeoKeyParser::read_model_pixel_scale_values(ifd, byte_order_handler, file_path) {
            if pixel_scale.len() >= 3 {
                info!("  Pixel Size: X={:.6} Y={:.6} meters (Z={:.6})",
                      pixel_scale[0], pixel_scale[1], pixel_scale[2]);
            }
        }
    }

    /// Display tiepoint information
    ///
    /// Shows the relationship between raster and map coordinates
    /// from ModelTiepoint tag.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing GeoTIFF information
    /// * `byte_order_handler` - Handler for interpreting byte order
    /// * `file_path` - Path to the TIFF file
    fn display_tiepoint(&self, ifd: &IFD,
                        byte_order_handler: &Box<dyn crate::io::byte_order::ByteOrderHandler>,
                        file_path: &str) {
        if let Ok(tiepoint) = GeoKeyParser::read_model_tiepoint_values(ifd, byte_order_handler, file_path) {
            if tiepoint.len() >= 6 {
                info!("  Tiepoint: Raster({:.1},{:.1},{:.1}) → Map({:.6},{:.6},{:.6})",
                      tiepoint[0], tiepoint[1], tiepoint[2],
                      tiepoint[3], tiepoint[4], tiepoint[5]);
            }
        }
    }

    /// Display GeoKey directory information
    ///
    /// Shows the GeoKey directory entries and provides additional
    /// information for specific keys like ProjectedCSTypeGeoKey.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing GeoTIFF information
    /// * `byte_order_handler` - Handler for interpreting byte order
    /// * `file_path` - Path to the TIFF file
    fn display_geokey_directory(&self, ifd: &IFD,
                                byte_order_handler: &Box<dyn crate::io::byte_order::ByteOrderHandler>,
                                file_path: &str) {
        if let Ok(geo_key_data) = GeoKeyParser::format_geo_keys(ifd, byte_order_handler, file_path) {
            if !geo_key_data.is_empty() {
                info!("  GeoKey Directory:");
                for (key_id, key_name, tiff_tag_location, count, value_offset, value_str) in &geo_key_data {
                    info!("    Key {} ({}): Location={}, Count={}, Value={}",
                          key_id, key_name, tiff_tag_location, count, value_str);

                    // Add extra information for certain keys
                    if *key_id == geo_keys::PROJECTED_CS_TYPE && *tiff_tag_location == 0 {
                        let code = *value_offset as u16;
                        info!("      → {}", get_projected_cs_description(code));
                    }
                }
            }
        }
    }

    /// Display the PROJ.4 string
    ///
    /// Generates and displays a PROJ.4 compatible projection string
    /// based on the GeoTIFF information.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing GeoTIFF information
    /// * `byte_order_handler` - Handler for interpreting byte order
    /// * `file_path` - Path to the TIFF file
    fn display_proj_string(&self, ifd: &IFD,
                           byte_order_handler: &Box<dyn crate::io::byte_order::ByteOrderHandler>,
                           file_path: &str) {
        if let Ok(geo_info) = GeoKeyParser::extract_geo_info(ifd, byte_order_handler, file_path) {
            let proj_string = GeoKeyParser::format_projection_string(&geo_info);
            info!("  PROJ.4 String:");
            info!("    {}", proj_string);
        }
    }

    /// Display a summary of the first few tags
    ///
    /// Shows detailed information for a subset of tags to avoid
    /// overwhelming output for large IFDs.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to summarize
    fn display_tag_summary(&self, ifd: &IFD) {
        let max_tags = 10;
        info!("  First {} tags:", ifd.entries.len().min(max_tags));
        for (j, entry) in ifd.entries.iter().take(max_tags).enumerate() {
            debug!("    {}: Tag {} (type: {}, count: {}, value/offset: {})",
                   j, entry.tag, entry.field_type, entry.count, entry.value_offset);
        }

        if ifd.entries.len() > max_tags {
            info!("    ... ({} more tags)", ifd.entries.len() - max_tags);
        }
    }
}

impl<'a> Command for AnalyzeCommand<'a> {
    fn execute(&self) -> TiffResult<()> {
        info!("Analyzing file: {}", self.input_file);

        if self.verbose {
            debug!("Verbose mode enabled");
        }

        // Create and use TIFF reader
        let mut reader = TiffReader::new(self.logger);
        let tiff = reader.load(&self.input_file)?;

        // Display basic TIFF information
        self.display_tiff_summary(&tiff);

        // Variable to track if any GeoTIFF tags were found
        let mut has_geotiff_tags = false;

        // Process each IFD
        for (i, ifd) in tiff.ifds.iter().enumerate() {
            // Display basic IFD info
            self.display_ifd_summary(ifd, i);

            // Display compression info
            self.display_compression_info(ifd);

            // Display subfile type
            self.display_subfile_type(ifd);

            // Check and display GeoTIFF tags
            let ifd_has_geotiff = self.display_geotiff_tags(ifd);

            if ifd_has_geotiff {
                has_geotiff_tags = true;

                // Display detailed GeoTIFF info
                self.display_geotiff_details(&reader, ifd);
            }

            // Display tag summary
            self.display_tag_summary(ifd);
        }

        debug!("Analysis completed successfully");
        self.logger.log("Analysis completed successfully")?;

        Ok(())
    }
}