//! Handler for uncompressed data

use crate::tiff::errors::TiffResult;
use super::handler::CompressionHandler;

/// Uncompressed data handler (compression code 1)
pub struct UncompressedHandler;

impl CompressionHandler for UncompressedHandler {
    fn decompress(&self, data: &[u8]) -> TiffResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn compress(&self, data: &[u8]) -> TiffResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn name(&self) -> &'static str {
        "Uncompressed"
    }

    fn code(&self) -> u64 {
        1
    }
}