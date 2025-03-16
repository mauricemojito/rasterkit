//! TIFF utility functions
//!
//! Common operations for working with TIFF files that are used
//! across different modules. This module provides utilities for
//! tag manipulation, data type handling, and other TIFF-specific
//! operations that are needed in multiple parts of the codebase.

use crate::tiff::ifd::{IFD, IFDEntry};
use crate::tiff::constants::field_types;
use log::trace;
use std::collections::HashMap;

/// Determine how much space a particular TIFF field type needs in bytes
///
/// TIFF files use numeric codes to identify data types. This function
/// translates those type codes into the number of bytes each value requires.
/// This is essential for calculating data offsets and buffer sizes when
/// reading or writing TIFF files.
pub fn get_field_type_size(field_type: u16) -> usize {
    match field_type {
        field_types::BYTE | field_types::ASCII | field_types::SBYTE | field_types::UNDEFINED => 1,
        field_types::SHORT | field_types::SSHORT => 2,
        field_types::LONG | field_types::SLONG | field_types::FLOAT => 4,
        field_types::RATIONAL | field_types::SRATIONAL | field_types::DOUBLE => 8,
        field_types::LONG8 | field_types::SLONG8 | field_types::IFD8 => 8,
        _ => 1,  // Default to 1 byte
    }
}

/// Update an IFD tag, replacing it if it already exists
///
/// This helper function simplifies the common pattern of removing an existing
/// tag and adding a new one with the same ID. This is frequently needed when
/// modifying existing TIFF files or copying data between IFDs.
pub fn update_ifd_tag(ifd: &mut IFD, tag: u16, entry: IFDEntry) {
    // Remove any existing entry with this tag
    ifd.entries.retain(|e| e.tag != tag);
    // Add the new entry
    ifd.add_entry(entry);
}

/// Create and store external tag data
///
/// In TIFF files, tag data that's too large to fit in the IFD entry itself
/// is stored externally in the file. This function handles the pattern of
/// creating a tag entry and associating it with external data that will be
/// written elsewhere in the file.
///
/// # Parameters
/// * `ifd` - The IFD where the tag will be added
/// * `external_data` - A map that stores external data by (IFD index, tag ID)
/// * `ifd_index` - The index of the current IFD
/// * `tag` - The tag ID
/// * `field_type` - The data type code
/// * `count` - The number of values
/// * `data` - The actual tag data as a byte vector
pub fn create_external_tag(
    ifd: &mut IFD,
    external_data: &mut HashMap<(usize, u16), Vec<u8>>,
    ifd_index: usize,
    tag: u16,
    field_type: u16,
    count: u64,
    data: Vec<u8>
) {
    update_ifd_tag(ifd, tag, IFDEntry::new(tag, field_type, count, 0));
    external_data.insert((ifd_index, tag), data);
}

/// Copy specific tags from source IFD to destination IFD
///
/// Copies the specified list of tags from the source IFD to the destination IFD,
/// handling duplicate resolution by replacing any existing tags.
///
/// # Parameters
/// * `dest_ifd` - The destination IFD where tags will be copied to
/// * `source_ifd` - The source IFD to copy tags from
/// * `tags` - A slice of tag IDs to be copied
pub fn copy_tags(
    dest_ifd: &mut IFD,
    source_ifd: &IFD,
    tags: &[u16]
) {
    for &tag in tags {
        if let Some(entry) = source_ifd.get_entry(tag) {
            trace!("Copying tag {} from source IFD to destination", tag);

            // Remove existing tag if present to avoid duplicates
            let existing_idx = dest_ifd.entries.iter().position(|e| e.tag == tag);
            if let Some(idx) = existing_idx {
                dest_ifd.entries.remove(idx);
            }

            // Add the cloned entry
            dest_ifd.add_entry(entry.clone());
        }
    }
}

/// Copy all tags from source IFD to destination IFD, except those specified
///
/// Copies all tags from the source IFD to the destination IFD, except for
/// the tags in the exclude list. Handles duplicate resolution by replacing
/// any existing tags.
///
/// # Parameters
/// * `dest_ifd` - The destination IFD where tags will be copied to
/// * `source_ifd` - The source IFD to copy tags from
/// * `exclude_tags` - A slice of tag IDs to be excluded from copying
pub fn copy_tags_except(
    dest_ifd: &mut IFD,
    source_ifd: &IFD,
    exclude_tags: &[u16]
) {
    // Loop through all entries in the source IFD
    for entry in &source_ifd.entries {
        // Skip any tags that are in our exclusion list
        if !exclude_tags.contains(&entry.tag) {
            trace!("Copying tag {} from source IFD to destination", entry.tag);

            // Check if this tag already exists in the destination
            let existing_idx = dest_ifd.entries.iter().position(|e| e.tag == entry.tag);
            if let Some(idx) = existing_idx {
                // Remove it so we don't end up with duplicates
                dest_ifd.entries.remove(idx);
            }

            // Add the copied entry
            dest_ifd.add_entry(entry.clone());
        }
    }
}