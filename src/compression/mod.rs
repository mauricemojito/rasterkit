//! Compression handling for TIFF files
//!
//! This module implements strategies for handling different compression methods.

mod handler;
mod uncompressed;
mod deflate;
mod factory;
mod zstd;
mod converter;

pub use handler::CompressionHandler;
pub use uncompressed::UncompressedHandler;
pub use deflate::AdobeDeflateHandler;
pub use factory::CompressionFactory;
pub use zstd::ZstdHandler;
pub use converter::CompressionConverter;