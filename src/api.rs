use std::path::Path;
use image::DynamicImage;
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
        // Create a TIFF reader and load the file directly
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
    /// This method provides several ways to specify the region to extract:
    /// - Pixel region with (x, y, width, height)
    /// - Geographic bounding box with "minx,miny,maxx,maxy"
    /// - Geographic coordinate and radius with (x,y) + radius in meters
    ///
    /// The coordinate and radius approach allows extracting circular or square regions
    /// around a specific point of interest.
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `output_path` - Path where to save the extracted image
    /// * `region` - Optional pixel region to extract (x, y, width, height)
    /// * `bbox` - Optional geographic bounding box as "minx,miny,maxx,maxy"
    /// * `coordinate` - Optional geographic coordinate as "x,y"
    /// * `radius` - Optional radius in meters around the coordinate
    /// * `shape` - Optional shape for coordinate-based extraction ("circle" or "square")
    /// * `crs` - Optional CRS code for the bounding box/coordinate coordinates
    /// * `colormap_path` - Optional path to a colormap file to apply
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn extract(&self,
                   input_path: &str,
                   output_path: &str,
                   region: Option<(u32, u32, u32, u32)>,
                   bbox: Option<&str>,
                   coordinate: Option<&str>,
                   radius: Option<f64>,
                   shape: Option<&str>,
                   crs: Option<u32>,
                   colormap_path: Option<&str>) -> TiffResult<()> {

        // Handle coordinate + radius extraction by converting to a bounding box
        let effective_bbox = if let (Some(coord_str), Some(rad)) = (coordinate, radius) {
            let shape_type = shape.unwrap_or("square");
            info!("Using coordinate-based extraction with {} meters radius (shape: {})",
              rad, shape_type);

            match crate::utils::coordinate_utils::coord_to_bbox(coord_str, rad, shape_type, crs) {
                Ok(bbox_str) => {
                    info!("Converted coordinate to bounding box: {}", bbox_str);
                    Some(bbox_str)
                },
                Err(e) => return Err(e),
            }
        } else {
            bbox.map(|s| {
                info!("Using bounding box extraction: {}", s);
                s.to_string()
            })
        };

        // If a colormap is specified, handle with colormap extraction
        if let Some(cmap_path) = colormap_path {
            info!("Colormap specified, using colormap extraction with '{}'", cmap_path);

            // First determine the extraction region
            let extraction_region = self.determine_extraction_region(input_path, region, effective_bbox.as_deref(), crs)?;

            // Convert the Region to the tuple format expected by extract_with_colormap
            let region_tuple = extraction_region.map(|r| (r.x, r.y, r.width, r.height));

            // Use the extract_with_colormap function
            return self.extract_with_colormap(input_path, output_path, cmap_path, region_tuple, shape);
        }

        // Regular extraction without colormap
        let mut extractor = ImageExtractor::new(&self.logger);

        // Determine the extraction region
        let extraction_region = self.determine_extraction_region(input_path, region, effective_bbox.as_deref(), crs)?;

        // Perform the extraction
        extractor.extract_to_file(input_path, output_path, extraction_region, shape)
    }

    /// Helper method to determine extraction region from parameters
    ///
    /// Analyzes the provided extraction parameters and determines the
    /// appropriate region to extract. Handles pixel coordinates, geographic
    /// bounding boxes, and geographic coordinates with radius.
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `region` - Optional pixel region to extract (x, y, width, height)
    /// * `bbox` - Optional geographic bounding box as "minx,miny,maxx,maxy"
    /// * `crs` - Optional CRS code for the bounding box coordinates
    ///
    /// # Returns
    /// An optional Region for extraction, or None to extract the entire image
    fn determine_extraction_region(&self,
                                   input_path: &str,
                                   region: Option<(u32, u32, u32, u32)>,
                                   bbox: Option<&str>,
                                   crs: Option<u32>) -> TiffResult<Option<Region>> {
        if let Some((x, y, width, height)) = region {
            info!("Using pixel region: x={}, y={}, width={}, height={}", x, y, width, height);
            Ok(Some(Region::new(x, y, width, height)))
        } else if let Some(bbox_str) = bbox {
            info!("Converting bounding box '{}' to pixel region", bbox_str);

            // Parse bounding box and convert to pixel coordinates
            let mut bbox = BoundingBox::from_string(bbox_str)
                .map_err(|e| crate::tiff::errors::TiffError::GenericError(e))?;

            // Set CRS code if provided
            if let Some(code) = crs {
                info!("Using CRS code {} for coordinate transformation", code);
                bbox.epsg = Some(code);
            }

            // Create a reader to access the TIFF file
            let mut reader = crate::tiff::TiffReader::new(&self.logger);
            let tiff = reader.load(input_path)?;

            // Determine the extraction region from the bounding box
            let region = crate::utils::image_extraction_utils::determine_extraction_region(
                bbox, &tiff, &reader, input_path, &self.logger)?;

            Ok(Some(region))
        } else {
            info!("No region specified, extracting entire image");
            Ok(None) // Extract the entire image
        }
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
    /// * `shape` - Optional shape for extraction ("circle" or "square")
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn extract_with_colormap(&self,
                                 input_path: &str,
                                 output_path: &str,
                                 colormap_path: &str,
                                 region: Option<(u32, u32, u32, u32)>,
                                 shape: Option<&str>) -> TiffResult<()> {

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
            &self.logger,
            shape
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
        let mut extractor = ImageExtractor::new_array_extractor(&self.logger);

        // Convert region format if provided
        let extraction_region = region.map(|(x, y, width, height)| Region::new(x, y, width, height));

        // Extract array data
        extractor.extract_array_data(input_path, extraction_region)
    }

    /// Extract an image from a TIFF file to memory
    ///
    /// This method provides the same region specification options as `extract`,
    /// but returns the image in memory instead of writing it to a file.
    ///
    /// # Arguments
    /// * `input_path` - Path to the input TIFF file
    /// * `region` - Optional pixel region to extract (x, y, width, height)
    /// * `bbox` - Optional geographic bounding box as "minx,miny,maxx,maxy"
    /// * `coordinate` - Optional geographic coordinate as "x,y"
    /// * `radius` - Optional radius in meters around the coordinate
    /// * `shape` - Optional shape for coordinate-based extraction ("circle" or "square")
    /// * `crs` - Optional CRS code for the bounding box/coordinate coordinates
    /// * `colormap_path` - Optional path to a colormap file to apply
    ///
    /// # Returns
    /// Result containing the extracted image or an error
    pub fn extract_to_buffer(&self,
                             input_path: &str,
                             region: Option<(u32, u32, u32, u32)>,
                             bbox: Option<&str>,
                             coordinate: Option<&str>,
                             radius: Option<f64>,
                             shape: Option<&str>,
                             crs: Option<u32>,
                             colormap_path: Option<&str>) -> TiffResult<DynamicImage> {

        // Handle coordinate + radius extraction by converting to a bounding box
        let effective_bbox = if let (Some(coord_str), Some(rad)) = (coordinate, radius) {
            let shape_type = shape.unwrap_or("square");
            info!("Using coordinate-based extraction with {} meters radius (shape: {})",
            rad, shape_type);

            match crate::utils::coordinate_utils::coord_to_bbox(coord_str, rad, shape_type, crs) {
                Ok(bbox_str) => {
                    info!("Converted coordinate to bounding box: {}", bbox_str);
                    Some(bbox_str)
                },
                Err(e) => return Err(e),
            }
        } else {
            bbox.map(|s| {
                info!("Using bounding box extraction: {}", s);
                s.to_string()
            })
        };

        // Determine the extraction region
        let extraction_region = self.determine_extraction_region(input_path, region, effective_bbox.as_deref(), crs)?;

        // Create an extractor instance
        let mut extractor = ImageExtractor::new(&self.logger);

        // If a colormap is specified, handle with colormap extraction
        if let Some(cmap_path) = colormap_path {
            info!("Colormap specified, using colormap extraction with '{}'", cmap_path);

            // Extract image data to memory
            let image = extractor.extract_image(input_path, extraction_region)?;

            // Apply colormap to the extracted image
            let grayscale = image.to_luma8();
            let colormap = crate::utils::colormap_utils::load_colormap(cmap_path, &self.logger)?;
            let rgb_image = crate::utils::colormap_utils::apply_colormap_to_image(&grayscale, &colormap);

            // Apply shape mask if needed
            if let Some(shape_str) = shape {
                if shape_str.to_lowercase() == "circle" {
                    return Ok(crate::utils::mask_utils::apply_shape_mask(&DynamicImage::ImageRgb8(rgb_image), shape_str));
                }
            }

            return Ok(DynamicImage::ImageRgb8(rgb_image));
        }

        // Extract the image
        let mut image = extractor.extract_image(input_path, extraction_region)?;

        // Apply shape mask if needed
        if let Some(shape_str) = shape {
            if shape_str.to_lowercase() == "circle" {
                image = crate::utils::mask_utils::apply_shape_mask(&image, shape_str);
            }
        }

        Ok(image)
    }
}