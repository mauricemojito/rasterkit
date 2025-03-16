//! Bounding box structure for defining regions

use super::point::Point;

/// A bounding box in a coordinate system
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    /// Minimum X coordinate
    pub min_x: f64,
    /// Minimum Y coordinate
    pub min_y: f64,
    /// Maximum X coordinate
    pub max_x: f64,
    /// Maximum Y coordinate
    pub max_y: f64,
    /// EPSG code of the coordinate system
    pub epsg: Option<u32>,
}

impl BoundingBox {
    /// Create a new bounding box
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        BoundingBox {
            min_x,
            min_y,
            max_x,
            max_y,
            epsg: None,
        }
    }

    /// Create a new bounding box with coordinate system
    pub fn new_with_crs(min_x: f64, min_y: f64, max_x: f64, max_y: f64, epsg: u32) -> Self {
        BoundingBox {
            min_x,
            min_y,
            max_x,
            max_y,
            epsg: Some(epsg),
        }
    }

    /// Parse a bounding box from a string (format: "minx,miny,maxx,maxy")
    pub fn from_string(bbox_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = bbox_str.split(',').collect();
        if parts.len() != 4 {
            return Err("Bounding box must have 4 comma-separated values".to_string());
        }

        let min_x = parts[0].trim().parse::<f64>()
            .map_err(|_| "Invalid min_x value".to_string())?;
        let min_y = parts[1].trim().parse::<f64>()
            .map_err(|_| "Invalid min_y value".to_string())?;
        let max_x = parts[2].trim().parse::<f64>()
            .map_err(|_| "Invalid max_x value".to_string())?;
        let max_y = parts[3].trim().parse::<f64>()
            .map_err(|_| "Invalid max_y value".to_string())?;

        Ok(BoundingBox::new(min_x, min_y, max_x, max_y))
    }

    /// Get the width of the bounding box
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Get the height of the bounding box
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Get the center point of the bounding box
    pub fn center(&self) -> Point {
        Point::new(
            self.min_x + self.width() / 2.0,
            self.min_y + self.height() / 2.0,
        )
    }

    /// Check if this bounding box contains a point
    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.min_x && point.x <= self.max_x &&
            point.y >= self.min_y && point.y <= self.max_y
    }

    /// Create a buffer around a point (square buffer)
    pub fn from_point_buffer(center: &Point, buffer_size: f64) -> Self {
        BoundingBox::new(
            center.x - buffer_size,
            center.y - buffer_size,
            center.x + buffer_size,
            center.y + buffer_size,
        )
    }

    /// Convert to a pixel region given a geotransform
    pub fn to_pixel_region(&self, geotransform: &[f64]) -> crate::extractor::Region {
        let origin_x = geotransform[0];
        let pixel_width = geotransform[1];
        let origin_y = geotransform[3];
        let pixel_height = geotransform[5];

        // Calculate pixel coordinates - use f64 for intermediate calculations to avoid overflow
        let x_min_f = ((self.min_x - origin_x) / pixel_width).floor();
        let y_min_f = ((self.min_y - origin_y) / pixel_height).floor();
        let x_max_f = ((self.max_x - origin_x) / pixel_width).ceil();
        let y_max_f = ((self.max_y - origin_y) / pixel_height).ceil();

        // Convert to i64 to handle possible negative values safely
        let x_min = x_min_f as i64;
        let y_min = y_min_f as i64;
        let x_max = x_max_f as i64;
        let y_max = y_max_f as i64;

        // Ensure coordinates are positive or zero
        let start_x = x_min.max(0) as u32;
        let start_y = y_min.max(0) as u32;

        // Calculate dimensions safely
        let width = (x_max - x_min).max(0) as u32;
        let height = (y_max - y_min).max(0) as u32;

        crate::extractor::Region::new(start_x, start_y, width, height)
    }
}