//! Image extractor strategy definitions
//!
//! This module defines the strategy pattern for different image format extractors,
//! allowing for extensible support of various file formats.

use std::path::Path;
use image::DynamicImage;
use log::{info, debug, error};

use crate::utils::logger::Logger;
use crate::tiff::errors::{TiffError, TiffResult};

use super::region::Region;
use super::array_strategy::ArrayData;

/// Strategy for extracting images from different formats
///
/// This trait defines the interface that all format extractors must implement.
/// It allows for a pluggable system where new formats can be easily added.
pub trait ExtractorStrategy {
    /// Extract an image from a file to another file
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `output_path` - Path where the extracted image should be saved
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    /// * `shape` - Optional shape to use ("circle" or "square")
    ///
    /// # Returns
    /// Result indicating success or an error with details
    fn extract_to_file(&mut self, source_path: &str, output_path: &str,
                       region: Option<Region>, shape: Option<&str>) -> TiffResult<()>;

    /// Extract an image from a file to memory
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted image or an error
    fn extract_image(&mut self, source_path: &str,
                     region: Option<Region>) -> TiffResult<DynamicImage>;

    /// Extract array data from a file to another file
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `output_path` - Path where the extracted array should be saved
    /// * `format` - Format for the output (e.g., "csv", "json", "npy")
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result indicating success or an error with details
    fn extract_to_array(&mut self, source_path: &str, output_path: &str,
                        format: &str, region: Option<Region>) -> TiffResult<()>;

    /// Extract array data from a file to memory
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted array data or an error
    fn extract_array_data(&mut self, source_path: &str,
                          region: Option<Region>) -> TiffResult<ArrayData>;

    /// Check if this strategy supports the given file format
    ///
    /// # Arguments
    /// * `file_path` - Path to check for format compatibility
    ///
    /// # Returns
    /// `true` if this strategy can handle the file format, `false` otherwise
    fn supports_format(&self, file_path: &str) -> bool {
        // Default implementation checks for TIFF files
        let extension = Path::new(file_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_lowercase();

        matches!(extension.as_str(), "tif" | "tiff")
    }
}

/// Factory for creating appropriate extractor strategies
///
/// This factory examines file extensions and creates the appropriate
/// strategy for handling that file format.
pub struct ExtractorStrategyFactory<'a> {
    /// Logger for recording operations
    logger: &'a Logger,
    /// Flag to indicate if we should use the array extractor
    use_array_extractor: bool,
}

impl<'a> ExtractorStrategyFactory<'a> {
    /// Create a new factory instance
    ///
    /// # Arguments
    /// * `logger` - Logger for recording operations
    /// * `use_array_extractor` - Whether to use array extraction instead of image extraction
    pub fn new(logger: &'a Logger, use_array_extractor: bool) -> Self {
        ExtractorStrategyFactory {
            logger,
            use_array_extractor,
        }
    }

    /// Create an appropriate strategy for the given file path
    ///
    /// # Arguments
    /// * `file_path` - Path to the file that needs to be processed
    ///
    /// # Returns
    /// A strategy that can handle the file format, or an error if unsupported
    pub fn create_strategy(&self, file_path: &str) -> TiffResult<Box<dyn ExtractorStrategy + 'a>> {
        // Extract file extension and convert to lowercase for case-insensitive matching
        let extension = Path::new(file_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_lowercase();

        debug!("Determining strategy for file extension: {}", extension);

        // Create the appropriate strategy based on file extension and extraction mode
        match extension.as_str() {
            "tif" | "tiff" => {
                if self.use_array_extractor {
                    info!("Using array extractor strategy for {}", file_path);
                    Ok(Box::new(super::array_strategy::ArrayExtractorStrategy::new(self.logger)))
                } else {
                    info!("Using TIFF extractor strategy for {}", file_path);
                    Ok(Box::new(super::tiff_strategy::TiffExtractorStrategy::new(self.logger)))
                }
            },
            // Add more formats here as needed
            _ => {
                error!("Unsupported file format: {}", extension);
                Err(TiffError::GenericError(format!("Unsupported file format: {}", extension)))
            }
        }
    }
}

/// Main extractor that delegates to appropriate format strategies
///
/// This facade provides a simple interface for extracting images from
/// various formats without exposing the strategy details.
pub struct ImageExtractor<'a> {
    /// Logger for recording operations
    logger: &'a Logger,
    /// Factory for creating format-specific strategies
    factory: ExtractorStrategyFactory<'a>,
}

impl<'a> ImageExtractor<'a> {
    /// Create a new image extractor
    ///
    /// # Arguments
    /// * `logger` - Logger for recording operations
    pub fn new(logger: &'a Logger) -> Self {
        ImageExtractor {
            logger,
            factory: ExtractorStrategyFactory::new(logger, false),
        }
    }

    /// Create a new array extractor
    ///
    /// # Arguments
    /// * `logger` - Logger for recording operations
    pub fn new_array_extractor(logger: &'a Logger) -> Self {
        ImageExtractor {
            logger,
            factory: ExtractorStrategyFactory::new(logger, true),
        }
    }

    /// Extract an image region from a file to another file
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `output_path` - Path where the extracted image should be saved
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    /// * `shape` - Optional shape to use ("circle" or "square")
    ///
    /// # Returns
    /// Result indicating success or an error with details
    pub fn extract_to_file(&mut self, source_path: &str, output_path: &str,
                           region: Option<Region>, shape: Option<&str>) -> TiffResult<()> {
        info!("Extracting from {} to {}", source_path, output_path);

        // Create an appropriate strategy for this file format
        let mut strategy = self.factory.create_strategy(source_path)?;

        // Delegate the extraction to the strategy
        strategy.extract_to_file(source_path, output_path, region, shape)
    }

    /// Extract an image from a file to memory
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted image or an error
    pub fn extract_image(&mut self, source_path: &str,
                         region: Option<Region>) -> TiffResult<DynamicImage> {
        info!("Extracting image from {} to memory", source_path);

        // Create an appropriate strategy for this file format
        let mut strategy = self.factory.create_strategy(source_path)?;

        // Delegate the extraction to the strategy
        strategy.extract_image(source_path, region)
    }

    /// Extract array data from a file to another file
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `output_path` - Path where the extracted array should be saved
    /// * `format` - Format for the output (e.g., "csv", "json", "npy")
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result indicating success or an error with details
    pub fn extract_to_array(&mut self, source_path: &str, output_path: &str,
                            format: &str, region: Option<Region>) -> TiffResult<()> {
        info!("Extracting array data from {} to {} in {} format",
              source_path, output_path, format);

        // Create an appropriate strategy for this file format
        let mut strategy = self.factory.create_strategy(source_path)?;

        // Delegate the extraction to the strategy
        strategy.extract_to_array(source_path, output_path, format, region)
    }

    /// Extract array data from a file to memory
    ///
    /// # Arguments
    /// * `source_path` - Path to the source image file
    /// * `region` - Optional region to extract (if None, extracts the entire image)
    ///
    /// # Returns
    /// Result containing the extracted array data or an error
    pub fn extract_array_data(&mut self, source_path: &str,
                              region: Option<Region>) -> TiffResult<ArrayData> {
        info!("Extracting array data from {} to memory", source_path);

        // Create an appropriate strategy for this file format
        let mut strategy = self.factory.create_strategy(source_path)?;

        // Delegate the extraction to the strategy
        strategy.extract_array_data(source_path, region)
    }
}