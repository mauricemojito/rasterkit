//! Point structure for representing coordinates

/// A point in a coordinate system
#[derive(Debug, Clone, Copy)]
pub struct Point {
    /// X coordinate (longitude in geographic systems)
    pub x: f64,
    /// Y coordinate (latitude in geographic systems)
    pub y: f64,
    /// Z coordinate (elevation, optional)
    pub z: Option<f64>,
}

impl Point {
    /// Create a new 2D point
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y, z: None }
    }

    /// Create a new 3D point
    pub fn new_3d(x: f64, y: f64, z: f64) -> Self {
        Point { x, y, z: Some(z) }
    }

    /// Check if this point has a Z coordinate
    pub fn has_z(&self) -> bool {
        self.z.is_some()
    }

    /// Get the Z coordinate, or 0.0 if not present
    pub fn z_value(&self) -> f64 {
        self.z.unwrap_or(0.0)
    }
}