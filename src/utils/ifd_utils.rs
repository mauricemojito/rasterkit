//! IFD utilities
//!
//! Utilities for working with Image File Directories (IFDs) in TIFF files.

use log::debug;
use crate::io::seekable::SeekableReader;
use crate::io::byte_order::ByteOrderHandler;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::IFD;

/// Reads the first IFD offset from a TIFF file header
///
/// # Arguments
/// * `reader` - The seekable reader to use
/// * `is_big_tiff` - Whether the file is in BigTIFF format
/// * `byte_order_handler` - Handler for the file's byte order
///
/// # Returns
/// The offset to the first IFD
pub fn read_first_ifd_offset(
    reader: &mut dyn SeekableReader,
    is_big_tiff: bool,
    byte_order_handler: &Box<dyn ByteOrderHandler>
) -> TiffResult<u64> {
    if is_big_tiff {
        debug!("Reading BigTIFF first IFD offset");
        byte_order_handler.read_u64(reader).map_err(TiffError::IoError)
    } else {
        debug!("Reading standard TIFF first IFD offset");
        byte_order_handler.read_u32(reader)
            .map(|v| v as u64)
            .map_err(TiffError::IoError)
    }
}

/// Reads the next IFD offset
///
/// # Arguments
/// * `reader` - The seekable reader to use
/// * `is_big_tiff` - Whether the file is in BigTIFF format
/// * `byte_order_handler` - Handler for the file's byte order
///
/// # Returns
/// The offset to the next IFD, or 0 if there are no more IFDs
pub fn read_next_ifd_offset(
    reader: &mut dyn SeekableReader,
    is_big_tiff: bool,
    byte_order_handler: &Box<dyn ByteOrderHandler>
) -> TiffResult<u64> {
    if is_big_tiff {
        byte_order_handler.read_u64(reader).map_err(TiffError::IoError)
    } else {
        byte_order_handler.read_u32(reader)
            .map(|v| v as u64)
            .map_err(TiffError::IoError)
    }
}

/// Calculates the size of an IFD in bytes
///
/// Used to determine where the next IFD offset is located
///
/// # Arguments
/// * `ifd` - The IFD to calculate the size for
/// * `is_big_tiff` - Whether the file is in BigTIFF format
///
/// # Returns
/// The size of the IFD in bytes
pub fn calculate_ifd_size(ifd: &IFD, is_big_tiff: bool) -> u64 {
    if is_big_tiff {
        // 8 (entry count) + 20 (each entry) + 8 (next IFD offset)
        8 + (20 * ifd.entries.len() as u64) + 8
    } else {
        // 2 (entry count) + 12 (each entry) + 4 (next IFD offset)
        2 + (12 * ifd.entries.len() as u64) + 4
    }
}