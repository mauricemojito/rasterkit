//! Handler for Adobe Deflate compressed data

use std::io::{Read, Write};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use crate::tiff::errors::{TiffError, TiffResult};
use super::handler::CompressionHandler;

/// Adobe Deflate (Zlib) compression handler (compression code 8)
pub struct AdobeDeflateHandler;

impl CompressionHandler for AdobeDeflateHandler {
    fn decompress(&self, data: &[u8]) -> TiffResult<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed_data = Vec::new();
        match decoder.read_to_end(&mut decompressed_data) {
            Ok(_) => Ok(decompressed_data),
            Err(e) => Err(TiffError::IoError(e))
        }
    }

    fn compress(&self, data: &[u8]) -> TiffResult<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        match encoder.write_all(data) {
            Ok(_) => (),
            Err(e) => return Err(TiffError::IoError(e)),
        }

        match encoder.finish() {
            Ok(compressed) => Ok(compressed),
            Err(e) => Err(TiffError::IoError(e))
        }
    }

    fn name(&self) -> &'static str {
        "Adobe Deflate"
    }

    fn code(&self) -> u64 {
        8
    }
}