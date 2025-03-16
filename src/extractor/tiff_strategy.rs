//! TIFF format extractor strategy implementation
//!
//! This module implements the extraction strategy for TIFF format images,
//! handling both the standard TIFF format and GeoTIFF extensions.

use log::info;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use image::{ImageBuffer, Rgb, DynamicImage};
use crate::extractor::array_strategy::ArrayData;
use crate::tiff::{TiffReader, TiffBuilder};
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::IFD;
use crate::tiff::constants::{tags, photometric};
use crate::utils::logger::Logger;
use crate::utils::tiff_extraction_utils;

use super::region::Region;
use super::tile_reader::TileReader;
use super::strip_reader::StripReader;
use super::extractor_strategy::ExtractorStrategy;

/// TIFF format extractor implementation
///
/// This strategy handles extraction from TIFF files, including support for
/// tiled and stripped data organizations, and preserves GeoTIFF metadata.
pub struct TiffExtractorStrategy<'a> {
    /// Logger for recording operations
    logger: &'a Logger,
    /// TIFF reader for parsing TIFF files
    reader: TiffReader<'a>,
}

impl<'a> TiffExtractorStrategy<'a> {
    /// Create a new TIFF extractor strategy
    ///
    /// # Arguments
    /// * `logger` - Logger for recording operations
    pub fn new(logger: &'a Logger) -> Self {
        TiffExtractorStrategy {
            logger,
            reader: TiffReader::new(logger),
        }
    }
}

impl<'a> ExtractorStrategy for TiffExtractorStrategy<'a> {
    /// Extract an image from a TIFF file to another file
    ///
    /// This implementation preserves GeoTIFF metadata and handles both
    /// tiled and stripped data organizations.
    ///
    /// # Arguments
    /// * `tiff_path` - Path to the source TIFF file
    /// * `output_path` - Path where the extracted image should be saved
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result indicating success or an error with details
    fn extract_to_file(&mut self, tiff_path: &str, output_path: &str,
                       region: Option<Region>) -> TiffResult<()> {
        info!("Extracting image from {} to {}", tiff_path, output_path);

        // Load the source TIFF
        let tiff = self.reader.load(tiff_path)?;
        if tiff.ifds.is_empty() {
            return Err(TiffError::GenericError("No IFDs found in TIFF file".to_string()));
        }

        // Use the first IFD
        let original_ifd = &tiff.ifds[0];

        // Get basic image properties
        let (bits_per_sample, photometric, samples_per_pixel) =
            tiff_extraction_utils::get_tiff_image_properties(original_ifd);

        // Get the file path and GeoTIFF information
        let file_path = self.reader.get_file_path().unwrap_or(tiff_path);
        let (pixel_scale, tiepoint) = tiff_extraction_utils::read_geotiff_info(
            original_ifd, &self.reader, file_path);

        // Determine extraction region
        let extracted_region = region.unwrap_or_else(|| {
            if let Some((width, height)) = original_ifd.get_dimensions() {
                Region::new(0, 0, width as u32, height as u32)
            } else {
                Region::new(0, 0, 0, 0)
            }
        });

        info!("Extracting region: x={}, y={}, width={}, height={}",
              extracted_region.x, extracted_region.y,
              extracted_region.width, extracted_region.height);

        // Extract the image data
        let image = self.extract_image(tiff_path, Some(extracted_region))?;

        // Create a TIFF builder and set up base structure
        let mut builder = TiffBuilder::new(self.logger, false);
        let new_ifd = IFD::new(0, 0);
        let ifd_index = builder.add_ifd(new_ifd);

        // Set up common TIFF tags
        tiff_extraction_utils::setup_tiff_tags(&mut builder, ifd_index, original_ifd, &image)?;

        // Copy statistics tags
        builder.copy_statistics_tags(ifd_index, original_ifd);

        // Copy and adjust GeoTIFF metadata
        builder.copy_geotiff_tags(ifd_index, original_ifd, &mut self.reader)?;
        builder.adjust_geotiff_for_region(ifd_index, &extracted_region, &pixel_scale, &tiepoint)?;

        // Process image data based on format
        if samples_per_pixel == 1 {
            // Single band (grayscale) image
            tiff_extraction_utils::process_grayscale_image(&image, &mut builder, ifd_index, bits_per_sample)?;
        } else {
            // Multi-band (RGB) image
            tiff_extraction_utils::process_rgb_image(&image, &mut builder, ifd_index)?;
        }

        // Handle NoData value
        let nodata_value = tiff_extraction_utils::extract_nodata_value(original_ifd, &self.reader);
        let metadata_str = tiff_extraction_utils::extract_gdal_metadata(original_ifd, &self.reader);

        // Set NoData tag and metadata
        info!("Setting NoData value: '{}'", nodata_value);
        builder.add_nodata_tag(ifd_index, &nodata_value);
        builder.add_gdal_metadata_tag(ifd_index, metadata_str.as_deref(), &nodata_value);

        // Ensure proper photometric interpretation
        tiff_extraction_utils::set_photometric_interpretation(
            &mut builder, ifd_index, photometric::BLACK_IS_ZERO);

        // Write the file
        builder.write(output_path)?;

        info!("Saved {}x{} image to {} with adjusted GeoTIFF metadata",
              image.width(), image.height(), output_path);

        Ok(())
    }

