//! Coordinate transformation functionality

use super::point::Point;
use super::bbox::BoundingBox;
use super::crs::CoordinateSystem;
use crate::tiff::errors::{TiffError, TiffResult};
use std::f64::consts::PI;

/// Transformer for converting between coordinate systems
pub struct CoordinateTransformer;

impl CoordinateTransformer {
    /// Earth radius in meters
    const EARTH_RADIUS: f64 = 6378137.0;

    /// Convert from WGS84 (EPSG:4326) to Web Mercator (EPSG:3857)
    pub fn wgs84_to_web_mercator(&self, lon: f64, lat: f64) -> Point {
        // Web Mercator has limits - constrain latitude to valid range
        // Maximum latitude for Web Mercator is ~85.05 degrees
        let lat = lat.max(-85.05).min(85.05);

        // Convert to Web Mercator
        let x = lon * Self::EARTH_RADIUS * PI / 180.0;
        let y = f64::ln(f64::tan((90.0 + lat) * PI / 360.0)) * Self::EARTH_RADIUS;

        Point::new(x, y)
    }

    /// Convert from Web Mercator (EPSG:3857) to WGS84 (EPSG:4326)
    pub fn web_mercator_to_wgs84(&self, x: f64, y: f64) -> Point {
        // Convert to longitude/latitude
        let lon = x * 180.0 / (Self::EARTH_RADIUS * PI);
        let lat = 180.0 / PI * (2.0 * f64::atan(f64::exp(y / Self::EARTH_RADIUS)) - PI / 2.0);

        Point::new(lon, lat)
    }

    /// Transform a point between coordinate systems
    pub fn transform_point(&self, point: &Point, from_crs: &CoordinateSystem, to_crs: &CoordinateSystem) -> TiffResult<Point> {
        if from_crs == to_crs {
            return Ok(*point);
        }

        match (from_crs, to_crs) {
            (CoordinateSystem::WGS84, CoordinateSystem::WebMercator) => {
                Ok(self.wgs84_to_web_mercator(point.x, point.y))
            },
            (CoordinateSystem::WebMercator, CoordinateSystem::WGS84) => {
                Ok(self.web_mercator_to_wgs84(point.x, point.y))
            },
            _ => Err(TiffError::GenericError(format!(
                "Unsupported coordinate transformation from {} to {}",
                from_crs.description(), to_crs.description()
            ))),
        }
    }

    /// Transform a bounding box between coordinate systems
    pub fn transform_bbox(&self, bbox: &BoundingBox, from_crs: &CoordinateSystem, to_crs: &CoordinateSystem) -> TiffResult<BoundingBox> {
        if from_crs == to_crs {
            return Ok(*bbox);
        }

        let min_point = Point::new(bbox.min_x, bbox.min_y);
        let max_point = Point::new(bbox.max_x, bbox.max_y);

        let transformed_min = self.transform_point(&min_point, from_crs, to_crs)?;
        let transformed_max = self.transform_point(&max_point, from_crs, to_crs)?;

        Ok(BoundingBox::new(
            transformed_min.x,
            transformed_min.y,
            transformed_max.x,
            transformed_max.y,
        ))
    }

    /// Create a buffer around a point in the given coordinate system
    pub fn create_buffer(&self, center: &Point, buffer_size: f64, crs: &CoordinateSystem) -> BoundingBox {
        // For geographic coordinates, we need to adjust for latitude
        match crs {
            CoordinateSystem::WGS84 => {
                // Convert buffer size from meters to approximate degrees
                // This is a simple approximation, not accurate for all locations
                let lat_buffer = buffer_size / 111320.0; // 1 degree latitude is approximately 111.32 km

                // Longitude degrees vary with latitude
                let lon_buffer = buffer_size / (111320.0 * f64::cos(center.y.to_radians()));

                BoundingBox::new(
                    center.x - lon_buffer,
                    center.y - lat_buffer,
                    center.x + lon_buffer,
                    center.y + lat_buffer,
                )
            },
            _ => {
                // For projected coordinates, we can use a simple square buffer
                BoundingBox::from_point_buffer(center, buffer_size)
            }
        }
    }
}