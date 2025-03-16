//! Strip-based image data extraction
//!
//! This module implements a reader for extracting image data from stripped TIFF files.
//! Stripped TIFFs organize image data in horizontal strips across the entire width
//! of the image, which is the traditional TIFF format and well-suited for line-by-line
//! processing.

use log::{debug, info, warn};
use std::io::SeekFrom;
use image::{ImageBuffer, Rgb};

use crate::io::seekable::SeekableReader;
use crate::tiff::{TiffReader, TiffError};
use crate::tiff::errors::TiffResult;
use crate::tiff::ifd::IFD;
use crate::tiff::constants::{tags, predictor as pred_consts};
use crate::compression::CompressionFactory;
use crate::utils::image_extraction_utils;

use super::region::Region;

/// Reads image data from stripped TIFF files
///
/// This reader handles the extraction of pixel data from TIFFs that use
/// the stripped data organization, including handling of various compression
/// methods and coordinate mapping.
pub struct StripReader<'a, R: SeekableReader> {
    /// Reader for accessing the TIFF file
    reader: R,
    /// IFD containing the image metadata
    ifd: &'a IFD,
    /// TIFF reader for accessing tag values
    tiff_reader: &'a TiffReader<'a>,
}

impl<'a, R: SeekableReader> StripReader<'a, R> {
    /// Create a new strip reader
    ///
    /// # Arguments
    /// * `reader` - Seekable reader for the TIFF file
    /// * `ifd` - IFD containing the image metadata
    /// * `tiff_reader` - TIFF reader for accessing tag values
    ///
    /// # Returns
    /// A new StripReader instance
    pub fn new(reader: R, ifd: &'a IFD, tiff_reader: &'a TiffReader<'a>) -> Self {
        StripReader {
            reader,
            ifd,
            tiff_reader
        }
    }

    /// Get strip parameters from the IFD
    ///
    /// Reads the rows per strip and image width from the IFD.
    /// If RowsPerStrip is not specified, defaults to the entire image height.
    ///
    /// # Returns
    /// A tuple containing (rows_per_strip, image_width) or an error
    fn get_strip_parameters(&self) -> TiffResult<(u32, u32)> {
        // Get image dimensions
        let (img_width, _) = self.ifd.get_dimensions()
            .ok_or_else(|| TiffError::GenericError("Missing image dimensions".to_string()))?;

        // Get rows per strip, defaulting to the full image height
        let rows_per_strip = self.ifd.get_tag_value(tags::ROWS_PER_STRIP)
            .unwrap_or(img_width) as u32;

        Ok((rows_per_strip, img_width as u32))
    }

    /// Read a single strip from the TIFF file
    ///
    /// Reads and decompresses a strip from the TIFF file, applying
    /// the appropriate predictor if needed.
    ///
    /// # Arguments
    /// * `offset` - File offset where the strip data starts
    /// * `byte_count` - Size of the strip data in bytes
    /// * `compression_handler` - Handler for the compression method used
    /// * `predictor` - Predictor used for the image data
    /// * `width` - Width of the image in pixels
    /// * `rows_per_strip` - Number of rows in each strip
    ///
    /// # Returns
    /// Strip data as a byte vector, or an error
    fn read_strip(
        &mut self,
        offset: u64,
        byte_count: u64,
        compression_handler: &dyn crate::compression::CompressionHandler,
        predictor: usize,
        width: usize,
        rows_per_strip: usize
    ) -> TiffResult<Vec<u8>> {
        // Read the compressed strip data
        self.reader.seek(SeekFrom::Start(offset))?;
        let mut compressed_data = vec![0u8; byte_count as usize];
        self.reader.read_exact(&mut compressed_data)?;

        // Decompress the strip data
        let mut strip_data = compression_handler.decompress(&compressed_data)?;

        // Apply predictor if needed
        if predictor == pred_consts::HORIZONTAL_DIFFERENCING as usize {
            image_extraction_utils::apply_horizontal_predictor(&mut strip_data, width, rows_per_strip);
        }

        Ok(strip_data)
    }

