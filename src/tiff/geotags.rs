//! GeoTIFF tag definitions and utilities
//!
//! This module provides structures and methods for handling GeoTIFF tags
//! and GeoKey directories.

use std::collections::HashMap;
use std::fs;
use lazy_static::lazy_static;
use crate::tiff::errors::{TiffError, TiffResult};

// Path to the GeoTIFF tags definition file
const GEOTIFF_TAGS_FILE: &str = "geotiff_tags.toml";

lazy_static! {
    // Parse the TOML file at startup
    static ref GEOTIFF_DEFINITIONS: GeoTiffDefinitions = {
        let content = include_str!("../../geotiff_tags.toml");
        GeoTiffDefinitions::from_str(content).unwrap_or_else(|e| {
                eprintln!("Warning: Failed to parse GeoTIFF tag definitions: {}", e);
                GeoTiffDefinitions::default()
            })
    };
}

/// Container for GeoTIFF tag and key definitions
#[derive(Debug, Default)]
pub struct GeoTiffDefinitions {
    // Maps tag IDs to tag names
    pub tag_names: HashMap<u16, String>,
    // Maps GeoKey IDs to key names
    pub key_names: HashMap<u16, String>,
    // Maps model type codes to names
    pub model_type_names: HashMap<u16, String>,
    // Maps raster type codes to names
    pub raster_type_names: HashMap<u16, String>,
    // Maps linear unit codes to names
    pub linear_unit_names: HashMap<u16, String>,
    // Maps angular unit codes to names
    pub angular_unit_names: HashMap<u16, String>,
    // Maps geographic CS codes to names
    pub geographic_cs_names: HashMap<u16, String>,
    // Maps geodetic datum codes to names
    pub geodetic_datum_names: HashMap<u16, String>,
    // Maps ellipsoid codes to names
    pub ellipsoid_names: HashMap<u16, String>,
    // Maps prime meridian codes to names
    pub prime_meridian_names: HashMap<u16, String>,
    // Maps projected CS codes to names
    pub projected_cs_names: HashMap<u16, String>,
    // Maps projection codes to names
    pub projection_names: HashMap<u16, String>,
    // Maps coordinate transformation codes to names
    pub coord_transform_names: HashMap<u16, String>,
    // Maps vertical CS codes to names
    pub vertical_cs_names: HashMap<u16, String>,
}

impl GeoTiffDefinitions {
    /// Parse GeoTIFF definitions from a TOML string
    pub fn from_str(content: &str) -> TiffResult<Self> {
        let toml_value: toml::Value = match content.parse() {
            Ok(value) => value,
            Err(e) => return Err(TiffError::GenericError(format!("Failed to parse TOML: {}", e))),
        };

        let mut defs = GeoTiffDefinitions::default();

        // Parse tag IDs
        if let Some(table) = toml_value.get("tag_ids").and_then(|v| v.as_table()) {
            for (k, v) in table {
                if let (Ok(id), Some(name)) = (k.parse::<u16>(), v.as_str()) {
                    defs.tag_names.insert(id, name.to_string());
                }
            }
        }

        // Parse key IDs
        if let Some(table) = toml_value.get("key_ids").and_then(|v| v.as_table()) {
            for (k, v) in table {
                if let (Ok(id), Some(name)) = (k.parse::<u16>(), v.as_str()) {
                    defs.key_names.insert(id, name.to_string());
                }
            }
        }

        // Parse remaining code tables
        Self::parse_code_table(&toml_value, "model_type_codes", &mut defs.model_type_names);
        Self::parse_code_table(&toml_value, "raster_type_codes", &mut defs.raster_type_names);
        Self::parse_code_table(&toml_value, "linear_unit_codes", &mut defs.linear_unit_names);
        Self::parse_code_table(&toml_value, "angular_unit_codes", &mut defs.angular_unit_names);
        Self::parse_code_table(&toml_value, "geographic_cs_codes", &mut defs.geographic_cs_names);
        Self::parse_code_table(&toml_value, "geodetic_datum_codes", &mut defs.geodetic_datum_names);
        Self::parse_code_table(&toml_value, "ellipsoid_codes", &mut defs.ellipsoid_names);
        Self::parse_code_table(&toml_value, "prime_meridian_codes", &mut defs.prime_meridian_names);
        Self::parse_code_table(&toml_value, "projected_cs_codes", &mut defs.projected_cs_names);
        Self::parse_code_table(&toml_value, "projection_codes", &mut defs.projection_names);
        Self::parse_code_table(&toml_value, "coord_transformation_codes", &mut defs.coord_transform_names);
        Self::parse_code_table(&toml_value, "vertical_cs_codes", &mut defs.vertical_cs_names);

        Ok(defs)
    }

