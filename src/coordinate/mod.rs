//! Coordinate handling for geospatial data
//!
//! This module provides structures and functionality for handling
//! different coordinate systems and transformations.

mod bbox;
mod point;
mod transform;
mod crs;

// Re-export key types
pub use self::bbox::BoundingBox;
pub use self::point::Point;
pub use self::transform::CoordinateTransformer;
pub use self::crs::{CoordinateSystem, CoordinateSystemFactory};