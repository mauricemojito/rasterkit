//! TIFF file construction utilities
//!
//! This module provides functionality for constructing TIFF files
//! while preserving metadata and structure.

use std::collections::HashMap;
use log::{info, error};

use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::IFD;
use crate::utils::logger::Logger;
use crate::extractor::Region;

use crate::tiff::builders::basic_tags::BasicTagsBuilder;
use crate::tiff::builders::geo_tags::GeoTagsBuilder;
use crate::tiff::builders::metadata_tags::MetadataBuilder;
use crate::tiff::builders::writer::WriterBuilder;

/// Builder for creating TIFF files
pub struct TiffBuilder<'a> {
    logger: &'a Logger,
    is_big_tiff: bool,
    pub ifds: Vec<IFD>,
    image_data: HashMap<usize, Vec<u8>>,
    external_data: HashMap<(usize, u16), Vec<u8>>,
}

impl<'a> TiffBuilder<'a> {
    /// Create a new TIFF builder
    pub fn new(logger: &'a Logger, is_big_tiff: bool) -> Self {
        info!("Creating new TiffBuilder (is_big_tiff: {})", is_big_tiff);
        TiffBuilder {
            logger,
            is_big_tiff,
            ifds: Vec::new(),
            image_data: HashMap::new(),
            external_data: HashMap::new(),
        }
    }

    /// Add an IFD to the TIFF
    pub fn add_ifd(&mut self, ifd: IFD) -> usize {
        let ifd_index = self.ifds.len();
        info!("Adding IFD #{} to TiffBuilder", ifd_index);
        self.ifds.push(ifd);
        ifd_index
    }

    /// Set image data for an IFD
    pub fn set_image_data(&mut self, ifd_index: usize, data: Vec<u8>) {
        info!("Setting image data for IFD #{}: {} bytes", ifd_index, data.len());

        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        self.image_data.insert(ifd_index, data);
    }

    /// Set external data for a tag
    pub fn set_external_data(&mut self, ifd_index: usize, tag: u16, data: Vec<u8>) {
        info!("Setting external data for IFD #{}, tag {}: {} bytes", ifd_index, tag, data.len());

        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        self.external_data.insert((ifd_index, tag), data);
    }

    /// Add bits per sample values for an IFD
    pub fn add_bits_per_sample(&mut self, ifd_index: usize, bits_per_channel: &[u16]) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        BasicTagsBuilder::add_bits_per_sample(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            bits_per_channel
        );
    }

    /// Set up a single strip for image data
    pub fn setup_single_strip(&mut self, ifd_index: usize, strip_data: Vec<u8>) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        BasicTagsBuilder::setup_single_strip(
            &mut self.ifds[ifd_index],
            &mut self.image_data,
            ifd_index,
            strip_data
        );
    }

    /// Add common tags for a basic RGB image
    pub fn add_basic_rgb_tags(&mut self, ifd_index: usize, width: u32, height: u32) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        BasicTagsBuilder::add_basic_rgb_tags(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            width,
            height
        );
    }

    /// Add common tags for a grayscale image
    pub fn add_basic_gray_tags(&mut self, ifd_index: usize, width: u32, height: u32, bits_per_sample: u16) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        BasicTagsBuilder::add_basic_gray_tags(
            &mut self.ifds[ifd_index],
            width,
            height,
            bits_per_sample
        );
    }

    /// Add color map for a palette-color image
    pub fn add_color_map(&mut self, ifd_index: usize, color_map: &[u16]) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        BasicTagsBuilder::add_color_map(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            color_map
        );
    }

    /// Copy GeoTIFF tags from source IFD
    pub fn copy_geotiff_tags(&mut self, ifd_index: usize, source_ifd: &IFD, reader: &mut crate::tiff::TiffReader) -> TiffResult<()> {
        if ifd_index >= self.ifds.len() {
            return Err(TiffError::GenericError(format!(
                "Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len())));
        }

        GeoTagsBuilder::copy_geotiff_tags(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            source_ifd,
            reader
        )
    }

    /// Adjust GeoTIFF tags for an extracted region
    pub fn adjust_geotiff_for_region(
        &mut self,
        ifd_index: usize,
        region: &Region,
        pixel_scale: &[f64],
        tiepoint: &[f64]
    ) -> TiffResult<()> {
        if ifd_index >= self.ifds.len() {
            return Err(TiffError::GenericError(format!(
                "Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len())));
        }

        GeoTagsBuilder::adjust_geotiff_for_region(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            region,
            pixel_scale,
            tiepoint
        )
    }

    /// Copy appearance-related tags from source IFD
    pub fn copy_appearance_tags(&mut self, ifd_index: usize, source_ifd: &IFD) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        GeoTagsBuilder::copy_appearance_tags(
            &mut self.ifds[ifd_index],
            source_ifd
        );
    }

    /// Copy tags from source IFD, excluding specified ones
    pub fn copy_tags_from(&mut self, ifd_index: usize, source_ifd: &IFD, exclude_tags: &[u16]) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        GeoTagsBuilder::copy_tags_from(
            &mut self.ifds[ifd_index],
            source_ifd,
            exclude_tags
        );
    }

    /// Add a GDAL NoData tag to an IFD
    pub fn add_nodata_tag(&mut self, ifd_index: usize, nodata_value: &str) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        MetadataBuilder::add_nodata_tag(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            nodata_value
        );
    }

    /// Add or update GDAL metadata tag
    pub fn add_gdal_metadata_tag(&mut self, ifd_index: usize, existing_metadata: Option<&str>, nodata_value: &str) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        MetadataBuilder::add_gdal_metadata_tag(
            &mut self.ifds[ifd_index],
            &mut self.external_data,
            ifd_index,
            existing_metadata,
            nodata_value
        );
    }

    /// Copy statistics tags from source IFD
    pub fn copy_statistics_tags(&mut self, ifd_index: usize, source_ifd: &IFD) {
        if ifd_index >= self.ifds.len() {
            error!("Invalid IFD index {}, only have {} IFDs", ifd_index, self.ifds.len());
            return;
        }

        MetadataBuilder::copy_statistics_tags(
            &mut self.ifds[ifd_index],
            source_ifd
        );
    }

    /// Write the TIFF file to disk
    pub fn write(&self, output_path: &str) -> TiffResult<()> {
        info!("Writing TIFF to {}", output_path);
        self.logger.log(&format!("Writing TIFF to {}", output_path))?;

        WriterBuilder::write(
            self.is_big_tiff,
            &self.ifds,
            &self.image_data,
            &self.external_data,
            output_path
        )
    }
}