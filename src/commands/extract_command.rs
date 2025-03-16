use clap::ArgMatches;
use log::{debug, info, warn, error};
use std::path::Path;
use image::DynamicImage;
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
use crate::utils::coordinate_utils;
use crate::utils::reprojection_utils;

/// Command for extracting image data from TIFF files
pub struct ExtractCommand<'a> {
    /// Path to the input file
    input_file: String,
    /// Path to the output file
    output_file: String,
    /// Bounding box string for region extraction
    bbox_str: Option<String>,
    /// Coordinate string for point-based extraction
    coordinate_str: Option<String>,
    /// Radius in meters for point-based extraction
    radius: Option<f64>,
    /// Shape for coordinate-based extraction (circle or square)
    shape: String,
    /// CRS code for the bounding box/coordinate
    crs_code: Option<u32>,
    /// Target projection EPSG code for reprojection
    proj_code: Option<u32>,
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

        // Get coordinate and radius if provided
        let coordinate_str = args.get_one::<String>("coordinate").cloned();
        info!("Coordinate: {:?}", coordinate_str);

        let radius = if let Some(radius_str) = args.get_one::<String>("radius") {
            match radius_str.parse::<f64>() {
                Ok(r) => {
                    info!("Radius: {} meters", r);
                    Some(r)
                },
                Err(e) => {
                    return Err(TiffError::GenericError(
                        format!("Invalid radius value: {}", e)));
                }
            }
        } else {
            None
        };

        // Get shape for coordinate-based extraction
        let shape = args.get_one::<String>("shape")
            .cloned()
            .unwrap_or_else(|| "square".to_string());
        info!("Shape: {}", shape);

        // Validate that if radius is specified, coordinate is also specified
        if radius.is_some() && coordinate_str.is_none() {
            return Err(TiffError::GenericError(
                "Radius specified but no coordinate provided".to_string()));
        }

        // Get CRS code if provided
        let crs_code = if let Some(crs_str) = args.get_one::<String>("crs") {
            // If a CRS was provided, parse it
            info!("Parsing CRS code: {}", crs_str);
            match crs_str.parse::<u32>() {
                Ok(code) => {
                    info!("Using CRS code: {}", code);
                    Some(code)
                },
                Err(_) => {
                    return Err(TiffError::GenericError(format!("Invalid CRS code: {}", crs_str)));
                }
            }
        } else if let Some(epsg_str) = args.get_one::<String>("epsg") {
            // For backward compatibility with --epsg
            info!("Using EPSG code from --epsg parameter: {}", epsg_str);
            match epsg_str.parse::<u32>() {
                Ok(code) => {
                    info!("Using coordinate system EPSG:{}", code);
                    Some(code)
                },
                Err(_) => return Err(TiffError::GenericError(format!("Invalid EPSG code: {}", epsg_str)))
            }
        } else {
            // Only default to WGS84 if no CRS/EPSG was explicitly specified
            if coordinate_str.is_some() || bbox_str.is_some() {
                // If we have coordinates but no CRS, default to WGS84
                info!("No CRS specified with coordinates, defaulting to EPSG:4326 (WGS84)");
                Some(4326)
            } else {
                None
            }
        };

        info!("CRS code: {:?}", crs_code);

        // Get target projection code if provided
        let proj_code = if let Some(proj_str) = args.get_one::<String>("proj") {
            info!("Parsing target projection code: {}", proj_str);
            match proj_str.parse::<u32>() {
                Ok(code) => {
                    info!("Using target projection EPSG:{}", code);
                    Some(code)
                },
                Err(_) => {
                    return Err(TiffError::GenericError(format!("Invalid projection code: {}", proj_str)));
                }
            }
        } else {
            None
        };

        info!("Target projection code: {:?}", proj_code);

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
            coordinate_str,
            radius,
            shape,
            crs_code,
            proj_code,
            colormap_output,
            colormap_input,
            array_mode,
            array_format,
            logger,
        })
    }

    /// Determine the effective bounding box based on input parameters
    ///
    /// This method analyzes the command parameters to determine the appropriate
    /// bounding box to use. It handles the following cases:
    /// - Coordinate + radius: Converts to bounding box using coordinate_utils
    /// - Direct bounding box: Uses the provided bbox_str
    /// - No spatial filter: Returns None to extract the entire image
    ///
    /// # Returns
    /// An optional string containing the bounding box coordinates, or None if no spatial filter specified
    fn determine_effective_bbox(&self) -> TiffResult<Option<String>> {
        // If coordinate and radius are specified, convert to bbox
        if let (Some(coord_str), Some(rad)) = (&self.coordinate_str, self.radius) {
            info!("Converting coordinate and radius to bounding box");
            let bbox_str = coordinate_utils::coord_to_bbox(
                coord_str,
                rad,
                &self.shape,
                self.crs_code  // This was using epsg_code - now using crs_code
            )?;
            info!("Calculated bounding box from coordinate: {}", bbox_str);
            Ok(Some(bbox_str))
        }
        // Otherwise use the provided bbox if any
        else if let Some(bbox) = &self.bbox_str {
            info!("Using provided bounding box: {}", bbox);
            Ok(Some(bbox.clone()))
        }
        // No spatial filter specified
        else {
            info!("No bounding box or coordinate specified");
            Ok(None)
        }
    }

    /// Determine extraction region from input parameters
    ///
    /// Converts geographic coordinates (bounding box or coordinate+radius)
    /// to pixel coordinates for extraction. Handles different spatial
    /// filter methods and coordinate reference systems.
    ///
    /// # Returns
    /// An optional Region for extraction, or None to extract the entire image
    fn determine_region(&self) -> TiffResult<Option<Region>> {
        info!("Determining extraction region");

        // Get the effective bounding box (either from bbox_str or calculated from coordinate+radius)
        let effective_bbox = self.determine_effective_bbox()?;

        // If no spatial filter specified, use full image
        let Some(bbox_str) = effective_bbox else {
            info!("No spatial filter specified, will use full image");
            return Ok(None);
        };

        info!("Using bounding box: {}", bbox_str);

        // Parse the bounding box
        info!("Parsing bounding box");
        let mut bbox = image_extraction_utils::parse_bbox(&bbox_str)?;

        // Set the CRS code if we have one
        if let Some(code) = self.crs_code {
            bbox.epsg = Some(code);
        }

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
    ///
    /// If a colormap output path is specified, extracts the colormap
    /// from the input file and saves it.
    ///
    /// # Returns
    /// Result indicating success or an error
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
    ///
    /// Extracts an image and applies a colormap to it, transforming
    /// grayscale values to RGB colors based on the colormap.
    ///
    /// # Arguments
    /// * `extractor` - Image extractor to use
    /// * `region` - Region to extract
    /// * `colormap_path` - Path to the colormap file
    ///
    /// # Returns
    /// Result indicating success or an error
    fn extract_with_colormap(&self, extractor: &mut ImageExtractor, region: Option<Region>, colormap_path: &str) -> TiffResult<()> {
        info!("Will apply colormap from {} when extracting", colormap_path);

        // First extract the image to memory for colormap application
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
                return extractor.extract_to_file(&self.input_file, &self.output_file, region, Some(&self.shape));
            }
        };

        info!("Applying colormap with {} entries", colormap.len());

        // Convert to grayscale if not already
        info!("Converting image to grayscale");
        let grayscale = image.to_luma8();

        // Apply colormap to transform image
        info!("Applying colormap to transform image");
        let rgb_image = colormap_utils::apply_colormap_to_image(&grayscale, &colormap);

        // Save the image, passing shape for proper masking
        colormap_utils::save_colorized_tiff(
            rgb_image,
            &self.output_file,
            &self.input_file,
            region,
            self.logger,
            Some(&self.shape)  // Pass the shape
        )
    }

    /// Save colorized image in appropriate format
    ///
    /// Saves an RGB image to a file, preserving georeferencing if it's a TIFF.
    ///
    /// # Arguments
    /// * `rgb_image` - The RGB image to save
    /// * `region` - Region that was extracted (for georeferencing)
    ///
    /// # Returns
    /// Result indicating success or an error
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
                self.logger,
                Some(&self.shape)
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
    ///
    /// Extracts numeric array data from a TIFF file and saves it in the
    /// specified format (CSV, JSON, or NPY).
    ///
    /// # Arguments
    /// * `region` - Region to extract
    ///
    /// # Returns
    /// Result indicating success or an error
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

    /// Determine region with radius information
    fn determine_region_with_radius(&self, radius_meters: Option<f64>) -> TiffResult<Option<Region>> {
        info!("Determining extraction region with radius information");

        // Get the effective bounding box (either from bbox_str or calculated from coordinate+radius)
        let effective_bbox = self.determine_effective_bbox()?;

        // If no spatial filter specified, use full image
        let Some(bbox_str) = effective_bbox else {
            info!("No spatial filter specified, will use full image");
            return Ok(None);
        };

        info!("Using bounding box: {}", bbox_str);

        // Parse the bounding box
        info!("Parsing bounding box");
        let mut bbox = image_extraction_utils::parse_bbox(&bbox_str)?;

        // Add radius information if available
        if let Some(radius) = radius_meters {
            info!("Using radius of {} meters for fallback handling", radius);
            bbox.radius_meters = Some(radius);
        }

        // Set the CRS code if we have one
        if let Some(code) = self.crs_code {
            bbox.epsg = Some(code);
        }

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
}

