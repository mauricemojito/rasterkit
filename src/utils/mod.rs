//! Utility modules for common functionality
//!
//! This module provides various utility functions and types used throughout the application.

pub mod logger;
mod progress;
pub(crate) mod tiff_utils;
pub(crate) mod xml_utils;
pub(crate) mod write_utils;
pub mod tiff_code_translators;
mod byte_order_utils;
pub(crate) mod ifd_utils;
pub(crate) mod string_utils;
pub(crate) mod format_utils;
pub(crate) mod tag_utils;
pub(crate) mod tiff_extraction_utils;
pub(crate) mod image_extraction_utils;
pub(crate) mod colormap_utils;
pub(crate) mod reference_utils;
pub(crate) mod coordinate_utils;
pub(crate) mod mask_utils;
mod coordinate_transformer;
pub(crate) mod reprojection_utils;