    /// Extract an image from a TIFF file to memory
    ///
    /// # Arguments
    /// * `tiff_path` - Path to the source TIFF file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted image or an error
    fn extract_image(&mut self, tiff_path: &str,
                     region: Option<Region>) -> TiffResult<DynamicImage> {
        // Load the TIFF file
        let tiff = self.reader.load(tiff_path)?;

        if tiff.ifds.is_empty() {
            return Err(TiffError::GenericError("No IFDs found in TIFF file".to_string()));
        }

        // Use the first IFD
        let ifd = &tiff.ifds[0];

        // Determine and validate the extraction region
        let region = tiff_extraction_utils::determine_extraction_region(region, ifd)?;

        info!("Extracting region: ({}, {}) with size {}x{}",
              region.x, region.y, region.width, region.height);

        // Open file for reading
        let file = File::open(tiff_path)?;
        let reader = BufReader::with_capacity(1024 * 1024, file);

        // Extract the pixel data
        let mut image = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(region.width, region.height);

        // Check if we're using strips or tiles
        let is_tiled = ifd.has_tag(tags::TILE_WIDTH) && ifd.has_tag(tags::TILE_LENGTH);

        if is_tiled {
            let mut tile_reader = TileReader::new(reader, ifd, &self.reader);
            tile_reader.extract(&mut image, region)?;
        } else {
            let mut strip_reader = StripReader::new(reader, ifd, &self.reader);
            strip_reader.extract(&mut image, region)?;
        }

        Ok(DynamicImage::ImageRgb8(image))
    }

    // Existing method implementations...

    /// Extract array data from a file to another file
    ///
    /// # Arguments
    /// * `source_path` - Path to the source TIFF file
    /// * `output_path` - Path where the extracted array should be saved
    /// * `format` - Format for the output (e.g., "csv", "json", "npy")
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result indicating success or an error with details
    fn extract_to_array(&mut self, source_path: &str, output_path: &str,
                        format: &str, region: Option<Region>) -> TiffResult<()> {
        info!("TIFF strategy: Converting image to array format {}", format);

        // Extract array data
        let array_data = self.extract_array_data(source_path, region)?;

        // Save to file
        array_data.save_to_file(output_path, format)
    }

    /// Extract array data from a file to memory
    ///
    /// # Arguments
    /// * `source_path` - Path to the source TIFF file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted array data or an error
    fn extract_array_data(&mut self, source_path: &str,
                          region: Option<Region>) -> TiffResult<ArrayData> {
        info!("TIFF strategy: Extracting array data to memory");

        // Extract image first
        let image = self.extract_image(source_path, region)?;

        // Convert to array data
        Ok(ArrayData::from_image(&image))
    }

    /// Check if this strategy supports the given file format
    ///
    /// # Arguments
    /// * `file_path` - Path to check for TIFF format compatibility
    ///
    /// # Returns
    /// `true` if this is a TIFF file, `false` otherwise
    fn supports_format(&self, file_path: &str) -> bool {
        let extension = Path::new(file_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_lowercase();

        matches!(extension.as_str(), "tif" | "tiff")
    }
}