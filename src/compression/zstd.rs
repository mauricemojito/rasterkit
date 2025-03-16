//! Handler for ZSTD compressed data

use crate::tiff::errors::{TiffError, TiffResult};
use super::handler::CompressionHandler;
use log::{debug, warn};

/// ZSTD compression handler (compression code 14)
pub struct ZstdHandler {
    /// Compression level (1-22, default 3)
    compression_level: i32,
}

impl ZstdHandler {
    /// Create a new ZSTD handler with default compression level
    pub fn new() -> Self {
        ZstdHandler {
            compression_level: 3
        }
    }

    /// Create a new ZSTD handler with specified compression level
    pub fn with_level(level: i32) -> Self {
        let level = level.clamp(1, 22);
        ZstdHandler {
            compression_level: level
        }
    }
}

impl Default for ZstdHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionHandler for ZstdHandler {
    fn decompress(&self, data: &[u8]) -> TiffResult<Vec<u8>> {
        debug!("ZSTD decompressing {} bytes", data.len());
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match zstd::decode_all(data) {
            Ok(decompressed_data) => {
                debug!("ZSTD decompressed to {} bytes", decompressed_data.len());
                Ok(decompressed_data)
            },
            Err(e) => {
                warn!("ZSTD decompression error: {}", e);
                Err(TiffError::GenericError(format!("ZSTD decompression error: {}", e)))
            }
        }
    }

    fn compress(&self, data: &[u8]) -> TiffResult<Vec<u8>> {
        debug!("ZSTD compressing {} bytes with level {}", data.len(), self.compression_level);
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match zstd::encode_all(data, self.compression_level) {
            Ok(compressed) => {
                debug!("ZSTD compressed to {} bytes", compressed.len());
                Ok(compressed)
            },
            Err(e) => {
                warn!("ZSTD compression error: {}", e);
                Err(TiffError::GenericError(format!("ZSTD compression error: {}", e)))
            }
        }
    }

    fn name(&self) -> &'static str {
        "ZSTD"
    }

    fn code(&self) -> u64 {
        14
    }
}