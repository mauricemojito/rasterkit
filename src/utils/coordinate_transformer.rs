//! Coordinate transformation utilities
//!
//! This module provides functions for transforming coordinates between different
//! coordinate reference systems, particularly between WGS84 (EPSG:4326) and
//! Web Mercator (EPSG:3857).

use std::f64::consts::PI;
use log::debug;
use crate::coordinate::Point;

/// Earth radius in meters used for coordinate transformations
pub const EARTH_RADIUS: f64 = 6378137.0;

/// Convert coordinates from WGS84 (EPSG:4326) to Web Mercator (EPSG:3857)
///
/// # Arguments
/// * `lon` - Longitude in degrees (WGS84)
/// * `lat` - Latitude in degrees (WGS84)
///
/// # Returns
/// A Point with x and y coordinates in meters (Web Mercator)
pub fn wgs84_to_web_mercator(lon: f64, lat: f64) -> Point {
    use std::f64::consts::PI;

    // Constrain latitude to valid range for Web Mercator (-85.06 to 85.06)
    let lat_constrained = lat.max(-85.06).min(85.06);

    // Convert longitude to x (6378137.0 is the Earth radius in meters)
    let x = lon * PI * 6378137.0 / 180.0;

    // Convert latitude to y using the Mercator formula
    let lat_rad = lat_constrained * PI / 180.0;
    let y = 6378137.0 * f64::ln(f64::tan(PI/4.0 + lat_rad/2.0));

    debug!("Transformed WGS84 ({}, {}) to Web Mercator ({}, {})",
           lon, lat, x, y);

    Point::new(x, y)
}

/// Convert coordinates from Web Mercator (EPSG:3857) to WGS84 (EPSG:4326)
///
/// # Arguments
/// * `x` - X coordinate in meters (Web Mercator)
/// * `y` - Y coordinate in meters (Web Mercator)
///
/// # Returns
/// A Point with longitude and latitude in degrees (WGS84)
///
/// # Examples
/// ```
/// let wgs84 = web_mercator_to_wgs84(-8237642.2, 4970241.3);
/// println!("WGS84: ({}, {})", wgs84.x, wgs84.y);
/// ```
pub fn web_mercator_to_wgs84(x: f64, y: f64) -> Point {
    // Convert x to longitude
    let lon = (x * 180.0) / (EARTH_RADIUS * PI);

    // Convert y to latitude
    let lat = (2.0 * f64::atan(f64::exp(y / EARTH_RADIUS)) - PI/2.0) * 180.0 / PI;

    debug!("Transformed Web Mercator ({}, {}) to WGS84 ({}, {})",
           x, y, lon, lat);

    Point::new(lon, lat)
}

/// Convert a bounding box from WGS84 to Web Mercator
///
/// # Arguments
/// * `min_x` - Minimum longitude in degrees (WGS84)
/// * `min_y` - Minimum latitude in degrees (WGS84)
/// * `max_x` - Maximum longitude in degrees (WGS84)
/// * `max_y` - Maximum latitude in degrees (WGS84)
///
/// # Returns
/// A tuple containing (min_x, min_y, max_x, max_y) in meters (Web Mercator)
pub fn wgs84_bbox_to_web_mercator(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> (f64, f64, f64, f64) {
    // In WGS84, longitude comes first (x), latitude second (y)
    let sw = wgs84_to_web_mercator(min_x, min_y);
    let ne = wgs84_to_web_mercator(max_x, max_y);

    // For Web Mercator, ensure proper min/max ordering
    let min_mercator_x = sw.x.min(ne.x);
    let min_mercator_y = sw.y.min(ne.y);
    let max_mercator_x = sw.x.max(ne.x);
    let max_mercator_y = sw.y.max(ne.y);

    debug!("Transformed WGS84 bbox ({}, {}, {}, {}) to Web Mercator ({}, {}, {}, {})",
           min_x, min_y, max_x, max_y, min_mercator_x, min_mercator_y, max_mercator_x, max_mercator_y);

    (min_mercator_x, min_mercator_y, max_mercator_x, max_mercator_y)
}