//! TIFF format constants
//!
//! This module defines constants used throughout the TIFF processing code,
//! making the code more readable and maintainable by replacing magic numbers
//! with descriptive names.

/// TIFF header constants
pub mod header {
    /// Standard TIFF version number (42)
    pub const TIFF_VERSION: u16 = 42;

    /// BigTIFF version number (43)
    pub const BIG_TIFF_VERSION: u16 = 43;

    /// "II" byte order marker for little-endian
    pub const LITTLE_ENDIAN_MARKER: [u8; 2] = [0x49, 0x49];

    /// "MM" byte order marker for big-endian
    pub const BIG_ENDIAN_MARKER: [u8; 2] = [0x4D, 0x4D];

    /// BigTIFF offset size (8 bytes)
    pub const BIGTIFF_OFFSET_SIZE: u16 = 8;
}

/// Field types as defined in the TIFF spec
pub mod field_types {
    pub const BYTE: u16 = 1;       // 8-bit unsigned integer
    pub const ASCII: u16 = 2;      // 8-bit byte containing ASCII character
    pub const SHORT: u16 = 3;      // 16-bit unsigned integer
    pub const LONG: u16 = 4;       // 32-bit unsigned integer
    pub const RATIONAL: u16 = 5;   // Two LONGs: numerator and denominator
    pub const SBYTE: u16 = 6;      // 8-bit signed integer
    pub const UNDEFINED: u16 = 7;  // 8-bit byte with unspecified format
    pub const SSHORT: u16 = 8;     // 16-bit signed integer
    pub const SLONG: u16 = 9;      // 32-bit signed integer
    pub const SRATIONAL: u16 = 10; // Two SLONGs: numerator and denominator
    pub const FLOAT: u16 = 11;     // Single precision IEEE floating point
    pub const DOUBLE: u16 = 12;    // Double precision IEEE floating point
    pub const LONG8: u16 = 16;     // BigTIFF 64-bit unsigned integer
    pub const SLONG8: u16 = 17;    // BigTIFF 64-bit signed integer
    pub const IFD8: u16 = 18;      // BigTIFF 64-bit IFD offset
}

/// Standard TIFF tags
pub mod tags {
    // Basic image structure tags
    pub const IMAGE_WIDTH: u16 = 256;              // Width of the image in pixels
    pub const IMAGE_LENGTH: u16 = 257;             // Height of the image in pixels
    pub const BITS_PER_SAMPLE: u16 = 258;          // Bits per component
    pub const COMPRESSION: u16 = 259;              // Compression scheme
    pub const PHOTOMETRIC_INTERPRETATION: u16 = 262; // Color space of image data
    pub const FILL_ORDER: u16 = 266;               // Logical order of bits within a byte
    pub const SAMPLES_PER_PIXEL: u16 = 277;        // Number of components per pixel
    pub const ROWS_PER_STRIP: u16 = 278;           // Rows per strip of data
    pub const STRIP_OFFSETS: u16 = 273;            // Offsets to the data strips
    pub const STRIP_BYTE_COUNTS: u16 = 279;        // Bytes counts for strips
    pub const MIN_SAMPLE_VALUE: u16 = 280;         // Minimum component value
    pub const MAX_SAMPLE_VALUE: u16 = 281;         // Maximum component value
    pub const PLANAR_CONFIGURATION: u16 = 284;     // How components are stored
    pub const COLOR_MAP: u16 = 320;                // Colormap for palette color images
    pub const SAMPLE_FORMAT: u16 = 339;            // Interpretation of sample data
    pub const PREDICTOR: u16 = 317;                // Prediction scheme used on image data

