//! TIFF tag utilities
//!
//! Utilities for working with TIFF tags and their values.

use byteorder::ReadBytesExt;

use crate::io::seekable::SeekableReader;
use crate::io::byte_order::ByteOrderHandler;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::IFDEntry;
use crate::tiff::constants::{field_types, tags, compression, photometric};

/// Reads an array of tag values based on the field type
///
/// # Arguments
/// * `reader` - The seekable reader to use
/// * `entry` - The IFD entry with tag information
/// * `handler` - The byte order handler
/// * `values` - The vector to store values in
///
/// # Returns
/// Result indicating success or failure
pub fn read_tag_value_array(
    reader: &mut dyn SeekableReader,
    entry: &IFDEntry,
    handler: &Box<dyn ByteOrderHandler>,
    values: &mut Vec<u64>
) -> TiffResult<()> {
    for _ in 0..entry.count {
        let value = match entry.field_type {
            field_types::BYTE | field_types::SBYTE | field_types::UNDEFINED => reader.read_u8()? as u64,
            field_types::SHORT | field_types::SSHORT => handler.read_u16(reader)? as u64,
            field_types::LONG | field_types::SLONG | field_types::FLOAT => handler.read_u32(reader)? as u64,
            field_types::RATIONAL | field_types::SRATIONAL => {
                let (num, den) = handler.read_rational(reader)?;
                ((num as u64) << 32) | (den as u64)
            },
            field_types::LONG8 | field_types::SLONG8 | field_types::IFD8 => handler.read_u64(reader)?,
            _ => return Err(TiffError::UnsupportedFieldType(entry.field_type)),
        };

        values.push(value);
    }

    Ok(())
}

/// Determines if a tag's value is stored inline or at an offset
///
/// # Arguments
/// * `entry` - The IFD entry to check
/// * `is_big_tiff` - Whether the file is BigTIFF format
///
/// # Returns
/// true if the value is stored inline, false if it's at an offset
pub fn is_value_inline(entry: &IFDEntry, is_big_tiff: bool) -> bool {
    let size_per_value = match entry.field_type {
        field_types::BYTE | field_types::SBYTE | field_types::UNDEFINED => 1,
        field_types::ASCII => 1,
        field_types::SHORT | field_types::SSHORT => 2,
        field_types::LONG | field_types::SLONG | field_types::FLOAT => 4,
        field_types::RATIONAL | field_types::SRATIONAL | field_types::DOUBLE |
        field_types::LONG8 | field_types::SLONG8 | field_types::IFD8 => 8,
        _ => 1, // Default for unknown types
    };

    let total_size = size_per_value * entry.count;

    if is_big_tiff {
        total_size <= 8 // In BigTIFF, 8 bytes are available for inline storage
    } else {
        total_size <= 4 // In standard TIFF, 4 bytes are available for inline storage
    }
}

/// Get the name of a TIFF tag
///
/// Returns a human-readable name for a tag based on its numeric ID.
/// If the tag is not recognized, returns "Unknown".
///
/// # Arguments
/// * `tag` - The tag ID to look up
///
/// # Returns
/// A string representing the tag name
pub fn get_tag_name(tag: u16) -> &'static str {
    match tag {
        // Basic image structure tags
        tags::IMAGE_WIDTH => "ImageWidth",
        tags::IMAGE_LENGTH => "ImageLength",
        tags::BITS_PER_SAMPLE => "BitsPerSample",
        tags::COMPRESSION => "Compression",
        tags::PHOTOMETRIC_INTERPRETATION => "PhotometricInterpretation",
        tags::FILL_ORDER => "FillOrder",
        tags::SAMPLES_PER_PIXEL => "SamplesPerPixel",
        tags::ROWS_PER_STRIP => "RowsPerStrip",
        tags::STRIP_OFFSETS => "StripOffsets",
        tags::STRIP_BYTE_COUNTS => "StripByteCounts",
        tags::MIN_SAMPLE_VALUE => "MinSampleValue",
        tags::MAX_SAMPLE_VALUE => "MaxSampleValue",
        tags::PLANAR_CONFIGURATION => "PlanarConfiguration",
        tags::COLOR_MAP => "ColorMap",
        tags::SAMPLE_FORMAT => "SampleFormat",
        tags::PREDICTOR => "Predictor",

        // Other common tags
        tags::RESOLUTION_UNIT => "ResolutionUnit",
        tags::X_RESOLUTION => "XResolution",
        tags::Y_RESOLUTION => "YResolution",
        tags::TRANSFER_FUNCTION => "TransferFunction",
        tags::SOFTWARE => "Software",
        tags::DATE_TIME => "DateTime",
        tags::ARTIST => "Artist",
        tags::HOST_COMPUTER => "HostComputer",
        tags::COPYRIGHT => "Copyright",

        // Tiling tags
        tags::TILE_OFFSETS => "TileOffsets",
        tags::TILE_BYTE_COUNTS => "TileByteCounts",
        tags::TILE_WIDTH => "TileWidth",
        tags::TILE_LENGTH => "TileLength",

        // Other important tags
        tags::NEW_SUBFILE_TYPE => "NewSubfileType",
        tags::SUBFILE_TYPE => "SubfileType",
        tags::ORIENTATION => "Orientation",

        // GeoTIFF tags
        tags::MODEL_PIXEL_SCALE_TAG => "ModelPixelScale",
        tags::MODEL_TIEPOINT_TAG => "ModelTiepoint",
        tags::GEO_KEY_DIRECTORY_TAG => "GeoKeyDirectory",
        tags::GEO_DOUBLE_PARAMS_TAG => "GeoDoubleParams",
        tags::GEO_ASCII_PARAMS_TAG => "GeoAsciiParams",
        tags::MODEL_TRANSFORMATION_TAG => "ModelTransformation",

        // GDAL specific tags
        tags::GDAL_METADATA => "GDALMetadata",
        tags::GDAL_NODATA => "GDALNoData",

        // Default for unknown tags
        _ => "Unknown",
    }
}

