//! TIFF writing utilities
//!
//! Helper functions for writing TIFF files to disk, handling alignment,
//! byte ordering, and other low-level details.

use crate::tiff::errors::TiffResult;
use crate::tiff::ifd::IFDEntry;
use std::collections::HashSet;
use std::io::Write;

/// Align an offset to a 4-byte boundary
///
/// TIFF specification recommends aligning data on word boundaries.
/// This function returns the next 4-byte aligned position given a current offset.
pub fn align_to_4_bytes(offset: u64) -> u64 {
    let remainder = offset % 4;
    if remainder == 0 {
        offset
    } else {
        offset + (4 - remainder)
    }
}

/// Write padding bytes to align to 4-byte boundary
///
/// After writing a block of data, this function adds the necessary
/// padding bytes to ensure the next write will be aligned to a 4-byte boundary.
pub fn write_padding(writer: &mut impl Write, data_len: usize) -> TiffResult<()> {
    let padding = (4 - (data_len % 4)) % 4;
    if padding > 0 {
        writer.write_all(&vec![0u8; padding])?;
    }
    Ok(())
}

/// Get a list of IFD entries sorted by tag number with duplicates removed
///
/// The TIFF specification requires tags to be sorted by ID and ensures
/// each tag appears only once. If multiple entries have the same tag ID,
/// only the last occurrence is kept.
pub fn get_unique_sorted_entries(entries: &[IFDEntry]) -> Vec<IFDEntry> {
    // First sort by tag number
    let mut sorted_entries = entries.to_vec();
    sorted_entries.sort_by_key(|entry| entry.tag);

    // Then ensure uniqueness (keep last occurrence of each tag)
    let mut unique_entries = Vec::new();
    let mut seen_tags = HashSet::new();

    // Process in reverse to keep the last occurrence of each tag
    for entry in sorted_entries.iter().rev() {
        if !seen_tags.contains(&entry.tag) {
            seen_tags.insert(entry.tag);
            unique_entries.push(entry.clone());
        }
    }

    // Reverse back to ascending order by tag ID
    unique_entries.reverse();
    unique_entries
}

/// Calculate padding required to align to 4-byte boundary
pub fn calculate_padding(data_len: usize) -> usize {
    (4 - (data_len % 4)) % 4
}