    // Other common tags
    pub const RESOLUTION_UNIT: u16 = 296;          // Unit of measurement for resolution
    pub const X_RESOLUTION: u16 = 282;             // Horizontal resolution
    pub const Y_RESOLUTION: u16 = 283;             // Vertical resolution
    pub const TRANSFER_FUNCTION: u16 = 301;        // Transfer function for image data
    pub const SOFTWARE: u16 = 305;                 // Software used to create the image
    pub const DATE_TIME: u16 = 306;                // Date and time of image creation
    pub const ARTIST: u16 = 315;                   // Person who created the image
    pub const HOST_COMPUTER: u16 = 316;            // Computer where the image was created
    pub const COPYRIGHT: u16 = 33432;              // Copyright notice

    pub const TILE_OFFSETS: u16 = 324;             // Offsets to the data tiles
    pub const TILE_BYTE_COUNTS: u16 = 325;         // Byte counts for tiles
    pub const TILE_WIDTH: u16 = 322;               // Width of a tile
    pub const TILE_LENGTH: u16 = 323;              // Length of a tile

    pub const NEW_SUBFILE_TYPE: u16 = 254;         // Subfile data descriptor
    pub const SUBFILE_TYPE: u16 = 255;             // Old-style subfile data descriptor
    pub const ORIENTATION: u16 = 274;              // Image orientation

    // GeoTIFF tags
    pub const MODEL_PIXEL_SCALE_TAG: u16 = 33550;   // Pixel size in map units
    pub const MODEL_TIEPOINT_TAG: u16 = 33922;      // Links raster to world coordinates
    pub const GEO_KEY_DIRECTORY_TAG: u16 = 34735;   // GeoTIFF keys structure
    pub const GEO_DOUBLE_PARAMS_TAG: u16 = 34736;   // GeoTIFF double parameters
    pub const GEO_ASCII_PARAMS_TAG: u16 = 34737;    // GeoTIFF ASCII parameters
    pub const MODEL_TRANSFORMATION_TAG: u16 = 34264; // Transformation matrix

    // GDAL specific tags
    pub const GDAL_METADATA: u16 = 42112;          // XML metadata
    pub const GDAL_NODATA: u16 = 42113;            // NoData marker value
}

/// Compression types
pub mod compression {
    pub const NONE: u16 = 1;              // No compression
    pub const CCITT_RLE: u16 = 2;         // CCITT modified Huffman RLE
    pub const CCITT_FAX3: u16 = 3;        // CCITT Group 3 fax
    pub const CCITT_FAX4: u16 = 4;        // CCITT Group 4 fax
    pub const LZW: u16 = 5;               // LZW compression
    pub const JPEG_OLD: u16 = 6;          // Old JPEG (deprecated)
    pub const JPEG: u16 = 7;              // JPEG compression
    pub const DEFLATE: u16 = 8;           // Adobe Deflate (zlib)
    pub const JBIG_BW: u16 = 9;           // JBIG for bi-level images
    pub const JBIG_COLOR: u16 = 10;       // JBIG for color images
    pub const ZSTD: u16 = 14;             // Zstandard compression
    pub const PACKBITS: u16 = 32773;      // PackBits compression
}

/// Photometric interpretation values
pub mod photometric {
    pub const WHITE_IS_ZERO: u16 = 0;     // Minimum value is white
    pub const BLACK_IS_ZERO: u16 = 1;     // Minimum value is black
    pub const RGB: u16 = 2;               // RGB color model
    pub const PALETTE: u16 = 3;           // Palette color (color map indexed)
    pub const TRANSPARENCY_MASK: u16 = 4; // Transparency mask
    pub const CMYK: u16 = 5;              // CMYK color model
    pub const YCBCR: u16 = 6;             // YCbCr color model
    pub const CIELAB: u16 = 8;            // CIE L*a*b color model
}

/// Planar configuration values
pub mod planar_config {
    pub const CHUNKY: u16 = 1;            // Components stored interleaved (RGBRGBRGB)
    pub const PLANAR: u16 = 2;            // Components stored separately (RRR...GGG...BBB)
}

