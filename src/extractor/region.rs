//! Region structure for defining extraction area
//!
//! This module defines the Region structure that specifies a rectangular
//! area of an image for extraction. The coordinates are in pixels and
//! follow the typical image coordinate system where (0,0) is the top-left
//! corner of the image.

/// Region for image extraction (in pixel coordinates)
///
/// Represents a rectangular area defined by its top-left corner coordinates
/// and dimensions. This is used to specify which portion of an image should
/// be extracted.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    /// X-coordinate of the top-left corner (pixels from left)
    pub x: u32,

    /// Y-coordinate of the top-left corner (pixels from top)
    pub y: u32,

    /// Width of the region in pixels
    pub width: u32,

    /// Height of the region in pixels
    pub height: u32,
}

impl Region {
    /// Create a new region
    ///
    /// # Arguments
    /// * `x` - X-coordinate of the top-left corner
    /// * `y` - Y-coordinate of the top-left corner
    /// * `width` - Width of the region in pixels
    /// * `height` - Height of the region in pixels
    ///
    /// # Returns
    /// A new Region instance with the specified coordinates and dimensions
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Region { x, y, width, height }
    }

    /// Get the rightmost X coordinate (exclusive)
    ///
    /// Returns the X-coordinate immediately to the right of the region.
    /// This is useful for boundary checks in extraction loops.
    ///
    /// # Returns
    /// The X-coordinate immediately after the rightmost pixel in the region
    pub fn end_x(&self) -> u32 {
        self.x + self.width
    }

    /// Get the bottommost Y coordinate (exclusive)
    ///
    /// Returns the Y-coordinate immediately below the region.
    /// This is useful for boundary checks in extraction loops.
    ///
    /// # Returns
    /// The Y-coordinate immediately after the bottommost pixel in the region
    pub fn end_y(&self) -> u32 {
        self.y + self.height
    }
}