    /// Extract image data to the provided buffer
    ///
    /// Reads all strips that intersect with the specified region and
    /// copies their pixel data to the output image.
    ///
    /// # Arguments
    /// * `image` - Output image buffer
    /// * `region` - Region of the image to extract
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn extract(
        &mut self,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        region: Region
    ) -> TiffResult<()> {
        // Get strip parameters
        let (rows_per_strip, img_width) = self.get_strip_parameters()?;

        // Get compression type
        let compression = self.ifd.get_tag_value(tags::COMPRESSION).unwrap_or(1);
        let compression_handler = CompressionFactory::create_handler(compression)?;
        info!("Using compression: {}", compression_handler.name());

        // Get predictor
        let predictor = self.ifd.get_tag_value(tags::PREDICTOR).unwrap_or(1) as usize;

        // Get strip offsets and byte counts
        let strip_offsets = self.tiff_reader.read_tag_values(&mut self.reader, self.ifd, tags::STRIP_OFFSETS)?;
        let strip_byte_counts = self.tiff_reader.read_tag_values(&mut self.reader, self.ifd, tags::STRIP_BYTE_COUNTS)?;

        info!("Rows per strip: {}", rows_per_strip);
        info!("Total strips: {}", strip_offsets.len());

        // Calculate which strips we need
        let start_strip = region.y / rows_per_strip;
        let end_strip = (region.end_y() + rows_per_strip - 1) / rows_per_strip;

        info!("Processing strips from {} to {}", start_strip, end_strip - 1);

        // Process each strip
        for strip_idx in start_strip..end_strip {
            // Skip if strip index is out of bounds
            if strip_idx as usize >= strip_offsets.len() {
                warn!("Strip index {} out of bounds (max {})",
                      strip_idx, strip_offsets.len() - 1);
                continue;
            }

            let offset = strip_offsets[strip_idx as usize];
            let byte_count = strip_byte_counts[strip_idx as usize];

            debug!("Reading strip {} at offset {} with {} bytes",
                  strip_idx, offset, byte_count);

            // Read and process the strip data
            let strip_data = match self.read_strip(
                offset,
                byte_count,
                &*compression_handler,
                predictor,
                img_width as usize,
                rows_per_strip as usize
            ) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Error reading strip {}: {:?}", strip_idx, e);
                    continue;
                }
            };

            // Calculate strip position in pixels
            let strip_start_y = strip_idx * rows_per_strip;

            // Copy strip data to image buffer
            self.copy_strip_to_image(
                &strip_data,
                image,
                img_width as usize,
                rows_per_strip as usize,
                strip_start_y,
                region
            );
        }

        Ok(())
    }

    /// Copy strip data to the image buffer
    ///
    /// Maps pixels from the strip to the appropriate positions in the output image,
    /// handling coordinate conversions and boundary checks.
    ///
    /// # Arguments
    /// * `strip_data` - Decompressed strip data
    /// * `image` - Output image buffer
    /// * `width` - Width of the image in pixels
    /// * `rows_in_strip` - Number of rows in the strip
    /// * `strip_start_y` - Y coordinate of the strip's top row
    /// * `region` - Region being extracted
    fn copy_strip_to_image(
        &self,
        strip_data: &[u8],
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        width: usize,
        rows_in_strip: usize,
        strip_start_y: u32,
        region: Region
    ) {
        // For each row in the strip
        for row in 0..rows_in_strip {
            let global_y = strip_start_y + row as u32;

            // Skip rows outside our region
            if global_y < region.y || global_y >= region.end_y() {
                continue;
            }

            let row_start = row * width;

            // For each pixel in the row within our region
            for x in region.x..region.end_x() {
                // Skip pixels outside image width
                if x >= width as u32 {
                    continue;
                }

                let strip_idx = row_start + x as usize;

                // Copy the pixel using the utility function
                image_extraction_utils::copy_pixel(
                    strip_data,
                    image,
                    x,
                    global_y,
                    strip_idx,
                    region
                );
            }
        }
    }
}