//! Image extraction from various raster formats
//!
//! This module provides functionality to extract image data from different
//! file formats using a strategy pattern.

mod region;
mod extractor_strategy;
mod tiff_strategy;
mod tile_reader;
mod strip_reader;
mod array_strategy;

// Public exports
pub use region::Region;
pub use extractor_strategy::{ExtractorStrategy, ExtractorStrategyFactory};
pub use tiff_strategy::TiffExtractorStrategy;
pub use array_strategy::{ArrayExtractorStrategy, ArrayData};

// Simple facade that delegates to the appropriate strategy
pub use extractor_strategy::ImageExtractor;