    /// Helper to parse code tables from TOML
    fn parse_code_table(toml_value: &toml::Value, table_name: &str, target: &mut HashMap<u16, String>) {
        if let Some(table) = toml_value.get(table_name).and_then(|v| v.as_table()) {
            for (k, v) in table {
                if let (Ok(id), Some(name)) = (k.parse::<u16>(), v.as_str()) {
                    target.insert(id, name.to_string());
                }
            }
        }
    }

    /// Load GeoTIFF definitions from a TOML file
    pub fn from_file(path: &str) -> TiffResult<Self> {
        let contents = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => return Err(TiffError::IoError(e)),
        };

        Self::from_str(&contents)
    }

    /// Get a tag name by ID
    pub fn get_tag_name(&self, tag_id: u16) -> String {
        self.tag_names.get(&tag_id)
            .cloned()
            .unwrap_or_else(|| format!("Unknown-{}", tag_id))
    }

    /// Get a GeoKey name by ID
    pub fn get_key_name(&self, key_id: u16) -> String {
        self.key_names.get(&key_id)
            .cloned()
            .unwrap_or_else(|| format!("Unknown-{}", key_id))
    }

    /// Get a code name from the appropriate table
    pub fn get_code_name(&self, code_type: &str, code_id: u16) -> String {
        let lookup_result = match code_type {
            "model_type" => self.model_type_names.get(&code_id),
            "raster_type" => self.raster_type_names.get(&code_id),
            "linear_unit" => self.linear_unit_names.get(&code_id),
            "angular_unit" => self.angular_unit_names.get(&code_id),
            "geographic_cs" => self.geographic_cs_names.get(&code_id),
            "geodetic_datum" => self.geodetic_datum_names.get(&code_id),
            "ellipsoid" => self.ellipsoid_names.get(&code_id),
            "prime_meridian" => self.prime_meridian_names.get(&code_id),
            "projected_cs" => self.projected_cs_names.get(&code_id),
            "projection" => self.projection_names.get(&code_id),
            "coord_transform" => self.coord_transform_names.get(&code_id),
            "vertical_cs" => self.vertical_cs_names.get(&code_id),
            _ => None,
        };

        lookup_result.map_or_else(
            || format!("{}", code_id),
            |s| s.clone()
        )
    }

    pub fn get_projected_cs_description(&self, code: u16) -> String {
        match code {
            // Web Mercator / Google Maps
            3857 => "WGS 84 / Web Mercator (Google Maps, OpenStreetMap)".to_string(),

            // WGS 84 based
            4326 => "WGS 84 (GPS, standard latitude/longitude)".to_string(),
            3395 => "WGS 84 / World Mercator".to_string(),
            4269 => "NAD 83 (North American Datum 1983)".to_string(),
            4267 => "NAD 27 (North American Datum 1927)".to_string(),

            // UTM Zones - WGS84
            32600..=32660 => format!("WGS 84 / UTM Northern Hemisphere zone {}", code - 32600),
            32700..=32760 => format!("WGS 84 / UTM Southern Hemisphere zone {}", code - 32700),

            // UTM Zones - NAD83
            26900..=26923 => format!("NAD 83 / UTM zone {} North", code - 26900),

            // UTM Zones - NAD27
            26700..=26722 => format!("NAD 27 / UTM zone {} North", code - 26700),

            // State Plane - NAD83
            2000..=2056 => "NAD 83 / State Plane (US)".to_string(),

            // European systems
            2044 => "NAD 83 / Alaska Albers".to_string(),
            3035 => "ETRS89 / LAEA Europe (Lambert Azimuthal Equal Area)".to_string(),
            3034 => "ETRS89 / LCC Europe (Lambert Conformal Conic)".to_string(),
            3038..=3051 => "ETRS89 / TM zone (European Transverse Mercator)".to_string(),
            27700 => "OSGB 1936 / British National Grid".to_string(),
            2180 => "ETRS89 / Poland CS92".to_string(),
            2154 => "RGF93 / Lambert-93 (France)".to_string(),
            25830..=25838 => format!("ETRS89 / UTM zone {} North (Europe)", code - 25830),

            // Australian systems
            3112 => "GDA94 / Geoscience Australia Lambert".to_string(),
            28348..=28358 => format!("GDA94 / MGA zone {}", code - 28300),

            // Canadian systems
            3157 => "NAD 83 / Canada Atlas Lambert".to_string(),
            2960..=2962 => "NAD 83 / Quebec Lambert".to_string(),

            // Chinese systems
            4490 => "CGCS 2000 (China)".to_string(),
            4479 => "China Geodetic Coordinate System 2000".to_string(),

            // Global equal area systems
            6933 => "WGS 84 / NSIDC EASE-Grid 2.0 Global".to_string(),
            6931..=6932 => "WGS 84 / NSIDC EASE-Grid 2.0 (equal area)".to_string(),

            // Other common systems
            5070 => "NAD 83 / Conus Albers (US)".to_string(),
            6350 => "EPSG / WGS 84 / UPS North (E,N)".to_string(),
            6351 => "WGS 84 / UPS South (E,N)".to_string(),

            // Default to lookup in the definitions table
            _ => self.projected_cs_names.get(&code)
                .cloned()
                .unwrap_or_else(|| format!("EPSG:{}", code))
        }
    }
}

