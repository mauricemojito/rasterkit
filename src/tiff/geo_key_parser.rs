//! GeoTIFF Metadata and GeoKey parsing functionality
//!
//! This module provides utilities for parsing and interpreting
//! geographic metadata stored in TIFF files according to the GeoTIFF standard.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use log::debug;

use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::IFD;
use crate::tiff::{GeoKeyEntry, get_key_name};
use crate::tiff::constants::{tags, geo_keys, epsg, proj_method};
use crate::io::byte_order::ByteOrderHandler;

/// Parser for GeoTIFF geographic metadata
pub struct GeoKeyParser;

impl GeoKeyParser {
    /// Parse the GeoKey directory from an IFD
    ///
    /// GeoKeys are the main way geographic information is stored in GeoTIFF files.
    /// They are organized as a directory with a standard structure defined by the
    /// GeoTIFF specification, consisting of a header and a series of key entries.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing the GeoKey directory
    /// * `byte_order_handler` - Handler for the file's byte order
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// * `TiffResult<Vec<GeoKeyEntry>>` - A vector of GeoKey entries if found
    pub fn parse_geo_key_directory(
        ifd: &IFD,
        byte_order_handler: &Box<dyn ByteOrderHandler>,
        file_path: &str
    ) -> TiffResult<Vec<GeoKeyEntry>> {
        // Check if the IFD has a GeoKeyDirectoryTag
        let geo_key_dir_entry = match ifd.get_entry(tags::GEO_KEY_DIRECTORY_TAG) {
            Some(entry) => entry,
            None => return Ok(Vec::new()), // No GeoKey directory
        };

        let key_dir_offset = geo_key_dir_entry.value_offset;
        let key_dir_count = geo_key_dir_entry.count;

        // GeoKey directory should have at least 4 values (header)
        if key_dir_count < 4 {
            return Err(TiffError::GenericError("Invalid GeoKey directory header".to_string()));
        }

        let file = File::open(file_path)?;
        let mut reader = file;
        reader.seek(SeekFrom::Start(key_dir_offset))?;

        // Read header (4 shorts: KeyDirectoryVersion, KeyRevision, MinorRevision, NumberOfKeys)
        let _key_dir_version = byte_order_handler.read_u16(&mut reader)?;
        let _key_revision = byte_order_handler.read_u16(&mut reader)?;
        let _minor_revision = byte_order_handler.read_u16(&mut reader)?;
        let num_keys = byte_order_handler.read_u16(&mut reader)?;

        debug!("GeoKey directory: version={}, revision={}.{}, keys={}",
             _key_dir_version, _key_revision, _minor_revision, num_keys);

        let mut geo_keys = Vec::with_capacity(num_keys as usize);

        // Read key entries (4 shorts each: KeyID, TIFFTagLocation, Count, Value_Offset)
        for _ in 0..num_keys {
            let key_id = byte_order_handler.read_u16(&mut reader)?;
            let tiff_tag_location = byte_order_handler.read_u16(&mut reader)?;
            let count = byte_order_handler.read_u16(&mut reader)?;
            let value_offset = byte_order_handler.read_u16(&mut reader)?;

            debug!("GeoKey: id={} ({}), location={}, count={}, offset={}",
                 key_id, get_key_name(key_id), tiff_tag_location, count, value_offset);

            geo_keys.push(GeoKeyEntry::new(key_id, tiff_tag_location, count, value_offset));
        }

        Ok(geo_keys)
    }

