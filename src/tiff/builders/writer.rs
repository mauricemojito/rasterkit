//! TIFF writing strategies
//!
//! This module handles the complex task of writing TIFF files to disk.
//! Writing a valid TIFF requires careful management of offsets, ordering,
//! and alignment to ensure the file can be read by other software.

use crate::tiff::ifd::IFD;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::constants::{header, tags};
use crate::utils::write_utils;
use log::info;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};

/// Handles writing TIFF files to disk
pub struct WriterBuilder;

impl WriterBuilder {
    /// Write a complete TIFF file to disk
    ///
    /// This is the main entry point for TIFF file creation. It handles the complex
    /// process of calculating offsets, writing headers, and organizing the
    /// data in the proper order according to the TIFF specification.
    pub fn write(
        is_big_tiff: bool,
        ifds: &[IFD],
        image_data: &HashMap<usize, Vec<u8>>,
        external_data: &HashMap<(usize, u16), Vec<u8>>,
        output_path: &str
    ) -> TiffResult<()> {
        info!("Writing TIFF to {}", output_path);

        // Create the output file and buffered writer
        let file = File::create(output_path).map_err(TiffError::from)?;
        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

        // Sort IFDs by tag number as required by TIFF spec
        let sorted_ifds = Self::prepare_sorted_ifds(ifds);

        // Write the TIFF header
        Self::write_header(&mut writer, is_big_tiff)?;

        // Calculate all offsets for IFDs and data
        let header_size = if is_big_tiff { 16 } else { 8 };
        let (ifd_offsets, tag_data_offsets) = Self::calculate_offsets(
            &sorted_ifds, external_data, image_data, header_size, is_big_tiff);

        // Write the offset to the first IFD in the header area
        let first_ifd_offset = ifd_offsets.first().copied().unwrap_or(0);
        Self::write_first_ifd_offset(&mut writer, first_ifd_offset, is_big_tiff)?;

        // Write all IFDs
        Self::write_ifds(&mut writer, &sorted_ifds, &ifd_offsets, &tag_data_offsets, is_big_tiff)?;

        // Write all external tag data
        Self::write_external_data(&mut writer, external_data, &tag_data_offsets)?;

        // Write all image data
        Self::write_image_data(&mut writer, image_data, &sorted_ifds, &tag_data_offsets)?;

        // Make sure everything is written to disk
        writer.flush()?;
        Ok(())
    }

    /// Prepare sorted IFDs with unique tags
    fn prepare_sorted_ifds(ifds: &[IFD]) -> Vec<IFD> {
        ifds.iter().map(|ifd| {
            let mut sorted_ifd = ifd.clone();
            sorted_ifd.entries = write_utils::get_unique_sorted_entries(&ifd.entries);
            sorted_ifd
        }).collect()
    }

    /// Calculate offsets for IFDs and external data
    fn calculate_offsets(
        sorted_ifds: &[IFD],
        external_data: &HashMap<(usize, u16), Vec<u8>>,
        image_data: &HashMap<usize, Vec<u8>>,
        header_size: u64,
        is_big_tiff: bool
    ) -> (Vec<u64>, HashMap<(usize, u16), u64>) {
        let mut current_offset = header_size;
        let mut ifd_offsets = Vec::with_capacity(sorted_ifds.len());
        let mut tag_data_offsets = HashMap::new();

        // First pass: calculate IFD offsets
        for ifd in sorted_ifds {
            ifd_offsets.push(current_offset);
            let ifd_size = Self::calculate_ifd_size(ifd, is_big_tiff);
            current_offset += ifd_size;
        }

        // Second pass: calculate tag data offsets
        for ((ifd_index, tag), data) in external_data {
            tag_data_offsets.insert((*ifd_index, *tag), current_offset);
            current_offset += data.len() as u64;
            current_offset = write_utils::align_to_4_bytes(current_offset);
        }

        // Third pass: calculate image data offsets
        for (ifd_index, data) in image_data {
            if let Some(ifd) = sorted_ifds.get(*ifd_index) {
                // Check for strip or tile offsets tags
                let offset_tags = [tags::STRIP_OFFSETS, tags::TILE_OFFSETS];

                for &tag in &offset_tags {
                    if ifd.has_tag(tag) {
                        tag_data_offsets.insert((*ifd_index, tag), current_offset);
                    }
                }
            }

            current_offset += data.len() as u64;
            current_offset = write_utils::align_to_4_bytes(current_offset);
        }

        (ifd_offsets, tag_data_offsets)
    }

    /// Write TIFF header
    ///
    /// The header is the first part of any TIFF file and includes:
    /// - Byte order indicator (II for little-endian or MM for big-endian)
    /// - Version number (42 for standard TIFF, 43 for BigTIFF)
    /// - Offset to the first IFD
    fn write_header(writer: &mut impl Write, is_big_tiff: bool) -> TiffResult<()> {
        // Write byte order marker - we always use little endian (II)
        writer.write_all(&header::LITTLE_ENDIAN_MARKER)?;

        if is_big_tiff {
            // BigTIFF header components
            writer.write_all(&header::BIG_TIFF_VERSION.to_le_bytes())?;
            writer.write_all(&[8u8, 0])?;  // Offset size (8 bytes)
            writer.write_all(&[0u8, 0])?;  // Reserved (always 0)
            writer.write_all(&[0u8; 8])?;  // 8-byte placeholder for first IFD offset
        } else {
            // Standard TIFF header
            writer.write_all(&header::TIFF_VERSION.to_le_bytes())?;
            writer.write_all(&[0u8; 4])?;  // 4-byte placeholder for first IFD offset
        }

        Ok(())
    }

