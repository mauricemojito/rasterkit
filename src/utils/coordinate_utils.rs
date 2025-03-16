//! Coordinate utility functions
//!
//! Utilities for working with geographic coordinates, including conversion between
//! point-and-radius specifications and bounding boxes. These functions support more
//! intuitive ways to specify extraction regions for geospatial data.

use crate::tiff::errors::{TiffError, TiffResult};
use crate::coordinate::BoundingBox;
use std::f64::consts::PI;
use log::{debug, info};

/// Convert a coordinate and radius to a bounding box string
///
/// Takes a geographic coordinate and a radius, and converts them to a bounding box
/// representation that can be used for extraction. The shape parameter controls whether
/// the resulting bounding box should encompass a square or circle around the point.
///
/// # Arguments
/// * `coord_str` - Coordinate string in format "x,y" or "lon,lat" for EPSG:4326
/// * `radius` - Radius in meters
/// * `shape` - Shape to use ("circle" or "square")
/// * `epsg` - Optional EPSG code for the coordinate reference system
///
/// # Returns
/// A string representation of the bounding box or an error
///
/// # Note
/// For EPSG:4326 (WGS84), the coordinate string should be "longitude,latitude"
/// The resulting bounding box will correctly account for distortion at different latitudes.
pub fn coord_to_bbox(coord_str: &str, radius: f64, shape: &str, epsg: Option<u32>) -> TiffResult<String> {
    debug!("Converting coordinate '{}' with radius {} meters to bounding box (shape: {})",
           coord_str, radius, shape);

    // Parse the coordinate
    let parts: Vec<&str> = coord_str.split(',').collect();
    if parts.len() != 2 {
        return Err(TiffError::GenericError(
            "Coordinate must be in format 'x,y' or 'lon,lat' for EPSG:4326".to_string()));
    }

    let x = parts[0].trim().parse::<f64>()
        .map_err(|_| TiffError::GenericError("Invalid x/longitude coordinate".to_string()))?;
    let y = parts[1].trim().parse::<f64>()
        .map_err(|_| TiffError::GenericError("Invalid y/latitude coordinate".to_string()))?;

    debug!("Parsed coordinates: x/lon={}, y/lat={}", x, y);

    // Calculate bounding box based on shape and EPSG
    match shape.to_lowercase().as_str() {
        "circle" => {
            // For circular extraction, create a bounding box that encompasses the circle
            let (min_x, min_y, max_x, max_y) = calculate_circle_bbox(x, y, radius, epsg);
            debug!("Calculated circular bounding box: min_x={}, min_y={}, max_x={}, max_y={}",
                  min_x, min_y, max_x, max_y);

            // Format the bounding box string
            Ok(format!("{},{},{},{}", min_x, min_y, max_x, max_y))
        },
        "square" | _ => {
            // For Web Mercator and other projected systems where coordinates are in meters
            if let Some(code) = epsg {
                if code == 3857 || code == 3785 || code == 900913 {
                    // For projected coordinates in meters, we can add/subtract the radius directly
                    debug!("Square bbox for projected coordinates (EPSG:{}) in meters", code);
                    return Ok(format!("{},{},{},{}",
                                      x - radius, y - radius,
                                      x + radius, y + radius));
                }
                else if code == 4326 {
                    // For WGS84, convert meters to degrees based on latitude
                    debug!("Square bbox for WGS84 coordinates (EPSG:4326)");

                    // Extract longitude and latitude from the input
                    // In WGS84 (EPSG:4326), the first coordinate is longitude, the second is latitude
                    let lon = x;  // x is longitude
                    let lat = y;  // y is latitude

                    // Convert meters to degrees (dependent on latitude)
                    let lat_degree_meters = meters_per_latitude_degree();
                    let lon_degree_meters = meters_per_longitude_degree(lat);  // Note: using lat, not y

                    let lat_buffer = radius / lat_degree_meters;
                    let lon_buffer = radius / lon_degree_meters;

                    debug!("Lat buffer: {} degrees, Lon buffer: {} degrees at latitude {}",
           lat_buffer, lon_buffer, lat);

                    return Ok(format!("{},{},{},{}",
                                      lon - lon_buffer, lat - lat_buffer,
                                      lon + lon_buffer, lat + lat_buffer));
                }
            }

            // For generic case (degrees or other units)
            debug!("Using general calculation for square bbox");
            let half_size = radius / meters_per_degree(y, epsg);
            let bbox = format!("{},{},{},{}",
                               x - half_size, y - half_size,
                               x + half_size, y + half_size);
            debug!("Calculated square bounding box: {}", bbox);

            Ok(bbox)
        }
    }
}

