//! TIFF byte order utilities
//!
//! Utilities for detecting and handling byte order (endianness) in TIFF files.

use log::debug;
use crate::io::seekable::SeekableReader;
use crate::io::byte_order::{ByteOrder, ByteOrderHandler};
use crate::tiff::errors::{TiffError, TiffResult};

/// Detects and returns the byte order for a TIFF file
pub fn detect_byte_order(reader: &mut dyn SeekableReader) -> TiffResult<Box<dyn ByteOrderHandler>> {
    let byte_order = ByteOrder::detect(reader)?;
    debug!("Detected byte order: {}", byte_order.name());

    Ok(byte_order.create_handler())
}

/// Gets an unwrapped byte order handler or returns an error
pub fn get_handler_unwrapped(handler: &Option<Box<dyn ByteOrderHandler>>) -> TiffResult<&Box<dyn ByteOrderHandler>> {
    handler.as_ref()
        .ok_or_else(|| TiffError::GenericError("Byte order not yet determined".to_string()))
}
