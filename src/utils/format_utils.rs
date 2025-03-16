//! TIFF format utilities
//!
//! Utilities for working with TIFF format specifics like
//! byte order detection and format detection.

use log::debug;
use crate::io::seekable::SeekableReader;
use crate::io::byte_order::{ByteOrder, ByteOrderHandler};
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::constants::header;
use crate::tiff::validation;

/// Detects and returns the byte order for a TIFF file
pub fn detect_byte_order(reader: &mut dyn SeekableReader) -> TiffResult<Box<dyn ByteOrderHandler>> {
    let byte_order = ByteOrder::detect(reader)?;
    debug!("Detected byte order: {}", byte_order.name());

    Ok(byte_order.create_handler())
}

/// Detects whether a file is TIFF or BigTIFF based on its version number
///
/// # Arguments
/// * `reader` - The seekable reader to use
/// * `byte_order_handler` - Handler for the file's byte order
///
/// # Returns
/// A tuple with (is_big_tiff, version_number)
pub fn detect_tiff_format(
    reader: &mut dyn SeekableReader,
    byte_order_handler: &Box<dyn ByteOrderHandler>
) -> TiffResult<(bool, u16)> {
    let version = byte_order_handler.read_u16(reader)?;
    debug!("TIFF version: {}", version);

    let is_big_tiff = match version {
        header::BIG_TIFF_VERSION => {
            debug!("Detected BigTIFF format");
            validation::validate_bigtiff_header(reader, byte_order_handler)?;
            true
        },
        header::TIFF_VERSION => {
            debug!("Detected standard TIFF format");
            false
        },
        _ => return Err(TiffError::UnsupportedVersion(version)),
    };

    Ok((is_big_tiff, version))
}