/// Calculate a bounding box that surrounds a circle centered at a point
///
/// This function computes the corners of a bounding box that fully contains
/// a circle of the specified radius around the given coordinate.
///
/// # Arguments
/// * `x` - X coordinate of the center point (longitude for EPSG:4326)
/// * `y` - Y coordinate of the center point (latitude for EPSG:4326)
/// * `radius` - Radius in meters
/// * `epsg` - Optional EPSG code for the coordinate reference system
///
/// # Returns
/// A tuple containing (min_x, min_y, max_x, max_y)
fn calculate_circle_bbox(x: f64, y: f64, radius: f64, epsg: Option<u32>) -> (f64, f64, f64, f64) {
    // Web Mercator (EPSG:3857) and similar projections - direct calculation in meters
    if let Some(code) = epsg {
        if code == 3857 || code == 3785 || code == 900913 {
            debug!("Circle bbox for Web Mercator (EPSG:{})", code);
            return (x - radius, y - radius, x + radius, y + radius);
        }
        else if code == 4326 {
            // WGS84 (EPSG:4326) - lat/lon in degrees
            debug!("Circle bbox for WGS84 (EPSG:4326)");

            // Convert meters to degrees (dependent on latitude)
            let lat_degree_meters = meters_per_latitude_degree();
            let lon_degree_meters = meters_per_longitude_degree(y);

            let lat_buffer = radius / lat_degree_meters;
            let lon_buffer = radius / lon_degree_meters;

            debug!("Lat buffer: {} degrees, Lon buffer: {} degrees at latitude {}",
                   lat_buffer, lon_buffer, y);

            return (x - lon_buffer, y - lat_buffer, x + lon_buffer, y + lat_buffer);
        }
    }

    // Generic calculation for other coordinate systems
    debug!("Generic circle bbox calculation");
    let degrees_per_m = 1.0 / meters_per_degree(y, epsg);
    let radius_deg = radius * degrees_per_m;

    debug!("Converting radius {} meters to {} degrees at latitude/y={}",
           radius, radius_deg, y);

    (x - radius_deg, y - radius_deg, x + radius_deg, y + radius_deg)
}

/// Calculate meters per degree of latitude (approximately constant globally)
///
/// The length of a degree of latitude is relatively constant,
/// varying only slightly due to the Earth's ellipsoidal shape.
///
/// # Returns
/// Number of meters per degree of latitude
fn meters_per_latitude_degree() -> f64 {
    // Average value, varies between about 110.57km and 111.69km
    111_320.0
}

/// Calculate meters per degree of longitude at a given latitude
///
/// The length of a degree of longitude varies considerably with latitude,
/// being largest at the equator and approaching zero at the poles.
///
/// # Arguments
/// * `latitude` - Latitude in degrees
///
/// # Returns
/// Number of meters per degree of longitude at the specified latitude
fn meters_per_longitude_degree(latitude: f64) -> f64 {
    // Convert latitude to radians
    let lat_rad = latitude * PI / 180.0;

    // Formula: 111,320 * cos(latitude)
    111_320.0 * f64::cos(lat_rad)
}

/// Calculate how many meters per degree at a given latitude
///
/// This function approximates the length of a degree in meters at a specific
/// latitude. This is necessary because the size of a degree varies with latitude
/// due to the Earth's shape.
///
/// # Arguments
/// * `latitude` - Latitude in degrees
/// * `epsg` - Optional EPSG code for the coordinate reference system
///
/// # Returns
/// Number of meters per degree at the specified latitude
fn meters_per_degree(latitude: f64, epsg: Option<u32>) -> f64 {
    // If EPSG indicates this is not in lat/lon but in meters (like Web Mercator)
    if let Some(code) = epsg {
        match code {
            3857 | 3785 | 900913 => {
                debug!("EPSG code {} indicates coordinates are in meters already", code);
                return 1.0; // These are in meters already
            },
            4326 => {
                // For WGS84, calculate differently based on actual formulas
                let lat_meters = meters_per_latitude_degree();
                let lon_meters = meters_per_longitude_degree(latitude);
                debug!("At latitude {}: lat meters/deg={}, lon meters/deg={}",
                       latitude, lat_meters, lon_meters);
                return (lat_meters + lon_meters) / 2.0; // Average for approximation
            },
            _ => {}
        }
    }

    // For geographic coordinates, we need to account for the Earth's curvature
    let lat_rad = latitude.abs() * PI / 180.0;  // Use absolute latitude

    // Length of a degree of latitude (approximately constant)
    let lat_length = 111_132.92 - 559.82 * f64::cos(2.0 * lat_rad) + 1.175 * f64::cos(4.0 * lat_rad);

    // Length of a degree of longitude (varies with latitude)
    let lon_length = 111_412.84 * f64::cos(lat_rad) - 93.5 * f64::cos(3.0 * lat_rad);

    // Average for a rough approximation
    let meters_per_deg = (lat_length + lon_length) / 2.0;
    debug!("At latitude {}: estimated {} meters per degree", latitude, meters_per_deg);

    meters_per_deg
}

