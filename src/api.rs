use std::path::Path;
use log::info;
use crate::tiff::errors::TiffResult;
use crate::utils::logger::Logger;
use crate::extractor::{Region, ImageExtractor};
use crate::coordinate::BoundingBox;
use crate::compression::CompressionConverter;
use crate::compression::CompressionFactory;

/// Main interface to the RasterKit library
pub struct RasterKit {
    logger: Logger,
}

impl RasterKit {
    /// Create a new RasterKit instance
    ///
    /// # Arguments
    /// * `log_file` - Optional path to log file, defaults to "rasterkit.log"
    ///
    /// # Returns
    /// A RasterKit instance or an error if initialization fails
    pub fn new(log_file: Option<&str>) -> TiffResult<Self> {
        let log_path = log_file.unwrap_or("rasterkit.log");
        let logger = Logger::new(log_path)?;
        Ok(RasterKit { logger })
    }

    /// Analyze a TIFF file and return information about its structure
    ///
    /// # Arguments
    /// * `input_path` - Path to the TIFF file to analyze
    ///
    /// # Returns
    /// String containing analysis information or an error
    pub fn analyze(&self, input_path: &str) -> TiffResult<String> {
        // Create and configure the analyzer
        let mut analyzer = crate::commands::analyze_command::AnalyzeCommand::new(
            &clap::ArgMatches::default(),
            &self.logger,
        )?;

        // Use the analyzer's functionality but capture the output
        // Create a TIFF reader and load the file
        let mut reader = crate::tiff::TiffReader::new(&self.logger);
        let tiff = reader.load(input_path)?;

        // Format a summary of the file
        let mut result = format!("TIFF Analysis Results:\n");
        result.push_str(&format!("  Format: {}\n", if tiff.is_big_tiff { "BigTIFF" } else { "TIFF" }));
        result.push_str(&format!("  Number of IFDs: {}\n", tiff.ifd_count()));

        // Add details for each IFD
        for (i, ifd) in tiff.ifds.iter().enumerate() {
            result.push_str(&format!("\nIFD #{} (offset: {})\n", i, ifd.offset));
            result.push_str(&format!("  Number of entries: {}\n", ifd.entries.len()));

            if let Some((width, height)) = ifd.get_dimensions() {
                result.push_str(&format!("  Dimensions: {}x{}\n", width, height));
            }

            result.push_str(&format!("  Samples per pixel: {}\n", ifd.get_samples_per_pixel()));

            // Add compression info
            if let Some(entry) = ifd.get_entry(crate::tiff::constants::tags::COMPRESSION) {
                result.push_str(&format!("  Compression: {} ({})\n",
                                         entry.value_offset,
                                         crate::utils::tiff_code_translators::compression_code_to_name(entry.value_offset)));
            }
        }

        Ok(result)
    }

    /// Extract an image from a TIFF file
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `output_path` - Path where to save the extracted image
    /// * `region` - Optional pixel region to extract (x, y, width, height)
    /// * `bbox` - Optional geographic bounding box as "minx,miny,maxx,maxy"
    /// * `epsg` - Optional EPSG code for the bounding box coordinates
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn extract(&self,
                   input_path: &str,
                   output_path: &str,
                   region: Option<(u32, u32, u32, u32)>,
                   bbox: Option<&str>,
                   epsg: Option<u32>) -> TiffResult<()> {

        let mut extractor = ImageExtractor::new(&self.logger);

        // Determine the extraction region
        let extraction_region = if let Some((x, y, width, height)) = region {
            Some(Region::new(x, y, width, height))
        } else if let Some(bbox_str) = bbox {
            // Parse bounding box and convert to pixel coordinates
            let bbox = BoundingBox::from_string(bbox_str)
                .map_err(|e| crate::tiff::errors::TiffError::GenericError(e))?;

            // Set EPSG code if provided
            let bbox_with_epsg = if let Some(code) = epsg {
                let mut bb = bbox;
                bb.epsg = Some(code);
                bb
            } else {
                bbox
            };

            // Create a reader to access the TIFF file
            let mut reader = crate::tiff::TiffReader::new(&self.logger);
            let tiff = reader.load(input_path)?;

            // Determine the extraction region from the bounding box
            Some(crate::utils::image_extraction_utils::determine_extraction_region(
                bbox_with_epsg, &tiff, &reader, input_path, &self.logger)?
            )
        } else {
            None // Extract the entire image
        };

        // Perform the extraction
        extractor.extract_to_file(input_path, output_path, extraction_region)
    }