    /// Get the value of a GeoKey as a string
    ///
    /// GeoKeys can store values in three ways:
    /// 1. Directly in the value_offset field (when tiff_tag_location = 0)
    /// 2. In the GeoDoubleParamsTag (when tiff_tag_location = 34736)
    /// 3. In the GeoAsciiParamsTag (when tiff_tag_location = 34737)
    ///
    /// This method handles all three cases and returns the value as a string.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing the GeoKey
    /// * `key_entry` - The specific GeoKey entry to retrieve
    /// * `byte_order_handler` - Handler for the file's byte order
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// * `TiffResult<String>` - The key's value as a string
    pub fn get_geo_key_value_as_string(
        ifd: &IFD,
        key_entry: &GeoKeyEntry,
        byte_order_handler: &Box<dyn ByteOrderHandler>,
        file_path: &str
    ) -> TiffResult<String> {
        // If TIFFTagLocation is 0, the value is directly in value_offset
        if key_entry.tiff_tag_location == 0 {
            return Ok(format!("{}", key_entry.value_offset));
        }

        // Otherwise, we need to look up the value in the specified tag
        if key_entry.tiff_tag_location == tags::GEO_DOUBLE_PARAMS_TAG {
            if let Some(entry) = ifd.get_entry(tags::GEO_DOUBLE_PARAMS_TAG) {
                let offset = entry.value_offset;
                let file = File::open(file_path)?;
                let mut reader = file;
                reader.seek(SeekFrom::Start(offset + (key_entry.value_offset as u64) * 8))?;

                let value = byte_order_handler.read_f64(&mut reader)?;
                return Ok(format!("{}", value));
            }
        } else if key_entry.tiff_tag_location == tags::GEO_ASCII_PARAMS_TAG {
            if let Some(entry) = ifd.get_entry(tags::GEO_ASCII_PARAMS_TAG) {
                let offset = entry.value_offset;
                let file = File::open(file_path)?;
                let mut reader = file;
                reader.seek(SeekFrom::Start(offset + (key_entry.value_offset as u64)))?;

                let mut buffer = vec![0u8; key_entry.count as usize];
                reader.read_exact(&mut buffer)?;

                // Remove trailing nulls and convert to string
                while !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
                    buffer.pop();
                }

                return Ok(String::from_utf8_lossy(&buffer).to_string());
            }
        }

