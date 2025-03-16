//! Coordinate Reference System handling

use crate::tiff::errors::{TiffError, TiffResult};

/// Identifier for common coordinate systems
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateSystem {
    /// WGS 84 (EPSG:4326)
    WGS84,
    /// Web Mercator (EPSG:3857)
    WebMercator,
    /// UTM Zone (EPSG:326xx for northern hemisphere, 327xx for southern)
    UTM(u8, bool),
    /// Other EPSG code
    Other(u32),
}

impl CoordinateSystem {
    /// Get the EPSG code for this coordinate system
    pub fn epsg_code(&self) -> u32 {
        match self {
            CoordinateSystem::WGS84 => 4326,
            CoordinateSystem::WebMercator => 3857,
            CoordinateSystem::UTM(zone, is_northern) => {
                if *is_northern {
                    32600 + *zone as u32
                } else {
                    32700 + *zone as u32
                }
            },
            CoordinateSystem::Other(code) => *code,
        }
    }

    /// Get a description of this coordinate system
    pub fn description(&self) -> String {
        match self {
            CoordinateSystem::WGS84 => "WGS 84 (EPSG:4326)".to_string(),
            CoordinateSystem::WebMercator => "Web Mercator (EPSG:3857)".to_string(),
            CoordinateSystem::UTM(zone, is_northern) => {
                if *is_northern {
                    format!("UTM Zone {}N (EPSG:{})", zone, self.epsg_code())
                } else {
                    format!("UTM Zone {}S (EPSG:{})", zone, self.epsg_code())
                }
            },
            CoordinateSystem::Other(code) => format!("EPSG:{}", code),
        }
    }
}

/// Factory for creating coordinate systems
pub struct CoordinateSystemFactory;

impl CoordinateSystemFactory {
    /// Create a coordinate system from an EPSG code
    pub fn from_epsg(epsg: u32) -> TiffResult<CoordinateSystem> {
        match epsg {
            4326 => Ok(CoordinateSystem::WGS84),
            3857 => Ok(CoordinateSystem::WebMercator),
            32601..=32660 => Ok(CoordinateSystem::UTM((epsg - 32600) as u8, true)),
            32701..=32760 => Ok(CoordinateSystem::UTM((epsg - 32700) as u8, false)),
            _ => Ok(CoordinateSystem::Other(epsg)),
        }
    }

    /// Parse a coordinate system from a string (e.g. "EPSG:4326")
    pub fn from_string(crs_str: &str) -> TiffResult<CoordinateSystem> {
        let crs_str = crs_str.trim().to_uppercase();

        if crs_str.starts_with("EPSG:") {
            let epsg_str = crs_str.strip_prefix("EPSG:").unwrap();
            match epsg_str.parse::<u32>() {
                Ok(epsg) => Self::from_epsg(epsg),
                Err(_) => Err(TiffError::GenericError(format!("Invalid EPSG code: {}", epsg_str))),
            }
        } else if let Ok(epsg) = crs_str.parse::<u32>() {
            Self::from_epsg(epsg)
        } else {
            Err(TiffError::GenericError(format!("Unsupported CRS format: {}", crs_str)))
        }
    }
}