    /// Convert compression format of a TIFF file
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `output_path` - Path where to save the converted file
    /// * `compression` - Target compression method ("none", "deflate", "zstd")
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn convert_compression(&self,
                               input_path: &str,
                               output_path: &str,
                               compression: &str) -> TiffResult<()> {

        // Get compression code from the name
        let handler = CompressionFactory::get_handler_by_name(compression)?;
        let compression_code = handler.code();

        // Create converter and convert the file
        let mut converter = CompressionConverter::new(&self.logger);
        converter.convert_file(input_path, output_path, compression_code)
    }

    /// Extract the colormap from a TIFF file
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `output_path` - Path where to save the colormap
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn extract_colormap(&self, input_path: &str, output_path: &str) -> TiffResult<()> {
        crate::utils::colormap_utils::extract_colormap(input_path, output_path, &self.logger)
    }

    /// Apply a colormap to an image during extraction
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `output_path` - Path where to save the extracted image
    /// * `colormap_path` - Path to the colormap file to apply
    /// * `region` - Optional region to extract
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn extract_with_colormap(&self,
                                 input_path: &str,
                                 output_path: &str,
                                 colormap_path: &str,
                                 region: Option<(u32, u32, u32, u32)>) -> TiffResult<()> {

        let extraction_region = region.map(|(x, y, w, h)| Region::new(x, y, w, h));

        // Load the colormap
        let colormap = crate::utils::colormap_utils::load_colormap(colormap_path, &self.logger)?;

        // Create extractor and extract the image
        let mut extractor = ImageExtractor::new(&self.logger);
        let image = extractor.extract_image(input_path, extraction_region)?;

        // Convert to grayscale and apply colormap
        let grayscale = image.to_luma8();
        let rgb_image = crate::utils::colormap_utils::apply_colormap_to_image(&grayscale, &colormap);

        // Save the result
        crate::utils::colormap_utils::save_colorized_tiff(
            rgb_image,
            output_path,
            input_path,
            extraction_region,
            &self.logger
        )
    }

    /// List available compression methods
    ///
    /// # Returns
    /// Vector of compression method names
    pub fn list_compression_methods(&self) -> Vec<String> {
        let handlers = CompressionFactory::get_available_handlers();
        handlers.iter().map(|h| h.name().to_string()).collect()
    }

    /// Extract array data from a TIFF file to another file
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `output_path` - Path where to save the extracted array
    /// * `format` - Format for the output (csv, json, or npy)
    /// * `region` - Optional pixel region to extract (x, y, width, height)
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn extract_to_array(&self,
                            input_path: &str,
                            output_path: &str,
                            format: &str,
                            region: Option<(u32, u32, u32, u32)>) -> TiffResult<()> {
        info!("Extracting array data from {} to {} in {} format",
         input_path, output_path, format);

        // Create an array extractor
        let mut extractor = crate::extractor::ImageExtractor::new_array_extractor(&self.logger);

        // Convert region format if provided
        let extraction_region = region.map(|(x, y, width, height)| Region::new(x, y, width, height));

        // Extract to file in the specified format
        extractor.extract_to_array(input_path, output_path, format, extraction_region)
    }

    /// Extract array data from a TIFF file to memory
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `region` - Optional pixel region to extract (x, y, width, height)
    ///
    /// # Returns
    /// Result containing the array data or an error
    pub fn extract_array_data(&self,
                              input_path: &str,
                              region: Option<(u32, u32, u32, u32)>) -> TiffResult<crate::extractor::ArrayData> {
        info!("Extracting array data from {} to memory", input_path);

        // Create an array extractor
        let mut extractor = crate::extractor::ImageExtractor::new_array_extractor(&self.logger);

        // Convert region format if provided
        let extraction_region = region.map(|(x, y, width, height)| Region::new(x, y, width, height));

        // Extract array data
        extractor.extract_array_data(input_path, extraction_region)
    }
}