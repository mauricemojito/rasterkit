//! TIFF validation utilities
//!
//! This module provides validation functions for TIFF files
//! to ensure data integrity and prevent errors when processing
//! potentially malformed files.

use log::{debug, error, warn};
use std::io::SeekFrom;

use crate::io::seekable::SeekableReader;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::io::byte_order::ByteOrderHandler;
use crate::tiff::constants::header;

/// Validates an IFD offset to ensure it's within reasonable bounds
///
/// # Arguments
/// * `reader` - The seekable reader to use
/// * `offset` - The offset to validate
/// * `file_size` - The file size for validation
///
/// # Returns
/// Ok if the offset is valid, an error otherwise
pub fn validate_ifd_offset(offset: u64, file_size: u64) -> TiffResult<()> {
    if offset >= file_size || offset < 8 {
        return Err(TiffError::GenericError(format!(
            "Invalid IFD offset: {} (file size: {})",
            offset, file_size
        )));
    }

    Ok(())
}

/// Gets the file size for validation purposes
///
/// # Arguments
/// * `reader` - The seekable reader to use
///
/// # Returns
/// The file size or u64::MAX if it couldn't be determined
pub fn get_file_size(reader: &mut dyn SeekableReader) -> TiffResult<u64> {
    let current_position = reader.seek(SeekFrom::Current(0))?;
    let file_size = match reader.seek(SeekFrom::End(0)) {
        Ok(size) => {
            // Reset position after getting size
            reader.seek(SeekFrom::Start(current_position))?;
            size
        },
        Err(e) => {
            warn!("Could not determine file size: {}", e);
            // Reset position and return MAX as fallback
            reader.seek(SeekFrom::Start(current_position))?;
            u64::MAX
        }
    };

    Ok(file_size)
}

/// Validates the BigTIFF header
///
/// BigTIFF has specific header requirements beyond the standard TIFF.
/// This method verifies that those requirements are met.
///
/// # Arguments
/// * `reader` - The seekable reader to use
/// * `byte_order_handler` - Handler for the file's byte order
pub fn validate_bigtiff_header(
    reader: &mut dyn SeekableReader,
    byte_order_handler: &Box<dyn ByteOrderHandler>
) -> TiffResult<()> {
    // In BigTIFF, after the version number (43) comes:
    // - Offset size (should be 8)
    // - Reserved value (should be 0)
    let offset_size = byte_order_handler.read_u16(reader)?;
    let zeros = byte_order_handler.read_u16(reader)?;

    debug!("BigTIFF offset size: {}", offset_size);
    debug!("BigTIFF zeros: {}", zeros);

    if offset_size != header::BIGTIFF_OFFSET_SIZE || zeros != 0 {
        error!("Invalid BigTIFF header: offset_size={}, zeros={}", offset_size, zeros);
        return Err(TiffError::InvalidBigTIFFHeader);
    }

    Ok(())
}

/// Validates a numeric range to ensure it's within bounds
///
/// # Arguments
/// * `value` - The value to validate
/// * `min` - The minimum valid value (inclusive)
/// * `max` - The maximum valid value (inclusive)
/// * `name` - Name of the value for error messages
///
/// # Returns
/// Ok if the value is valid, an error otherwise
pub fn validate_range<T>(value: T, min: T, max: T, name: &str) -> TiffResult<()>
where
    T: PartialOrd + std::fmt::Display,
{
    if value < min || value > max {
        return Err(TiffError::GenericError(format!(
            "Invalid {}: {} (must be between {} and {})",
            name, value, min, max
        )));
    }

    Ok(())
}