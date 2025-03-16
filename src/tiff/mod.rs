//! TIFF file format parsing module
//!
//! This module provides structures and functions for reading
//! TIFF and BigTIFF format files.

pub mod errors;
pub mod ifd;
pub(crate) mod types;
pub mod reader;
mod tests;
pub mod geotags;
pub mod builder;
mod builders;
pub(crate) mod constants;
pub mod geo_key_parser;
pub(crate) mod validation;
pub(crate) mod colormap;

pub use crate::io::byte_order::{BigEndianHandler, ByteOrder, ByteOrderHandler, LittleEndianHandler};
pub use errors::{TiffError, TiffResult};
pub use ifd::{IFD, IFDEntry};
pub use reader::TiffReader;
pub use types::TIFF;
pub use geotags::{GeoKeyEntry, get_key_name, get_projected_cs_description, get_tag_name, is_geotiff_tag};
pub use builder::TiffBuilder;

// Constants for TIFF format
pub const BIGTIFF_VERSION: u16 = 43;
pub const BIGTIFF_OFFSETSIZE: u16 = 8;