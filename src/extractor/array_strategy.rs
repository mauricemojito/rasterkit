//! Array data extractor strategy implementation
//!
//! This module implements the extraction strategy for TIFF files as raw arrays,
//! allowing access to the underlying pixel values as numeric data rather than
//! images.

use log::{info, debug, warn};
use std::fs::File;
use std::io::{BufWriter, Write, BufReader};
use std::path::Path;
use image::{DynamicImage, GenericImageView};

use crate::tiff::TiffReader;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::constants::tags;
use crate::utils::logger::Logger;

use super::region::Region;
use super::tile_reader::TileReader;
use super::strip_reader::StripReader;
use super::extractor_strategy::ExtractorStrategy;

/// Represents array data extracted from an image
///
/// This struct contains the raw numeric data along with
/// dimensional information for interpreting it.
#[derive(Debug, Clone)]
pub struct ArrayData {
    /// Width of the array (columns)
    pub width: u32,
    /// Height of the array (rows)
    pub height: u32,
    /// Raw data values in row-major order
    pub data: Vec<u8>,
}

impl ArrayData {
    /// Create a new ArrayData instance from an image
    ///
    /// # Arguments
    /// * `image` - Source image to extract data from
    ///
    /// # Returns
    /// A new ArrayData instance
    pub fn from_image(image: &DynamicImage) -> Self {
        let gray_image = image.to_luma8();
        let width = gray_image.width();
        let height = gray_image.height();
        let data = gray_image.into_raw();

        ArrayData {
            width,
            height,
            data,
        }
    }

    /// Get a specific value from the array
    ///
    /// # Arguments
    /// * `x` - Column index
    /// * `y` - Row index
    ///
    /// # Returns
    /// The value at the specified position, or None if out of bounds
    pub fn get(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let idx = (y * self.width + x) as usize;
        self.data.get(idx).copied()
    }

    /// Save the array to a file in the specified format
    ///
    /// # Arguments
    /// * `path` - Path to save the file
    /// * `format` - Format to use ("csv", "json", "npy")
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn save_to_file(&self, path: &str, format: &str) -> TiffResult<()> {
        match format.to_lowercase().as_str() {
            "csv" => self.save_as_csv(path),
            "json" => self.save_as_json(path),
            "npy" => self.save_as_npy(path),
            _ => Err(TiffError::GenericError(format!("Unsupported array format: {}", format))),
        }
    }

    /// Save the array as CSV
    ///
    /// # Arguments
    /// * `path` - Path to save the CSV file
    ///
    /// # Returns
    /// Result indicating success or an error
    fn save_as_csv(&self, path: &str) -> TiffResult<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write CSV header with column numbers (optional)
        write!(writer, "row/col")?;
        for x in 0..self.width {
            write!(writer, ",{}", x)?;
        }
        writeln!(writer)?;

        // Write data rows
        for y in 0..self.height {
            // Row number as first column
            write!(writer, "{}", y)?;

            // Write pixel values for this row
            for x in 0..self.width {
                if let Some(value) = self.get(x, y) {
                    write!(writer, ",{}", value)?;
                } else {
                    write!(writer, ",")?;
                }
            }
            writeln!(writer)?;
        }

        Ok(())
    }

    /// Save the array as JSON
    ///
    /// # Arguments
    /// * `path` - Path to save the JSON file
    ///
    /// # Returns
    /// Result indicating success or an error
    fn save_as_json(&self, path: &str) -> TiffResult<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Start JSON structure
        writeln!(writer, "{{")?;
        writeln!(writer, "  \"width\": {},", self.width)?;
        writeln!(writer, "  \"height\": {},", self.height)?;
        writeln!(writer, "  \"data\": [")?;

        // Write rows as nested arrays
        for y in 0..self.height {
            write!(writer, "    [")?;

            for x in 0..self.width {
                if let Some(value) = self.get(x, y) {
                    write!(writer, "{}", value)?;
                } else {
                    write!(writer, "0")?;
                }

                // Add comma if not the last element
                if x < self.width - 1 {
                    write!(writer, ", ")?;
                }
            }

            // Close row array with comma if not the last row
            if y < self.height - 1 {
                writeln!(writer, "],")?;
            } else {
                writeln!(writer, "]")?;
            }
        }

        // Close JSON structure
        writeln!(writer, "  ]")?;
        writeln!(writer, "}}")?;

        Ok(())
    }

    /// Save the array as NumPy NPY file
    ///
    /// # Arguments
    /// * `path` - Path to save the NPY file
    ///
    /// # Returns
    /// Result indicating success or an error
    fn save_as_npy(&self, path: &str) -> TiffResult<()> {
        let mut file = File::create(path)?;

        // NPY format magic string and version
        file.write_all(b"\x93NUMPY")?;  // Magic string
        file.write_all(&[0x01, 0x00])?; // Version 1.0

        // Create header string
        let header_str = format!(
            "{{'descr': '<u1', 'fortran_order': False, 'shape': ({}, {}), }}",
            self.height, self.width
        );

        // Calculate padding to make header + length marker divisible by 64
        let header_len = header_str.len() + 1; // +1 for newline
        let padding_len = (64 - ((header_len + 10) % 64)) % 64;
        let padded_header = format!("{}{}\n", header_str, " ".repeat(padding_len));

        // Write header length and header
        file.write_all(&[(padded_header.len() as u8) & 0xFF])?;
        file.write_all(&[0x00])?; // For version 1.0, header length is 2 bytes
        file.write_all(padded_header.as_bytes())?;

        // Write image data as raw bytes
        file.write_all(&self.data)?;

        Ok(())
    }
}

