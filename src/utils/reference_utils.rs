//! Georeferencing utilities
//!
//! Utilities for working with georeferenced TIFF files, including
//! preserving georeferencing when modifying TIFF files.

use crate::tiff::errors::TiffResult;
use crate::utils::logger::Logger;
use crate::extractor::Region;
use crate::tiff::TiffReader;
use crate::tiff::geo_key_parser::GeoKeyParser;
use crate::tiff::TiffBuilder;

/// Add georeferencing information to a TIFF builder
///
/// # Arguments
/// * `builder` - The TIFF builder to modify
/// * `ifd_index` - Index of the IFD to add georeference to
/// * `extract_region` - The region that was extracted
/// * `input_path` - Path to the input file
/// * `logger` - Logger for recording operations
///
/// # Returns
/// Result indicating success or an error
pub fn add_georeferencing_to_builder(
    builder: &mut TiffBuilder,
    ifd_index: usize,
    extract_region: &Region,
    input_path: &str,
    logger: &Logger
) -> TiffResult<()> {
    // Load the original TIFF file to get GeoTIFF information
    let mut tiff_reader = TiffReader::new(logger);
    let tiff = tiff_reader.load(input_path)?;

    if tiff.ifds.is_empty() {
        return Ok(());
    }

    let source_ifd = &tiff.ifds[0];

    // Get byte order handler
    let byte_order_handler = match tiff_reader.get_byte_order_handler() {
        Some(handler) => handler,
        None => return Ok(()),
    };

    let file_path = tiff_reader.get_file_path().unwrap_or(input_path);

    // Try to read pixel scale and tiepoint
    if let Ok(pixel_scale) = GeoKeyParser::read_model_pixel_scale_values(
        source_ifd, byte_order_handler, file_path) {

        if let Ok(tiepoint) = GeoKeyParser::read_model_tiepoint_values(
            source_ifd, byte_order_handler, file_path) {

            // Adjust geotransform for the extracted region
            builder.adjust_geotiff_for_region(ifd_index, extract_region, &pixel_scale, &tiepoint)?;
        }
    }

    // Copy GeoTIFF keys
    builder.copy_geotiff_tags(ifd_index, source_ifd, &mut tiff_reader)?;

    // Set NoData value
    let nodata_value = crate::utils::tiff_extraction_utils::extract_nodata_value(source_ifd, &tiff_reader);
    builder.add_nodata_tag(ifd_index, &nodata_value);

    Ok(())
}