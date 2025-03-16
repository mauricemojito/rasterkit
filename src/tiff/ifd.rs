//! Image File Directory (IFD) structures and methods
//!
//! This module implements the core TIFF IFD (Image File Directory) structures
//! that store metadata about images in a TIFF file. IFDs are organized as
//! collections of tag entries, with each tag describing an aspect of the image.

use std::collections::HashMap;
use std::fmt;
use crate::tiff::constants::{field_types, tags};
use log::{debug, info, trace};
use crate::utils::tag_utils;

/// Represents an Image File Directory (IFD) in a TIFF file
///
/// An IFD contains metadata about an image, stored as a series of tag entries.
/// TIFF files can contain multiple IFDs, each describing a separate image in
/// a multipage TIFF.
#[derive(Debug, Clone)]
pub struct IFD {
    /// Entries in this IFD
    pub entries: Vec<IFDEntry>,
    /// IFD number (0-based)
    pub number: usize,
    /// Offset to this IFD in the file
    pub offset: u64,
    /// Cached tag values for quick lookup
    tag_map: HashMap<u16, IFDEntry>,
}

/// Represents an entry in an Image File Directory (IFD)
///
/// Each entry describes one aspect of the image (dimensions, color space,
/// compression, etc.) using a tag-value pair. The field_type determines
/// how to interpret the value or offset.
#[derive(Debug, Clone)]
pub struct IFDEntry {
    /// TIFF tag identifier
    pub tag: u16,
    /// Field type
    pub field_type: u16,
    /// Number of values
    pub count: u64,
    /// Value or offset to values
    pub value_offset: u64,
}

impl IFDEntry {
    /// Creates a new IFD entry
    ///
    /// This constructs a tag entry with the specified parameters.
    /// For small values, value_offset contains the actual value.
    /// For larger values, it contains an offset to where the value is stored.
    pub fn new(tag: u16, field_type: u16, count: u64, value_offset: u64) -> Self {
        let tag_name = tag_utils::get_tag_name(tag);
        let field_type_name = tag_utils::get_field_type_name(field_type);

        debug!("Creating new IFD entry: tag={} ({}), type={} ({}), count={}, offset/value={}",
               tag, tag_name, field_type, field_type_name, count, value_offset);

        Self {
            tag,
            field_type,
            count,
            value_offset,
        }
    }

    /// Get the size in bytes for this entry's field type
    ///
    /// Different TIFF field types take up different amounts of space.
    /// This method returns how many bytes a single value of this entry's type requires.
    pub fn get_field_type_size(&self) -> usize {
        match self.field_type {
            field_types::BYTE | field_types::ASCII | field_types::SBYTE | field_types::UNDEFINED => 1,
            field_types::SHORT | field_types::SSHORT => 2,
            field_types::LONG | field_types::SLONG | field_types::FLOAT => 4,
            field_types::RATIONAL | field_types::SRATIONAL | field_types::DOUBLE => 8,
            field_types::LONG8 | field_types::SLONG8 | field_types::IFD8 => 8,
            _ => {
                debug!("Unknown field type: {}, assuming 1 byte", self.field_type);
                1 // Default to 1 byte
            }
        }
    }

    /// Determines if the value is stored inline in value_offset
    /// rather than at the offset location
    ///
    /// TIFF format allows small values to be stored directly in the IFD entry
    /// rather than requiring a separate data area. This method determines
    /// if this entry's value is stored inline or at an external offset.
    pub fn is_value_inline(&self, is_big_tiff: bool) -> bool {
        let total_size = self.get_field_type_size() * self.count as usize;
        let inline_size = if is_big_tiff { 8 } else { 4 };

        let is_inline = total_size <= inline_size;
        let tag_name = tag_utils::get_tag_name(self.tag);

        trace!("Tag {} ({}) value storage: {}bytes, {} inline (max {}bytes)",
              self.tag, tag_name, total_size,
              if is_inline { "is" } else { "not" }, inline_size);

        is_inline
    }

    /// Returns a human-readable description of this entry
    ///
    /// This is useful for debugging and logging purposes.
    pub fn description(&self) -> String {
        let tag_name = tag_utils::get_tag_name(self.tag);
        let field_type_name = tag_utils::get_field_type_name(self.field_type);

        // Special handling for common tags to provide more meaningful output
        let value_display = match self.tag {
            tags::COMPRESSION => format!("{} ({})",
                                         self.value_offset,
                                         tag_utils::get_compression_name(self.value_offset)),

            tags::PHOTOMETRIC_INTERPRETATION => format!("{} ({})",
                                                        self.value_offset,
                                                        tag_utils::get_photometric_name(self.value_offset)),

            _ => self.value_offset.to_string()
        };

        format!("Tag: {} ({}), Type: {} ({}), Count: {}, Value/Offset: {}",
                self.tag, tag_name, self.field_type, field_type_name, self.count, value_display)
    }
}