/// Array extractor strategy implementation for TIFF files
///
/// This strategy handles extraction of raw numeric data from TIFF files,
/// providing the pixel values as arrays in various formats.
pub struct ArrayExtractorStrategy<'a> {
    /// Logger for recording operations
    logger: &'a Logger,
    /// TIFF reader for parsing TIFF files
    reader: TiffReader<'a>,
}

impl<'a> ArrayExtractorStrategy<'a> {
    /// Create a new array extractor strategy
    ///
    /// # Arguments
    /// * `logger` - Logger for recording operations
    ///
    /// # Returns
    /// A new ArrayExtractorStrategy instance
    pub fn new(logger: &'a Logger) -> Self {
        ArrayExtractorStrategy {
            logger,
            reader: TiffReader::new(logger),
        }
    }
}

impl<'a> ExtractorStrategy for ArrayExtractorStrategy<'a> {
    /// Extract an image from a file to another file
    ///
    /// For array strategy, this is not the primary method but is implemented
    /// to satisfy the trait requirements. Simply delegates to extract_to_array
    /// with CSV format.
    fn extract_to_file(&mut self, source_path: &str, output_path: &str,
                       region: Option<Region>) -> TiffResult<()> {
        // Default to CSV format for compatibility
        self.extract_to_array(source_path, output_path, "csv", region)
    }

    /// Extract an image from a file to memory
    ///
    /// This method extracts the image data that will be converted to arrays.
    /// Reuses the same extraction logic as the image extractor.
    fn extract_image(&mut self, source_path: &str,
                     region: Option<Region>) -> TiffResult<DynamicImage> {
        // Load the TIFF file
        let tiff = self.reader.load(source_path)?;

        if tiff.ifds.is_empty() {
            return Err(TiffError::GenericError("No IFDs found in TIFF file".to_string()));
        }

        // Use the first IFD
        let ifd = &tiff.ifds[0];

        // Determine and validate the extraction region
        let region = crate::utils::tiff_extraction_utils::determine_extraction_region(region, ifd)?;

        info!("Extracting region: ({}, {}) with size {}x{}",
              region.x, region.y, region.width, region.height);

        // Open file for reading
        let file = File::open(source_path)?;
        let reader = BufReader::with_capacity(1024 * 1024, file);

        // Extract the pixel data
        let mut image = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(region.width, region.height);

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

    /// Extract array data from a file to another file
    ///
    /// This method extracts raw array data and saves it to a file
    /// in the specified format.
    ///
    /// # Arguments
    /// * `source_path` - Path to the source TIFF file
    /// * `output_path` - Path where the extracted array should be saved
    /// * `format` - Format for the output ("csv", "json", or "npy")
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result indicating success or an error with details
    fn extract_to_array(&mut self, source_path: &str, output_path: &str,
                        format: &str, region: Option<Region>) -> TiffResult<()> {
        info!("Extracting array data from {} to {} in {} format",
              source_path, output_path, format);

        // Extract the array data
        let array_data = self.extract_array_data(source_path, region)?;

        // Save to file in the requested format
        array_data.save_to_file(output_path, format)
    }

    /// Extract array data from a file to memory
    ///
    /// This method extracts raw array data and returns it without
    /// writing to a file, giving more flexibility to the user.
    ///
    /// # Arguments
    /// * `source_path` - Path to the source TIFF file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted array data or an error
    fn extract_array_data(&mut self, source_path: &str,
                          region: Option<Region>) -> TiffResult<ArrayData> {
        info!("Extracting array data from {} to memory", source_path);

        // First extract the image
        let image = self.extract_image(source_path, region)?;

        // Convert to array data
        Ok(ArrayData::from_image(&image))
    }
}