    /// Calculate size of an IFD
    ///
    /// This is important for determining where things will be positioned
    /// in the file. The size depends on whether it's a standard TIFF or BigTIFF,
    /// and how many entries the IFD contains.
    fn calculate_ifd_size(ifd: &IFD, is_big_tiff: bool) -> u64 {
        let entries_count = ifd.entries.len() as u64;

        match is_big_tiff {
            true => {
                // BigTIFF IFD structure:
                // - 8 bytes for entry count
                // - 20 bytes per entry (tag, type, count, value/offset)
                // - 8 bytes for next IFD offset
                8 + (20 * entries_count) + 8
            },
            false => {
                // Standard TIFF IFD structure:
                // - 2 bytes for entry count
                // - 12 bytes per entry (tag, type, count, value/offset)
                // - 4 bytes for next IFD offset
                2 + (12 * entries_count) + 4
            }
        }
    }

    /// Write first IFD offset
    ///
    /// This goes back and updates the placeholder in the header with
    /// the actual offset to the first IFD, now that we know where it will be.
    fn write_first_ifd_offset(writer: &mut (impl Write + Seek), offset: u64, is_big_tiff: bool) -> TiffResult<()> {
        // Position in the header where the offset goes
        let position = if is_big_tiff { 8 } else { 4 };
        writer.seek(SeekFrom::Start(position))?;

        // Write the offset in the appropriate format
        match is_big_tiff {
            true => writer.write_all(&offset.to_le_bytes())?,      // 8 bytes
            false => writer.write_all(&(offset as u32).to_le_bytes())?, // 4 bytes
        }

        Ok(())
    }

    /// Write all IFDs to the file
    fn write_ifds(
        writer: &mut (impl Write + Seek),
        sorted_ifds: &[IFD],
        ifd_offsets: &[u64],
        tag_data_offsets: &HashMap<(usize, u16), u64>,
        is_big_tiff: bool
    ) -> TiffResult<()> {
        for (i, ifd) in sorted_ifds.iter().enumerate() {
            // Calculate offset to next IFD (or 0 if this is the last one)
            let next_ifd_offset = ifd_offsets.get(i + 1).copied().unwrap_or(0);

            // Seek to where this IFD should be written
            writer.seek(SeekFrom::Start(ifd_offsets[i]))?;

            // Write the IFD with its entries
            Self::write_ifd(writer, ifd, next_ifd_offset, tag_data_offsets, i, is_big_tiff)?;
        }

        Ok(())
    }

    /// Write all external tag data
    fn write_external_data(
        writer: &mut (impl Write + Seek),
        external_data: &HashMap<(usize, u16), Vec<u8>>,
        tag_data_offsets: &HashMap<(usize, u16), u64>
    ) -> TiffResult<()> {
        for ((ifd_index, tag), data) in external_data {
            let key = (*ifd_index, *tag);

            // Only process entries that have calculated offsets
            if let Some(offset) = tag_data_offsets.get(&key) {
                writer.seek(SeekFrom::Start(*offset))?;
                writer.write_all(data)?;
                write_utils::write_padding(writer, data.len())?;
            }
        }

        Ok(())
    }

    /// Write all image data
    fn write_image_data(
        writer: &mut (impl Write + Seek),
        image_data: &HashMap<usize, Vec<u8>>,
        sorted_ifds: &[IFD],
        tag_data_offsets: &HashMap<(usize, u16), u64>
    ) -> TiffResult<()> {
        for (ifd_index, data) in image_data {
            // Look for any offset tags that point to this image data
            let possible_tags = [tags::STRIP_OFFSETS, tags::TILE_OFFSETS];

            // Find the first applicable offset tag
            let offset = possible_tags.iter()
                .filter_map(|&tag| tag_data_offsets.get(&(*ifd_index, tag)))
                .next()
                .copied();

            // Write the data if we found a valid offset
            if let Some(file_offset) = offset {
                writer.seek(SeekFrom::Start(file_offset))?;
                writer.write_all(data)?;
                write_utils::write_padding(writer, data.len())?;
            }
        }

        Ok(())
    }

    /// Write an IFD (Image File Directory)
    ///
    /// An IFD contains metadata about the image, stored as a series of tags.
    /// Each IFD entry describes one aspect of the image (dimensions, format, etc.)
    fn write_ifd(
        writer: &mut (impl Write + Seek),
        ifd: &IFD,
        next_offset: u64,
        tag_offsets: &HashMap<(usize, u16), u64>,
        ifd_index: usize,
        is_big_tiff: bool
    ) -> TiffResult<()> {
        // Write the entry count
        match is_big_tiff {
            true => writer.write_all(&(ifd.entries.len() as u64).to_le_bytes())?,
            false => writer.write_all(&(ifd.entries.len() as u16).to_le_bytes())?,
        }

        // Write each entry
        for entry in &ifd.entries {
            // Get the actual offset for this tag's data if it's external
            let value_offset = tag_offsets.get(&(ifd_index, entry.tag))
                .copied()
                .unwrap_or(entry.value_offset);

            // Write the tag ID and field type
            writer.write_all(&entry.tag.to_le_bytes())?;
            writer.write_all(&entry.field_type.to_le_bytes())?;

            // Write the count (number of values)
            match is_big_tiff {
                true => writer.write_all(&entry.count.to_le_bytes())?,
                false => writer.write_all(&(entry.count as u32).to_le_bytes())?,
            }

            // Write the value or offset
            match is_big_tiff {
                true => writer.write_all(&value_offset.to_le_bytes())?,
                false => writer.write_all(&(value_offset as u32).to_le_bytes())?,
            }
        }

        // Write the offset to the next IFD (or 0 if last)
        match is_big_tiff {
            true => writer.write_all(&next_offset.to_le_bytes())?,
            false => writer.write_all(&(next_offset as u32).to_le_bytes())?,
        }

        Ok(())
    }
}