        Err(TiffError::GenericError(format!("Could not retrieve GeoKey value for key {}", key_entry.key_id)))
    }

    /// Read model pixel scale values (x_scale, y_scale, z_scale)
    ///
    /// ModelPixelScaleTag (33550) contains the pixel size in map units,
    /// which is essential for converting between pixel and world coordinates.
    /// The tag typically contains 3 values: x_scale, y_scale, and z_scale.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing the ModelPixelScaleTag
    /// * `byte_order_handler` - Handler for the file's byte order
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// * `TiffResult<Vec<f64>>` - Vector of scale values [x_scale, y_scale, z_scale]
    pub fn read_model_pixel_scale_values(
        ifd: &IFD,
        byte_order_handler: &Box<dyn ByteOrderHandler>,
        file_path: &str
    ) -> TiffResult<Vec<f64>> {
        if let Some(entry) = ifd.get_entry(tags::MODEL_PIXEL_SCALE_TAG) {
            let file = File::open(file_path)?;
            let mut reader = file;
            reader.seek(SeekFrom::Start(entry.value_offset))?;

            let mut values = Vec::with_capacity(entry.count as usize);
            for _ in 0..entry.count {
                values.push(byte_order_handler.read_f64(&mut reader)?);
            }

            return Ok(values);
        }

        Err(TiffError::TagNotFound(tags::MODEL_PIXEL_SCALE_TAG))
    }

    /// Read model tiepoint values (i,j,k,x,y,z)
    ///
    /// ModelTiepointTag (33922) defines how raster coordinates map to world coordinates.
    /// Each tiepoint consists of 6 values: i,j,k (raster coordinates) and x,y,z (world coordinates).
    /// The tag can contain multiple tiepoints, each with 6 values.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing the ModelTiepointTag
    /// * `byte_order_handler` - Handler for the file's byte order
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// * `TiffResult<Vec<f64>>` - Vector of tiepoint values [i,j,k,x,y,z,...]
    pub fn read_model_tiepoint_values(
        ifd: &IFD,
        byte_order_handler: &Box<dyn ByteOrderHandler>,
        file_path: &str
    ) -> TiffResult<Vec<f64>> {
        if let Some(entry) = ifd.get_entry(tags::MODEL_TIEPOINT_TAG) {
            let file = File::open(file_path)?;
            let mut reader = file;
            reader.seek(SeekFrom::Start(entry.value_offset))?;

            let mut values = Vec::with_capacity(entry.count as usize);
            for _ in 0..entry.count {
                values.push(byte_order_handler.read_f64(&mut reader)?);
            }

            return Ok(values);
        }

        Err(TiffError::TagNotFound(tags::MODEL_TIEPOINT_TAG))
    }

    /// Extract geospatial information from a TIFF IFD
    ///
    /// Interprets all the GeoTIFF tags and keys to build a comprehensive
    /// GeoInfo structure containing projection information, pixel sizes,
    /// and georeferencing parameters.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to extract information from
    /// * `byte_order_handler` - Handler for the file's byte order
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// * `TiffResult<GeoInfo>` - Structure with extracted geospatial information
    pub fn extract_geo_info(
        ifd: &IFD,
        byte_order_handler: &Box<dyn ByteOrderHandler>,
        file_path: &str
    ) -> TiffResult<GeoInfo> {
        let mut geo_info = GeoInfo::new();

        // Extract projection information from GeoKeys
        let geo_keys = Self::parse_geo_key_directory(ifd, byte_order_handler, file_path)?;

        for key in &geo_keys {
            match key.key_id {
                geo_keys::PROJECTED_CS_TYPE => {
                    if key.tiff_tag_location == 0 {
                        geo_info.epsg_code = key.value_offset as u32;
                    }
                },
                geo_keys::PROJECTION => {
                    if key.tiff_tag_location == 0 {
                        let proj_code = key.value_offset as u16;
                        geo_info.projection_code = proj_code;
                    }
                },
                geo_keys::GEOGRAPHIC_TYPE => {
                    if key.tiff_tag_location == 0 {
                        geo_info.geographic_cs_code = key.value_offset as u32;
                    }
                },
                // Add more key interpretations as needed
                _ => {}
            }
        }

        // Try to get pixel scale
        if let Ok(pixel_scale) = Self::read_model_pixel_scale_values(ifd, byte_order_handler, file_path) {
            if pixel_scale.len() >= 2 {
                geo_info.pixel_size_x = pixel_scale[0];
                geo_info.pixel_size_y = pixel_scale[1];
            }
        }

        // Try to get tie points
        if let Ok(tie_points) = Self::read_model_tiepoint_values(ifd, byte_order_handler, file_path) {
            if tie_points.len() >= 6 {
                geo_info.tie_point = Some((
                    tie_points[0], tie_points[1], tie_points[2],  // i,j,k (raster coords)
                    tie_points[3], tie_points[4], tie_points[5]   // x,y,z (world coords)
                ));

                // If we have a tie point and pixel size, we can calculate the origin
                if geo_info.pixel_size_x != 0.0 && geo_info.pixel_size_y != 0.0 {
                    // Origin is at the top-left corner, but tie point might be elsewhere
                    // For a tie point at i,j with world x,y and pixel size dx,dy:
                    // origin_x = x - i * dx
                    // origin_y = y + j * dy (assuming y increases upward, TIFF has y increasing downward)
                    geo_info.origin_x = tie_points[3] - tie_points[0] * geo_info.pixel_size_x;
                    geo_info.origin_y = tie_points[4] + tie_points[1] * geo_info.pixel_size_y;
                }
            }
        }

        Ok(geo_info)
    }

    /// Format a human-readable GeoTIFF projection string
    ///
    /// Interprets the projection information in a GeoInfo structure
    /// and returns a human-readable description of the coordinate system.
    ///
    /// # Arguments
    /// * `geo_info` - The GeoInfo structure containing projection information
    ///
    /// # Returns
    /// * `String` - Human-readable description of the projection
    pub fn format_projection_string(geo_info: &GeoInfo) -> String {
        let mut projection = String::new();

        // First try to interpret the EPSG code
        if geo_info.epsg_code > 0 {
            // Use constants for the special cases rather than expressions in patterns
            const WGS84_WEB_MERCATOR_CODE: u32 = epsg::WGS84_WEB_MERCATOR as u32;
            const WGS84_CODE: u32 = epsg::WGS84 as u32;

            match geo_info.epsg_code {
                WGS84_WEB_MERCATOR_CODE => {
                    return "Web Mercator (EPSG:3857)".to_string();
                },
                WGS84_CODE => {
                    return "WGS84 Geographic (EPSG:4326)".to_string();
                },
                _ => {
                    projection = format!("EPSG:{}", geo_info.epsg_code);
                }
            }
        }
        // Otherwise try the projection code
        else if geo_info.projection_code > 0 {
            // Define constants for projection codes
            const LATLONG_CODE: u16 = proj_method::LATLONG;
            const MERCATOR_CODE: u16 = proj_method::MERCATOR;
            const STEREOGRAPHIC_CODE: u16 = proj_method::STEREOGRAPHIC;
            const TRANSVERSE_MERC_CODE: u16 = proj_method::TRANSVERSE_MERC;

            let proj_name = match geo_info.projection_code {
                LATLONG_CODE => "Geographic (lat/long)".to_string(),
                MERCATOR_CODE => "Mercator".to_string(),
                STEREOGRAPHIC_CODE => "Stereographic".to_string(),
                TRANSVERSE_MERC_CODE => "Transverse Mercator".to_string(),
                _ => format!("Projection code: {}", geo_info.projection_code),
            };
            projection = proj_name;
        }
        // Fallback to the geographic CS code
        else if geo_info.geographic_cs_code > 0 {
            projection = format!("Geographic CS: {}", geo_info.geographic_cs_code);
        }
        else {
            projection = "Unknown projection".to_string();
        }

        projection
    }

    /// Format GeoKeys for display
    ///
    /// Creates a vector of tuples containing all GeoKey information,
    /// formatted for display in a human-readable format.
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing the GeoKeys
    /// * `byte_order_handler` - Handler for the file's byte order
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// * `TiffResult<Vec<(u16, String, u16, u16, u16, String)>>` - Vector of tuples with
    ///   (key_id, key_name, tag_location, count, value_offset, value_string)
    pub fn format_geo_keys(
        ifd: &IFD,
        byte_order_handler: &Box<dyn ByteOrderHandler>,
        file_path: &str
    ) -> TiffResult<Vec<(u16, String, u16, u16, u16, String)>> {
        let geo_keys = Self::parse_geo_key_directory(ifd, byte_order_handler, file_path)?;
        let mut result = Vec::with_capacity(geo_keys.len());

        for key in &geo_keys {
            let key_name = get_key_name(key.key_id).to_string();
            let value_str = Self::get_geo_key_value_as_string(ifd, key, byte_order_handler, file_path)
                .unwrap_or_else(|_| "Unknown".to_string());

            result.push((
                key.key_id,
                key_name,
                key.tiff_tag_location,
                key.count,
                key.value_offset,
                value_str
            ));
        }

        Ok(result)
    }
}