/// Get the name of a TIFF field type
///
/// Returns a human-readable name for a field type based on its numeric ID.
///
/// # Arguments
/// * `field_type` - The field type ID to look up
///
/// # Returns
/// A string representing the field type name
pub fn get_field_type_name(field_type: u16) -> &'static str {
    match field_type {
        field_types::BYTE => "BYTE",
        field_types::ASCII => "ASCII",
        field_types::SHORT => "SHORT",
        field_types::LONG => "LONG",
        field_types::RATIONAL => "RATIONAL",
        field_types::SBYTE => "SBYTE",
        field_types::UNDEFINED => "UNDEFINED",
        field_types::SSHORT => "SSHORT",
        field_types::SLONG => "SLONG",
        field_types::SRATIONAL => "SRATIONAL",
        field_types::FLOAT => "FLOAT",
        field_types::DOUBLE => "DOUBLE",
        field_types::LONG8 => "LONG8",
        field_types::SLONG8 => "SLONG8",
        field_types::IFD8 => "IFD8",
        _ => "Unknown",
    }
}

/// Get the name of a compression method
///
/// Returns a human-readable name for a compression code.
///
/// # Arguments
/// * `compression_code` - The compression code to look up
///
/// # Returns
/// A string representing the compression method name
pub fn get_compression_name(compression_code: u64) -> &'static str {
    match compression_code as u16 {
        compression::NONE => "None",
        compression::CCITT_RLE => "CCITT RLE",
        compression::CCITT_FAX3 => "CCITT Group 3 Fax",
        compression::CCITT_FAX4 => "CCITT Group 4 Fax",
        compression::LZW => "LZW",
        compression::JPEG_OLD => "Old JPEG",
        compression::JPEG => "JPEG",
        compression::DEFLATE => "Adobe Deflate",
        compression::JBIG_BW => "JBIG (B&W)",
        compression::JBIG_COLOR => "JBIG (Color)",
        compression::ZSTD => "Zstandard",
        compression::PACKBITS => "PackBits",
        _ => "Unknown",
    }
}

/// Get the name of a photometric interpretation method
///
/// Returns a human-readable name for a photometric interpretation code.
///
/// # Arguments
/// * `photometric_code` - The photometric interpretation code to look up
///
/// # Returns
/// A string representing the photometric interpretation name
pub fn get_photometric_name(photometric_code: u64) -> &'static str {
    match photometric_code as u16 {
        photometric::WHITE_IS_ZERO => "WhiteIsZero",
        photometric::BLACK_IS_ZERO => "BlackIsZero",
        photometric::RGB => "RGB",
        photometric::PALETTE => "Palette",
        photometric::TRANSPARENCY_MASK => "TransparencyMask",
        photometric::CMYK => "CMYK",
        photometric::YCBCR => "YCbCr",
        photometric::CIELAB => "CIELAB",
        _ => "Unknown",
    }
}