impl IFD {
    /// Creates a new IFD
    ///
    /// Initializes an empty Image File Directory with the specified
    /// number (index) and file offset.
    pub fn new(number: usize, offset: u64) -> Self {
        info!("Creating new IFD #{} at offset {}", number, offset);

        Self {
            entries: Vec::new(),
            number,
            offset,
            tag_map: HashMap::new(),
        }
    }

    /// Adds an entry to this IFD
    ///
    /// This method adds a tag entry to the IFD and also updates the
    /// lookup cache for fast access by tag number.
    pub fn add_entry(&mut self, entry: IFDEntry) {
        trace!("Adding entry to IFD #{}: {}", self.number, entry.description());

        self.tag_map.insert(entry.tag, entry.clone());
        self.entries.push(entry);
    }

    /// Gets a tag value (value_offset) directly
    ///
    /// This is a convenience method for quickly retrieving the value/offset
    /// field of a tag without having to access the full entry.
    pub fn get_tag_value(&self, tag: u16) -> Option<u64> {
        let value = self.tag_map.get(&tag).map(|entry| entry.value_offset);
        let tag_name = tag_utils::get_tag_name(tag);

        if let Some(val) = value {
            trace!("Found tag {} ({}) in IFD #{}: value/offset={}", tag, tag_name, self.number, val);
        } else {
            trace!("Tag {} ({}) not found in IFD #{}", tag, tag_name, self.number);
        }

        value
    }

    /// Checks if this IFD has a specific tag
    ///
    /// Returns true if the tag exists in this IFD, false otherwise.
    pub fn has_tag(&self, tag: u16) -> bool {
        let has_tag = self.tag_map.contains_key(&tag);
        let tag_name = tag_utils::get_tag_name(tag);

        trace!("Checking if IFD #{} has tag {} ({}): {}",
               self.number, tag, tag_name, has_tag);

        has_tag
    }

    /// Gets an IFD entry by tag
    ///
    /// Returns the full IFD entry for the specified tag, if it exists.
    pub fn get_entry(&self, tag: u16) -> Option<&IFDEntry> {
        let entry = self.tag_map.get(&tag);
        let tag_name = tag_utils::get_tag_name(tag);

        if entry.is_some() {
            trace!("Retrieved entry for tag {} ({}) from IFD #{}", tag, tag_name, self.number);
        }

        entry
    }

    /// Gets the dimensions of the image described by this IFD
    ///
    /// Returns the width and height of the image if both tags are present.
    pub fn get_dimensions(&self) -> Option<(u64, u64)> {
        let width = self.get_tag_value(tags::IMAGE_WIDTH)?;
        let height = self.get_tag_value(tags::IMAGE_LENGTH)?;

        debug!("Image dimensions from IFD #{}: {}x{}", self.number, width, height);

        Some((width, height))
    }

    /// Returns number of samples per pixel (default 1 if not specified)
    ///
    /// This indicates how many color channels the image has:
    /// 1 for grayscale, 3 for RGB, 4 for RGBA, etc.
    pub fn get_samples_per_pixel(&self) -> u64 {
        let samples = self.get_tag_value(tags::SAMPLES_PER_PIXEL).unwrap_or(1);
        debug!("Samples per pixel from IFD #{}: {}", self.number, samples);
        samples
    }

    /// Gets all entries for this IFD
    ///
    /// Returns a reference to the entries vector.
    pub fn get_entries(&self) -> &Vec<IFDEntry> {
        &self.entries
    }

    /// Gets the number of entries in this IFD
    ///
    /// Returns the count of tag entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl fmt::Display for IFD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "IFD #{} (offset: {})", self.number, self.offset)?;
        writeln!(f, "  Number of entries: {}", self.entries.len())?;

        if let Some((width, height)) = self.get_dimensions() {
            writeln!(f, "  Dimensions: {}x{}", width, height)?;
        }

        writeln!(f, "  Samples per pixel: {}", self.get_samples_per_pixel())?;

        // Enhanced tag list with names
        writeln!(f, "  Tags:")?;
        for entry in &self.entries {
            let tag_name = tag_utils::get_tag_name(entry.tag);
            let field_type_name = tag_utils::get_field_type_name(entry.field_type);

            // Special handling for known tags for more meaningful output
            let value_display = match entry.tag {
                tags::COMPRESSION => format!("{} ({})",
                                             entry.value_offset,
                                             tag_utils::get_compression_name(entry.value_offset)),

                tags::PHOTOMETRIC_INTERPRETATION => format!("{} ({})",
                                                            entry.value_offset,
                                                            tag_utils::get_photometric_name(entry.value_offset)),

                _ => entry.value_offset.to_string()
            };

            writeln!(f, "    {} ({}): {} [{}]",
                     entry.tag, tag_name, value_display, field_type_name)?;
        }

        Ok(())
    }
}