//! Image extraction command
//!
//! This module implements the command for extracting image data
//! from TIFF files, with support for region extraction, coordinate conversion,
//! and colormap handling.

use clap::ArgMatches;
use log::{debug, info, warn, error};
use std::path::Path;

use crate::commands::command_traits::Command;
use crate::tiff::errors::{TiffResult, TiffError};
use crate::utils::logger::Logger;
use crate::extractor::{ImageExtractor, Region};
use crate::coordinate::BoundingBox;
use crate::tiff::TiffReader;
use crate::tiff::constants::epsg;
use crate::tiff::types::TIFF;
use crate::utils::colormap_utils;
use crate::utils::reference_utils;
use crate::utils::image_extraction_utils;

/// Command for extracting image data from TIFF files
pub struct ExtractCommand<'a> {
    /// Path to the input file
    input_file: String,
    /// Path to the output file
    output_file: String,
    /// Bounding box string for region extraction
    bbox_str: Option<String>,
    /// EPSG code for the bounding box coordinates
    epsg_code: Option<u32>,
    /// Path to save the colormap as SLD (optional)
    colormap_output: Option<String>,
    /// Path to a colormap file to apply (optional)
    colormap_input: Option<String>,
    /// Whether to extract array data instead of image
    array_mode: bool,
    /// Format for array output
    array_format: String,
    /// Logger for recording operations
    logger: &'a Logger,
}

impl<'a> ExtractCommand<'a> {
    /// Create a new extract command
    ///
    /// # Arguments
    /// * `args` - CLI argument matches from clap
    /// * `logger` - Logger for recording operations
    ///
    /// # Returns
    /// A new ExtractCommand instance or an error
    pub fn new(args: &ArgMatches, logger: &'a Logger) -> TiffResult<Self> {
        info!("Creating new extract command from arguments");

        let input_file = args.get_one::<String>("input")
            .ok_or_else(|| TiffError::GenericError("Missing input file".to_string()))?
            .clone();
        info!("Input file: {}", input_file);

        let output_file = args.get_one::<String>("output")
            .ok_or_else(|| TiffError::GenericError("Missing output file path for extraction".to_string()))?
            .clone();
        info!("Output file: {}", output_file);

        // Get bounding box string if provided
        let bbox_str = args.get_one::<String>("bbox").cloned();
        info!("Bounding box: {:?}", bbox_str);

        // Get EPSG code if provided
        let epsg_code = if bbox_str.is_some() {
            // Only parse EPSG if bbox is provided
            let code = if let Some(epsg_str) = args.get_one::<String>("epsg") {
                // If an EPSG was provided, parse it
                info!("Parsing EPSG code: {}", epsg_str);
                epsg_str.parse::<u32>()
                    .map_err(|_| TiffError::GenericError(format!("Invalid EPSG code: {}", epsg_str)))?
            } else {
                // Use default value WGS84
                info!("Using default EPSG code: {}", epsg::WGS84);
                epsg::WGS84 as u32
            };

            Some(code)
        } else {
            None
        };
        info!("EPSG code: {:?}", epsg_code);

        // Get colormap options
        let colormap_output = args.get_one::<String>("colormap-output").cloned();
        info!("Colormap output: {:?}", colormap_output);

        let colormap_input = args.get_one::<String>("colormap-input").cloned();
        info!("Colormap input: {:?}", colormap_input);

        // Get array extraction options
        let array_mode = args.get_flag("extract-array");
        info!("Array extraction mode: {}", array_mode);

        let array_format = args.get_one::<String>("array-format")
            .cloned()
            .unwrap_or_else(|| "csv".to_string());
        info!("Array format: {}", array_format);

        Ok(ExtractCommand {
            input_file,
            output_file,
            bbox_str,
            epsg_code,
            colormap_output,
            colormap_input,
            array_mode,
            array_format,
            logger,
        })
    }

    /// Determine the extraction region based on bounding box
    fn determine_region(&self) -> TiffResult<Option<Region>> {
        info!("Determining extraction region");

        let Some(bbox_str) = &self.bbox_str else {
            info!("No bounding box specified, will use full image");
            return Ok(None); // No bounding box specified, use full image
        };

        info!("Using bounding box: {}", bbox_str);

        // Parse the bounding box
        info!("Parsing bounding box");
        let bbox = image_extraction_utils::parse_bbox(bbox_str)?;
        info!("Parsed bounding box: min_x={}, min_y={}, max_x={}, max_y={}",
              bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y);

        // Load the TIFF file
        info!("Loading TIFF file to determine region");
        let mut reader = TiffReader::new(self.logger);
        let tiff = reader.load(&self.input_file)?;

        // Determine extraction region based on the bounding box
        info!("Converting bounding box to pixel region");
        let region = image_extraction_utils::determine_extraction_region(
            bbox, &tiff, &reader, &self.input_file, self.logger)?;

        info!("Determined extraction region: x={}, y={}, width={}, height={}",
              region.x, region.y, region.width, region.height);

        Ok(Some(region))
    }

    /// Extract colormap from input file if requested
    fn handle_colormap_extraction(&self) -> TiffResult<()> {
        info!("Checking if colormap extraction is requested");

        let Some(colormap_path) = &self.colormap_output else {
            info!("No colormap extraction requested");
            return Ok(());
        };

        info!("Extracting colormap from {} to {}", self.input_file, colormap_path);

        match colormap_utils::extract_colormap(&self.input_file, colormap_path, self.logger) {
            Ok(_) => {
                info!("Colormap extraction successful");
                Ok(())
            },
            Err(e) => {
                warn!("Failed to extract colormap: {}", e);
                // Continue with extraction even if colormap extraction fails
                Ok(())
            }
        }
    }

