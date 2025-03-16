//! Compression handler trait definition

use crate::tiff::errors::TiffResult;

/// Strategy trait for handling different compression methods
pub trait CompressionHandler: Send + Sync {
    /// Decompress the data
    fn decompress(&self, data: &[u8]) -> TiffResult<Vec<u8>>;

    /// Compress the data
    fn compress(&self, data: &[u8]) -> TiffResult<Vec<u8>>;

    /// Get the name of this compression method
    fn name(&self) -> &'static str;

    /// Get the compression code
    fn code(&self) -> u64;
}