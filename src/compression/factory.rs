//! Factory for creating compression handlers

use crate::tiff::errors::{TiffError, TiffResult};
use super::handler::CompressionHandler;
use super::uncompressed::UncompressedHandler;
use super::deflate::AdobeDeflateHandler;
use super::zstd::ZstdHandler;

/// Factory for creating compression handlers
pub struct CompressionFactory;

impl CompressionFactory {
    /// Create a compression handler for the given compression code
    pub fn create_handler(compression: u64) -> TiffResult<Box<dyn CompressionHandler>> {
        match compression {
            1 => Ok(Box::new(UncompressedHandler)),
            8 => Ok(Box::new(AdobeDeflateHandler)),
            14 => Ok(Box::new(ZstdHandler::new())),
            _ => Err(TiffError::UnsupportedCompression(compression))
        }
    }

    /// Get a handler by name
    pub fn get_handler_by_name(name: &str) -> TiffResult<Box<dyn CompressionHandler>> {
        match name.to_lowercase().as_str() {
            "uncompressed" | "none" => Ok(Box::new(UncompressedHandler)),
            "deflate" | "zip" | "adobe deflate" => Ok(Box::new(AdobeDeflateHandler)),
            "zstd" => Ok(Box::new(ZstdHandler::new())),
            _ => Err(TiffError::GenericError(format!("Unknown compression type: {}", name)))
        }
    }

    /// Get all available compression handlers
    pub fn get_available_handlers() -> Vec<Box<dyn CompressionHandler>> {
        vec![
            Box::new(UncompressedHandler),
            Box::new(AdobeDeflateHandler),
            Box::new(ZstdHandler::new())
        ]
    }
}