    /// Extract image with colormap application
    fn extract_with_colormap(&self, extractor: &mut ImageExtractor, region: Option<Region>, colormap_path: &str) -> TiffResult<()> {
        info!("Will apply colormap from {} when extracting", colormap_path);

        // First extract the image to memory
        info!("Extracting image to memory for colormap application");
        let image = extractor.extract_image(&self.input_file, region)?;
        info!("Image extracted: {}x{}", image.width(), image.height());

        // Load the colormap
        info!("Loading colormap from {}", colormap_path);
        let colormap = match colormap_utils::load_colormap(colormap_path, self.logger) {
            Ok(cm) => {
                info!("Colormap loaded with {} entries", cm.len());
                cm
            },
            Err(e) => {
                warn!("Failed to read colormap file: {:?}", e);
                warn!("Continuing with extraction without applying colormap");
                extractor.extract_to_file(&self.input_file, &self.output_file, region)?;
                return Ok(());
            }
        };

        info!("Applying colormap with {} entries", colormap.len());

        // Convert to grayscale if not already
        info!("Converting image to grayscale");
        let grayscale = image.to_luma8();

        // Apply colormap to transform image
        info!("Applying colormap to transform image");
        let rgb_image = colormap_utils::apply_colormap_to_image(&grayscale, &colormap);
        info!("Colormap applied, image is now RGB");

        self.save_colorized_image(rgb_image, region)
    }

    /// Save colorized image in appropriate format
    fn save_colorized_image(&self, rgb_image: image::RgbImage, region: Option<Region>) -> TiffResult<()> {
        info!("Saving colorized image to {}", self.output_file);

        // Check output format
        let is_tiff = Path::new(&self.output_file)
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
            .map(|ext| ext == "tif" || ext == "tiff")
            .unwrap_or(false);

        if is_tiff {
            // Save as georeferenced TIFF
            info!("Saving as georeferenced TIFF");
            colormap_utils::save_colorized_tiff(
                rgb_image,
                &self.output_file,
                &self.input_file,
                region,
                self.logger
            )
        } else {
            // For other formats, just save the RGB image
            info!("Saving as standard image format");
            match rgb_image.save(&self.output_file) {
                Ok(_) => {
                    info!("Image saved successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to save colorized image: {}", e);
                    Err(TiffError::GenericError(format!("Failed to save colorized image: {}", e)))
                }
            }
        }
    }

    /// Extract array data from input file
    fn extract_array_data(&self, region: Option<Region>) -> TiffResult<()> {
        info!("Starting array data extraction from {} to {} in {} format",
              self.input_file, self.output_file, self.array_format);

        // Test if output file is writable
        info!("Testing if output file is writable");
        let test_file = std::fs::File::create(&self.output_file);
        match test_file {
            Ok(_) => info!("Output path is writable"),
            Err(e) => {
                error!("Cannot write to output path: {}", e);
                return Err(TiffError::GenericError(format!("Cannot write to output file: {}", e)));
            }
        }

        // Create API instance
        info!("Creating RasterKit API instance");
        let api = match crate::api::RasterKit::new(Some("rasterkit.log")) {
            Ok(api) => {
                info!("API instance created successfully");
                api
            },
            Err(e) => {
                error!("Failed to create API instance: {}", e);
                return Err(e);
            }
        };

        // Extract the array data to file
        info!("Calling extract_to_array API method");
        let result = api.extract_to_array(
            &self.input_file,
            &self.output_file,
            &self.array_format,
            region.map(|r| (r.x, r.y, r.width, r.height))
        );

        // Check result
        match &result {
            Ok(_) => info!("Array extraction completed successfully"),
            Err(e) => error!("Array extraction failed: {}", e),
        }

        result
    }
}

impl<'a> Command for ExtractCommand<'a> {
    fn execute(&self) -> TiffResult<()> {
        info!("Executing extract command with array_mode={}", self.array_mode);

        // Determine region to extract
        info!("Determining extraction region");
        let region = match self.determine_region() {
            Ok(r) => {
                info!("Region determination successful: {:?}", r);
                r
            },
            Err(e) => {
                error!("Failed to determine region: {}", e);
                return Err(e);
            }
        };

        // Handle colormap extraction if requested (for both image and array modes)
        info!("Handling colormap extraction");
        if let Err(e) = self.handle_colormap_extraction() {
            error!("Colormap extraction failed: {}", e);
            return Err(e);
        }

        if self.array_mode {
            // Array extraction mode
            info!("Using array extraction mode");
            self.extract_array_data(region)
        } else {
            // Image extraction mode
            info!("Using image extraction mode");
            info!("Extracting image data from {} to {}", self.input_file, self.output_file);

            // Create image extractor
            info!("Creating image extractor");
            let mut extractor = ImageExtractor::new(self.logger);

            // Extract image with or without colormap
            if let Some(colormap_path) = &self.colormap_input {
                info!("Extracting with colormap from {}", colormap_path);
                self.extract_with_colormap(&mut extractor, region, colormap_path)?;
            } else {
                // Normal extraction without colormap
                info!("Extracting without colormap");
                extractor.extract_to_file(&self.input_file, &self.output_file, region)?;
            }

            info!("Image extraction successful");
            self.logger.log("Image extraction successful")?;

            Ok(())
        }
    }
}