impl<'a> Command for ExtractCommand<'a> {
    /// Execute the extract command
    ///
    /// This is the main entry point for the extract command. It determines
    /// the extraction region, handles colormap extraction if requested, and
    /// then performs either image or array extraction.
    ///
    /// # Returns
    /// Result indicating success or an error
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

            // Create an extractor instance
            let mut extractor = ImageExtractor::new(self.logger);

            // Check for reprojection requirement
            if let Some(proj_code) = self.proj_code {
                info!("Reprojection requested to EPSG:{}", proj_code);

                // Handle extraction with or without colormap
                if let Some(colormap_path) = &self.colormap_input {
                    // Extract image data to memory first
                    let image = extractor.extract_image(&self.input_file, region)?;

                    // Apply colormap to the extracted image
                    let grayscale = image.to_luma8();
                    let colormap = colormap_utils::load_colormap(colormap_path, self.logger)?;
                    let rgb_image = colormap_utils::apply_colormap_to_image(&grayscale, &colormap);

                    // Reproject and save image
                    reprojection_utils::reproject_and_save(
                        &DynamicImage::ImageRgb8(rgb_image),
                        &self.input_file,
                        &self.output_file,
                        region,
                        proj_code,
                        self.logger,
                        Some(&self.shape)
                    )
                } else {
                    // Extract, reproject and save without colormap
                    let image = extractor.extract_image(&self.input_file, region)?;

                    reprojection_utils::reproject_and_save(
                        &image,
                        &self.input_file,
                        &self.output_file,
                        region,
                        proj_code,
                        self.logger,
                        Some(&self.shape)
                    )
                }
            } else {
                // No reprojection requested - use standard extraction
                info!("No reprojection requested, using standard extraction");

                // Handle extraction with or without colormap
                if let Some(colormap_path) = &self.colormap_input {
                    // Extract with colormap
                    self.extract_with_colormap(&mut extractor, region, colormap_path)
                } else {
                    // Simple extraction with shape masking
                    extractor.extract_to_file(&self.input_file, &self.output_file, region, Some(&self.shape))
                }
            }
        }
    }
}