/// Structure to hold geospatial information extracted from a GeoTIFF
pub struct GeoInfo {
    /// EPSG code for the coordinate reference system
    pub epsg_code: u32,
    /// GeoTIFF projection code
    pub projection_code: u16,
    /// Geographic coordinate system code
    pub geographic_cs_code: u32,
    /// Pixel size in X direction (in map units)
    pub pixel_size_x: f64,
    /// Pixel size in Y direction (in map units)
    pub pixel_size_y: f64,
    /// Origin X coordinate (top-left corner in map units)
    pub origin_x: f64,
    /// Origin Y coordinate (top-left corner in map units)
    pub origin_y: f64,
    /// Optional tie point (i,j,k,x,y,z)
    pub tie_point: Option<(f64, f64, f64, f64, f64, f64)>,
}

impl GeoInfo {
    /// Creates a new empty GeoInfo structure
    pub fn new() -> Self {
        GeoInfo {
            epsg_code: 0,
            projection_code: 0,
            geographic_cs_code: 0,
            pixel_size_x: 0.0,
            pixel_size_y: 0.0,
            origin_x: 0.0,
            origin_y: 0.0,
            tie_point: None,
        }
    }

    /// Check if the GeoInfo contains valid georeferencing information
    pub fn is_georeferenced(&self) -> bool {
        self.epsg_code > 0 ||
            self.projection_code > 0 ||
            self.geographic_cs_code > 0 ||
            (self.pixel_size_x != 0.0 && self.pixel_size_y != 0.0 && self.tie_point.is_some())
    }

    /// Get the bounds of the georeferenced image
    ///
    /// Returns (min_x, min_y, max_x, max_y) in world coordinates if
    /// we have enough information to calculate the bounds.
    pub fn get_bounds(&self, width: u32, height: u32) -> Option<(f64, f64, f64, f64)> {
        if self.pixel_size_x == 0.0 || self.pixel_size_y == 0.0 {
            return None;
        }

        let min_x = self.origin_x;
        let max_y = self.origin_y;
        let max_x = min_x + (width as f64) * self.pixel_size_x;
        let min_y = max_y - (height as f64) * self.pixel_size_y;

        Some((min_x, min_y, max_x, max_y))
    }
}