/// Sample format values
pub mod sample_format {
    pub const UNSIGNED: u16 = 1;          // Unsigned integer data
    pub const SIGNED: u16 = 2;            // Signed integer data
    pub const IEEEFP: u16 = 3;            // IEEE floating point data
    pub const VOID: u16 = 4;              // Undefined data format
    pub const COMPLEX_INT: u16 = 5;       // Complex integer data
    pub const COMPLEX_IEEEFP: u16 = 6;    // Complex floating point data
}

/// Resolution unit values
pub mod resolution_unit {
    pub const NONE: u16 = 1;              // No meaningful units
    pub const INCH: u16 = 2;              // Inches (default)
    pub const CENTIMETER: u16 = 3;        // Centimeters
}

/// Orientation values
pub mod orientation {
    pub const TOP_LEFT: u16 = 1;          // 0th row = top, 0th column = left side
    pub const TOP_RIGHT: u16 = 2;         // 0th row = top, 0th column = right side
    pub const BOTTOM_RIGHT: u16 = 3;      // 0th row = bottom, 0th column = right side
    pub const BOTTOM_LEFT: u16 = 4;       // 0th row = bottom, 0th column = left side
    pub const LEFT_TOP: u16 = 5;          // 0th row = left side, 0th column = top
    pub const RIGHT_TOP: u16 = 6;         // 0th row = right side, 0th column = top
    pub const RIGHT_BOTTOM: u16 = 7;      // 0th row = right side, 0th column = bottom
    pub const LEFT_BOTTOM: u16 = 8;       // 0th row = left side, 0th column = bottom
}

/// Predictor values
pub mod predictor {
    pub const NONE: u16 = 1;                    // No prediction scheme
    pub const HORIZONTAL_DIFFERENCING: u16 = 2; // Horizontal differencing
    pub const FLOATING_POINT: u16 = 3;          // Floating point predictor
}

/// Fill order values
pub mod fill_order {
    pub const MSB_TO_LSB: u16 = 1;              // Most significant bit to least
    pub const LSB_TO_MSB: u16 = 2;              // Least significant bit to most
}

/// Extra sample values
pub mod extra_samples {
    pub const UNSPECIFIED: u16 = 0;             // Unspecified data
    pub const ASSOCIATED_ALPHA: u16 = 1;        // Associated alpha data
    pub const UNASSOCIATED_ALPHA: u16 = 2;      // Unassociated alpha data
}

/// Subfile type bit flags
pub mod new_subfile_type {
    pub const REDUCED_RESOLUTION: u32 = 1;      // Reduced resolution version of another image
    pub const SINGLE_PAGE: u32 = 2;             // One page of many
    pub const TRANSPARENCY_MASK: u32 = 4;       // Transparency mask for another image
}

/// GeoTIFF Key ID constants
pub mod geo_keys {
    pub const PROJECTED_CS_TYPE: u16 = 3072;  // ProjectedCSTypeGeoKey
    pub const PROJECTION: u16 = 3074;         // ProjectionGeoKey
    pub const GEOGRAPHIC_TYPE: u16 = 2048;    // GeographicTypeGeoKey
    pub const GEOG_LINEAR_UNITS: u16 = 2052;  // GeogLinearUnitsGeoKey
    pub const PROJ_LINEAR_UNITS: u16 = 3076;  // ProjLinearUnitsGeoKey
}

/// EPSG code constants for common projections
pub mod epsg {
    pub const WGS84_WEB_MERCATOR: u16 = 3857;  // Web Mercator
    pub const WGS84: u16 = 4326;               // WGS84 geographic
}

/// GeoTIFF projection method constants
pub mod proj_method {
    pub const LATLONG: u16 = 1;        // Latitude/Longitude
    pub const MERCATOR: u16 = 9;       // Mercator
    pub const STEREOGRAPHIC: u16 = 10; // Stereographic
    pub const TRANSVERSE_MERC: u16 = 11; // Transverse Mercator
}