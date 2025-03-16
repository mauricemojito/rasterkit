//! Custom error types for TIFF processing

use std::fmt;
use std::io;

/// TIFF-specific error types
#[derive(Debug)]
pub enum TiffError {
    /// I/O error
    IoError(io::Error),
    /// Invalid TIFF header
    InvalidHeader,
    /// Invalid byte order marker
    InvalidByteOrder(u16),
    /// Invalid BigTIFF header
    InvalidBigTIFFHeader,
    /// Unsupported TIFF version
    UnsupportedVersion(u16),
    /// Tag not found
    TagNotFound(u16),
    /// Unsupported field type
    UnsupportedFieldType(u16),
    /// Unsupported compression method
    UnsupportedCompression(u64),
    /// Image dimensions not found
    MissingDimensions,
    /// Generic error with message
    GenericError(String),
}

impl fmt::Display for TiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TiffError::IoError(e) => write!(f, "I/O error: {}", e),
            TiffError::InvalidHeader => write!(f, "Invalid TIFF header"),
            TiffError::InvalidByteOrder(v) => write!(f, "Invalid byte order marker: {:#06x}", v),
            TiffError::InvalidBigTIFFHeader => write!(f, "Invalid BigTIFF header"),
            TiffError::UnsupportedVersion(v) => write!(f, "Unsupported TIFF version: {}", v),
            TiffError::TagNotFound(tag) => write!(f, "Tag not found: {}", tag),
            TiffError::UnsupportedFieldType(ft) => write!(f, "Unsupported field type: {}", ft),
            TiffError::UnsupportedCompression(c) => write!(f, "Unsupported compression method: {}", c),
            TiffError::MissingDimensions => write!(f, "Image dimensions not found"),
            TiffError::GenericError(msg) => write!(f, "TIFF error: {}", msg),
        }
    }
}

impl std::error::Error for TiffError {}

impl From<io::Error> for TiffError {
    fn from(error: io::Error) -> Self {
        TiffError::IoError(error)
    }
}

/// Result type for TIFF operations
pub type TiffResult<T> = Result<T, TiffError>;

impl From<String> for TiffError {
    fn from(msg: String) -> Self {
        TiffError::GenericError(msg)
    }
}