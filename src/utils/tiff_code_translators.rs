//! TIFF code translators
//!
//! This module provides utilities for translating numeric TIFF tag values
//! into human-readable descriptions. These functions are used throughout
//! the codebase for displaying information about TIFF files to users.

use crate::tiff::constants::{compression, sample_format, predictor, photometric, planar_config};

/// Converts a TIFF compression code to its human-readable description
pub fn compression_code_to_name(compression_code: u64) -> &'static str {
    match compression_code {
        code if code == compression::NONE as u64 => "Uncompressed",
        code if code == compression::CCITT_RLE as u64 => "CCITT RLE",
        code if code == compression::CCITT_FAX3 as u64 => "CCITT Group 3 fax",
        code if code == compression::CCITT_FAX4 as u64 => "CCITT Group 4 fax",
        code if code == compression::LZW as u64 => "LZW",
        code if code == compression::JPEG_OLD as u64 => "JPEG (old-style)",
        code if code == compression::JPEG as u64 => "JPEG",
        code if code == compression::DEFLATE as u64 => "Adobe Deflate (zlib)",
        code if code == compression::JBIG_BW as u64 => "JBIG B&W",
        code if code == compression::JBIG_COLOR as u64 => "JBIG Color",
        code if code == compression::ZSTD as u64 => "ZSTD",
        code if code == compression::PACKBITS as u64 => "PackBits",
        _ => "Unknown",
    }
}

/// Converts a TIFF sample format code to its human-readable description
pub fn sample_format_code_to_name(sample_format_code: u64) -> &'static str {
    match sample_format_code {
        code if code == sample_format::UNSIGNED as u64 => "Unsigned integer",
        code if code == sample_format::SIGNED as u64 => "Signed integer",
        code if code == sample_format::IEEEFP as u64 => "IEEE floating point",
        code if code == sample_format::VOID as u64 => "Undefined",
        code if code == sample_format::COMPLEX_INT as u64 => "Complex integer",
        code if code == sample_format::COMPLEX_IEEEFP as u64 => "Complex floating point",
        _ => "Unknown",
    }
}

/// Converts a TIFF predictor code to its human-readable description
pub fn predictor_code_to_name(predictor_code: u64) -> &'static str {
    match predictor_code {
        code if code == predictor::NONE as u64 => "No prediction scheme",
        code if code == predictor::HORIZONTAL_DIFFERENCING as u64 => "Horizontal differencing",
        code if code == predictor::FLOATING_POINT as u64 => "Floating point predictor",
        _ => "Unknown",
    }
}

/// Converts a TIFF photometric interpretation code to its human-readable description
pub fn photometric_code_to_name(photometric_code: u64) -> &'static str {
    match photometric_code {
        code if code == photometric::WHITE_IS_ZERO as u64 => "WhiteIsZero",
        code if code == photometric::BLACK_IS_ZERO as u64 => "BlackIsZero",
        code if code == photometric::RGB as u64 => "RGB",
        code if code == photometric::PALETTE as u64 => "Palette Color",
        code if code == photometric::TRANSPARENCY_MASK as u64 => "Transparency Mask",
        code if code == photometric::CMYK as u64 => "CMYK",
        code if code == photometric::YCBCR as u64 => "YCbCr",
        code if code == photometric::CIELAB as u64 => "CIE L*a*b*",
        _ => "Unknown",
    }
}

/// Converts a TIFF planar configuration code to its human-readable description
pub fn planar_config_code_to_name(planar_config_code: u64) -> &'static str {
    match planar_config_code {
        code if code == planar_config::CHUNKY as u64 => "Chunky (interleaved)",
        code if code == planar_config::PLANAR as u64 => "Planar (separate)",
        _ => "Unknown",
    }
}