// Common GeoTIFF tag constants
pub const TAG_MODEL_PIXEL_SCALE: u16 = 33550;
pub const TAG_MODEL_TRANSFORMATION: u16 = 34264;
pub const TAG_MODEL_TIEPOINT: u16 = 33922;
pub const TAG_GEO_KEY_DIRECTORY: u16 = 34735;
pub const TAG_GEO_DOUBLE_PARAMS: u16 = 34736;
pub const TAG_GEO_ASCII_PARAMS: u16 = 34737;
pub const TAG_INTERGRAPH_MATRIX: u16 = 33920;

// Common GeoKey constants
pub const KEY_MODEL_TYPE: u16 = 1024;
pub const KEY_RASTER_TYPE: u16 = 1025;
pub const KEY_GEOGRAPHIC_TYPE: u16 = 2048;
pub const KEY_PROJECTED_CS_TYPE: u16 = 3072;
pub const KEY_VERTICAL_CS_TYPE: u16 = 4096;

/// Represents a GeoKey entry in a GeoKey directory
#[derive(Debug, Clone)]
pub struct GeoKeyEntry {
    pub key_id: u16,
    pub tiff_tag_location: u16,
    pub count: u16,
    pub value_offset: u16,
}

impl GeoKeyEntry {
    /// Create a new GeoKey entry
    pub fn new(key_id: u16, tiff_tag_location: u16, count: u16, value_offset: u16) -> Self {
        GeoKeyEntry {
            key_id,
            tiff_tag_location,
            count,
            value_offset,
        }
    }

    /// Get the name of this key
    pub fn get_name(&self) -> String {
        get_key_name(self.key_id)
    }
}

/// Check if a tag is a GeoTIFF tag
pub fn is_geotiff_tag(tag: u16) -> bool {
    matches!(tag,
        TAG_MODEL_PIXEL_SCALE |
        TAG_MODEL_TRANSFORMATION |
        TAG_MODEL_TIEPOINT |
        TAG_GEO_KEY_DIRECTORY |
        TAG_GEO_DOUBLE_PARAMS |
        TAG_GEO_ASCII_PARAMS |
        TAG_INTERGRAPH_MATRIX)
}

/// Get a GeoTIFF tag name
pub fn get_tag_name(tag: u16) -> String {
    GEOTIFF_DEFINITIONS.get_tag_name(tag)
}

/// Get a GeoKey name
pub fn get_key_name(key: u16) -> String {
    GEOTIFF_DEFINITIONS.get_key_name(key)
}

/// Get a code name
pub fn get_code_name(code_type: &str, code: u16) -> String {
    GEOTIFF_DEFINITIONS.get_code_name(code_type, code)
}

/// Get a projected coordinate system description
pub fn get_projected_cs_description(code: u16) -> String {
    GEOTIFF_DEFINITIONS.get_projected_cs_description(code)
}