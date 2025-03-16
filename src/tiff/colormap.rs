//! TIFF colormap handling functionality
//!
//! This module provides utilities for working with color maps in TIFF files
//! and converting between TIFF color maps and various formats like SLD.

use std::fs::File;
use std::io::{self, BufReader, Read, Write, BufWriter, BufRead};
use std::path::Path;
use std::collections::HashMap;
use log::{debug, info, warn, error};

use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::{IFD, IFDEntry};
use crate::tiff::constants::{tags, photometric, field_types};
use crate::io::byte_order::ByteOrderHandler;
use crate::io::seekable::SeekableReader;
use crate::tiff::TiffReader;
use crate::tiff::TiffBuilder;
use crate::utils::logger::Logger;

/// Simple RGB color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RgbColor {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl RgbColor {
    /// Create a new RGB color
    ///
    /// # Arguments
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # Returns
    /// A new RgbColor instance
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RgbColor { r, g, b }
    }

    /// Convert to hex string (#RRGGBB format)
    ///
    /// # Returns
    /// A hex color string in the format #RRGGBB
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Create from hex string
    ///
    /// # Arguments
    /// * `hex` - Hex color string (with or without # prefix)
    ///
    /// # Returns
    /// A Result containing the parsed RgbColor or an error
    pub fn from_hex(hex: &str) -> TiffResult<Self> {
        // Remove # prefix if present
        let hex = hex.trim_start_matches('#');

        // Validate hex string length
        if hex.len() != 6 {
            return Err(TiffError::GenericError(
                format!("Invalid hex color code: {} - must be 6 hexadecimal digits", hex)
            ));
        }

        // Parse RGB components
        let r = parse_hex_component(&hex[0..2], hex)?;
        let g = parse_hex_component(&hex[2..4], hex)?;
        let b = parse_hex_component(&hex[4..6], hex)?;

        Ok(RgbColor { r, g, b })
    }
}

/// Helper function to parse a hex color component
fn parse_hex_component(hex_part: &str, full_hex: &str) -> TiffResult<u8> {
    u8::from_str_radix(hex_part, 16)
        .map_err(|_| TiffError::GenericError(format!("Invalid hex color: {}", full_hex)))
}

/// Represents a color map entry with a value and RGB color
#[derive(Debug, Clone)]
pub struct ColorMapEntry {
    /// The pixel value this entry applies to
    pub value: u16,
    /// Optional label for the entry
    pub label: Option<String>,
    /// The RGB color for this value
    pub color: RgbColor,
}

impl ColorMapEntry {
    /// Create a new color map entry
    ///
    /// # Arguments
    /// * `value` - The pixel value this entry applies to
    /// * `color` - The RGB color for this value
    ///
    /// # Returns
    /// A new ColorMapEntry instance
    pub fn new(value: u16, color: RgbColor) -> Self {
        ColorMapEntry {
            value,
            label: None,
            color
        }
    }

    /// Create a new color map entry with a label
    ///
    /// # Arguments
    /// * `value` - The pixel value this entry applies to
    /// * `color` - The RGB color for this value
    /// * `label` - A label for this entry
    ///
    /// # Returns
    /// A new ColorMapEntry instance with a label
    pub fn with_label(value: u16, color: RgbColor, label: String) -> Self {
        ColorMapEntry {
            value,
            label: Some(label),
            color,
        }
    }

    /// Get the RGB color in hex format (#RRGGBB)
    ///
    /// # Returns
    /// A hex color string
    pub fn to_hex_color(&self) -> String {
        self.color.to_hex()
    }

    /// Create a color map entry from a hex color string
    ///
    /// # Arguments
    /// * `value` - The pixel value this entry applies to
    /// * `hex` - Hex color string (with or without # prefix)
    /// * `label` - Optional label for this entry
    ///
    /// # Returns
    /// A Result containing the new ColorMapEntry or an error
    pub fn from_hex_color(value: u16, hex: &str, label: Option<String>) -> TiffResult<Self> {
        let color = RgbColor::from_hex(hex)?;

        Ok(ColorMapEntry {
            value,
            label,
            color,
        })
    }
}

/// Represents a color map from a TIFF file or other sources
#[derive(Debug, Clone)]
pub struct ColorMap {
    /// Vector of color map entries, sorted by value
    pub entries: Vec<ColorMapEntry>,
    /// Type of the color map ("values", "intervals", or "ramp")
    pub map_type: String,
}

impl ColorMap {
    /// Create a new empty color map
    ///
    /// # Returns
    /// A new empty ColorMap instance
    pub fn new() -> Self {
        ColorMap {
            entries: Vec::new(),
            map_type: "ramp".to_string(), // Default to ramp (interpolated)
        }
    }

    /// Add a new entry to the color map
    ///
    /// # Arguments
    /// * `entry` - The ColorMapEntry to add
    pub fn add_entry(&mut self, entry: ColorMapEntry) {
        self.entries.push(entry);
        // Keep entries sorted by value
        self.entries.sort_by_key(|e| e.value);
    }

    /// Set the color map type
    ///
    /// # Arguments
    /// * `map_type` - The type of color map ("values", "intervals", or "ramp")
    pub fn set_type(&mut self, map_type: &str) {
        self.map_type = map_type.to_string();
    }

    /// Get the number of entries in the color map
    ///
    /// # Returns
    /// The number of entries in the color map
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the color map is empty
    ///
    /// # Returns
    /// true if the color map has no entries, false otherwise
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Read a TIFF colormap from an IFD
    ///
    /// # Arguments
    /// * `ifd` - The IFD containing the colormap
    /// * `reader` - Reader for accessing the file
    /// * `byte_order_handler` - Handler for the file's byte order
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn from_tiff_ifd<R: SeekableReader>(
        ifd: &IFD,
        reader: &mut R,
        byte_order_handler: &Box<dyn ByteOrderHandler>
    ) -> TiffResult<Self> {
        debug!("Reading color map from TIFF IFD");

        // Verify it's a palette color image
        let photometric_interp = ifd.get_tag_value(tags::PHOTOMETRIC_INTERPRETATION)
            .unwrap_or(0);

        if photometric_interp != photometric::PALETTE as u64 {
            return Err(TiffError::GenericError(
                "IFD does not contain a color map (not a palette image)".to_string()
            ));
        }

        // Get the bits per sample to determine the number of color map entries
        let bits_per_sample = ifd.get_tag_value(tags::BITS_PER_SAMPLE)
            .unwrap_or(8) as u16;

        let num_entries = 1 << bits_per_sample; // 2^bits
        debug!("Color map should have {} entries ({}-bit)", num_entries, bits_per_sample);

        // Get the color map entry
        let colormap_entry = ifd.get_entry(tags::COLOR_MAP)
            .ok_or_else(|| TiffError::GenericError("No ColorMap tag found in IFD".to_string()))?;

        // The color map should have 3 * num_entries values (for R, G, B)
        if colormap_entry.count != (3 * num_entries as u64) {
            return Err(TiffError::GenericError(
                format!("ColorMap has wrong size: {} (expected {})",
                        colormap_entry.count, 3 * num_entries)
            ));
        }

        // Seek to where the colormap data is stored
        reader.seek(std::io::SeekFrom::Start(colormap_entry.value_offset))?;

        // Read the color map data
        let (r_values, g_values, b_values) = read_colormap_data(
            reader,
            byte_order_handler,
            num_entries
        )?;

        // Create a new color map
        let mut colormap = ColorMap::new();

        // Convert 16-bit values to 8-bit and add entries
        for i in 0..num_entries {
            // Convert 16-bit to 8-bit by dividing by 257 (approximately 65535/255)
            let r = (r_values[i as usize] / 257) as u8;
            let g = (g_values[i as usize] / 257) as u8;
            let b = (b_values[i as usize] / 257) as u8;

            colormap.add_entry(ColorMapEntry::new(i, RgbColor::new(r, g, b)));
        }

        // Remove entries that have pure black (0,0,0) at the beginning
        colormap.remove_empty_entries();

        // Simplify if needed
        colormap.simplify_if_needed();

        Ok(colormap)
    }

    /// Remove entries that have pure black (0,0,0) at the beginning
    fn remove_empty_entries(&mut self) {
        while !self.entries.is_empty() &&
            self.entries[0].color.r == 0 &&
            self.entries[0].color.g == 0 &&
            self.entries[0].color.b == 0 {
            self.entries.remove(0);
        }
    }

    /// Simplify the color map if it has too many entries
    fn simplify_if_needed(&mut self) {
        if self.entries.len() <= 256 {
            return;
        }

        info!("Simplifying large color map with {} entries", self.entries.len());
        let mut simplified = Vec::new();
        let mut seen_colors = HashMap::new();

        for entry in &self.entries {
            let color_key = (entry.color.r, entry.color.g, entry.color.b);
            if !seen_colors.contains_key(&color_key) {
                seen_colors.insert(color_key, true);
                simplified.push(entry.clone());
            }
        }

        self.entries = simplified;
        info!("Simplified to {} unique colors", self.entries.len());
    }

    /// Read a color map from an SLD XML file
    ///
    /// # Arguments
    /// * `file_path` - Path to the SLD file
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn from_sld_file<P: AsRef<Path>>(file_path: P) -> TiffResult<Self> {
        debug!("Reading color map from SLD file: {:?}", file_path.as_ref());

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        Self::from_sld_reader(reader)
    }

    /// Read a color map from a reader containing SLD XML content
    ///
    /// # Arguments
    /// * `reader` - Reader containing SLD XML content
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn from_sld_reader<R: Read>(mut reader: R) -> TiffResult<Self> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let mut colormap = ColorMap::new();
        colormap.set_type("ramp");  // Default type for SLD

        // Look for the type attribute in the ColorMap tag
        if let Some(map_type) = extract_colormap_type(&content) {
            colormap.set_type(&map_type);
        }

        // Parse each line containing ColorMapEntry
        for line in content.lines() {
            if line.contains("sld:ColorMapEntry") || line.contains("ColorMapEntry") {
                parse_sld_entry_attributes(&mut colormap, line);
            }
        }

        if colormap.is_empty() {
            return Err(TiffError::GenericError("No color map entries found in SLD file".to_string()));
        }

        debug!("Read {} entries from SLD", colormap.len());
        Ok(colormap)
    }

    /// Read a color map from a CSV file
    ///
    /// The CSV should have one of these formats:
    /// - value,r,g,b
    /// - value,hexcolor
    /// - value,hexcolor,label
    ///
    /// # Arguments
    /// * `file_path` - Path to the CSV file
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn from_csv_file<P: AsRef<Path>>(file_path: P) -> TiffResult<Self> {
        debug!("Reading color map from CSV file: {:?}", file_path.as_ref());

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        Self::from_csv_reader(reader)
    }

    /// Read a color map from a reader containing CSV content
    ///
    /// # Arguments
    /// * `reader` - Reader containing CSV content
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn from_csv_reader<R: Read>(mut reader: R) -> TiffResult<Self> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let mut colormap = ColorMap::new();

        for line in content.lines() {
            // Skip empty lines and comments
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

            // Try to parse the line based on the number of parts
            if let Some(entry) = parse_csv_line(&parts) {
                colormap.add_entry(entry);
            } else {
                warn!("Ignoring invalid CSV line: {}", line);
            }
        }

        if colormap.is_empty() {
            return Err(TiffError::GenericError("No valid color map entries found in CSV".to_string()));
        }

        debug!("Read {} entries from CSV", colormap.len());
        Ok(colormap)
    }

    /// Create a TIFF colormap suitable for writing to a file
    ///
    /// Converts the ColorMap structure to the raw data format required by TIFF.
    ///
    /// # Returns
    /// A tuple containing (num_entries, raw_data) where raw_data is the combined RGB data
    pub fn to_tiff_colormap(&self) -> (u16, Vec<u16>) {
        // Find the highest value in the color map to determine the size needed
        let max_value = self.entries.iter()
            .map(|e| e.value)
            .max()
            .unwrap_or(0);

        // The number of entries should be a power of 2 and at least cover the max value
        let bits_needed = ((max_value as f32).log2().ceil() as u32).max(1);
        let num_entries = 1 << bits_needed; // 2^bits

        debug!("Creating TIFF colormap with {} entries (using {} bits)",
              num_entries, bits_needed);

        // Create arrays for R, G, B values
        let mut r_values = vec![0u16; num_entries as usize];
        let mut g_values = vec![0u16; num_entries as usize];
        let mut b_values = vec![0u16; num_entries as usize];

        // Fill in the values from our color map
        for entry in &self.entries {
            let idx = entry.value as usize;
            if idx < num_entries as usize {
                // Convert 8-bit colors to 16-bit (multiply by 257)
                r_values[idx] = entry.color.r as u16 * 257;
                g_values[idx] = entry.color.g as u16 * 257;
                b_values[idx] = entry.color.b as u16 * 257;
            }
        }

        // If the color map is a ramp (interpolated), fill in any gaps
        if self.map_type == "ramp" && self.entries.len() > 1 {
            self.interpolate_ramp_values(&mut r_values, &mut g_values, &mut b_values, num_entries);
        }

        // Combine all values into a single vector in TIFF's expected order: all R, then all G, then all B
        let mut result = Vec::with_capacity(3 * num_entries as usize);
        result.extend_from_slice(&r_values);
        result.extend_from_slice(&g_values);
        result.extend_from_slice(&b_values);

        (num_entries as u16, result)
    }

    /// Interpolate missing values in a color ramp
    fn interpolate_ramp_values(
        &self,
        r_values: &mut [u16],
        g_values: &mut [u16],
        b_values: &mut [u16],
        num_entries: u32
    ) {
        debug!("Interpolating color ramp for missing values");

        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by_key(|e| e.value);

        // Interpolate colors between defined values
        for i in 1..sorted_entries.len() {
            let prev = &sorted_entries[i-1];
            let curr = &sorted_entries[i];

            if curr.value <= prev.value + 1 {
                continue; // No gap to interpolate
            }

            // We have a gap to interpolate
            let gap_size = (curr.value - prev.value) as f32;

            for j in 1..curr.value - prev.value {
                let t = j as f32 / gap_size;
                let idx = (prev.value + j) as usize;

                if idx >= num_entries as usize {
                    continue; // Skip indices beyond our array size
                }

                // Linear interpolation between colors
                r_values[idx] = interpolate_color_component(prev.color.r, curr.color.r, t);
                g_values[idx] = interpolate_color_component(prev.color.g, curr.color.g, t);
                b_values[idx] = interpolate_color_component(prev.color.b, curr.color.b, t);
            }
        }
    }

    /// Write the color map to an SLD file
    ///
    /// # Arguments
    /// * `file_path` - Path to write the SLD file
    /// * `layer_name` - Name to use for the layer in the SLD
    ///
    /// # Returns
    /// A Result indicating success or an error
    pub fn to_sld_file<P: AsRef<Path>>(&self, file_path: P, layer_name: &str) -> TiffResult<()> {
        debug!("Writing color map to SLD file: {:?}", file_path.as_ref());

        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);

        // Write the SLD file manually since we're avoiding external XML libs for simplicity
        writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(writer, "<StyledLayerDescriptor xmlns=\"http://www.opengis.net/sld\" version=\"1.0.0\" xmlns:gml=\"http://www.opengis.net/gml\" xmlns:sld=\"http://www.opengis.net/sld\" xmlns:ogc=\"http://www.opengis.net/ogc\">")?;
        writeln!(writer, "  <UserLayer>")?;
        writeln!(writer, "    <sld:LayerFeatureConstraints>")?;
        writeln!(writer, "      <sld:FeatureTypeConstraint/>")?;
        writeln!(writer, "    </sld:LayerFeatureConstraints>")?;
        writeln!(writer, "    <sld:UserStyle>")?;
        writeln!(writer, "      <sld:Name>{}</sld:Name>", escape_xml(layer_name))?;
        writeln!(writer, "      <sld:FeatureTypeStyle>")?;
        writeln!(writer, "        <sld:Rule>")?;
        writeln!(writer, "          <sld:RasterSymbolizer>")?;
        writeln!(writer, "            <sld:ChannelSelection>")?;
        writeln!(writer, "              <sld:GrayChannel>")?;
        writeln!(writer, "                <sld:SourceChannelName>1</sld:SourceChannelName>")?;
        writeln!(writer, "              </sld:GrayChannel>")?;
        writeln!(writer, "            </sld:ChannelSelection>")?;
        writeln!(writer, "            <sld:ColorMap type=\"{}\">", self.map_type)?;

        // Write each color map entry
        for entry in &self.entries {
            let label = entry.label.as_ref().map_or_else(
                || format!("{:.4}", entry.value),
                |s| s.clone()
            );

            writeln!(writer, "              <sld:ColorMapEntry quantity=\"{}\" label=\"{}\" color=\"{}\"/>",
                     entry.value, escape_xml(&label), entry.to_hex_color())?;
        }

        writeln!(writer, "            </sld:ColorMap>")?;
        writeln!(writer, "          </sld:RasterSymbolizer>")?;
        writeln!(writer, "        </sld:Rule>")?;
        writeln!(writer, "      </sld:FeatureTypeStyle>")?;
        writeln!(writer, "    </sld:UserStyle>")?;
        writeln!(writer, "  </UserLayer>")?;
        writeln!(writer, "</StyledLayerDescriptor>")?;

        Ok(())
    }

    /// Print the color map to stdout in a human-readable format
    pub fn print(&self) {
        println!("Color Map with {} entries (type: {}):", self.entries.len(), self.map_type);
        println!("{:^8} {:^20} {:^10}", "Value", "Color (RGB)", "Label");
        println!("{:-^8} {:-^20} {:-^10}", "", "", "");

        for entry in &self.entries {
            println!("{:^8} {:^20} {:^10}",
                     entry.value,
                     format!("({},{},{}) {}",
                             entry.color.r,
                             entry.color.g,
                             entry.color.b,
                             entry.to_hex_color()
                     ),
                     entry.label.as_deref().unwrap_or("")
            );
        }
    }

    /// Apply the colormap to a TiffBuilder
    ///
    /// This adds the appropriate tags and data to make the output TIFF use this colormap.
    ///
    /// # Arguments
    /// * `builder` - The TiffBuilder to modify
    /// * `ifd_index` - Index of the IFD to add the colormap to
    ///
    /// # Returns
    /// A Result indicating success or an error
    pub fn apply_to_builder(&self, builder: &mut TiffBuilder, ifd_index: usize) -> TiffResult<()> {
        if ifd_index >= builder.ifds.len() {
            return Err(TiffError::GenericError(format!(
                "Invalid IFD index {}, only have {} IFDs", ifd_index, builder.ifds.len())));
        }

        // Convert the colormap to TIFF format
        let (num_entries, colormap_data) = self.to_tiff_colormap();

        // Calculate bits needed for the colormap
        let bits_needed = match num_entries {
            0..=2 => 1,
            3..=4 => 2,
            5..=16 => 4,
            17..=256 => 8,
            _ => 16,
        };

        debug!("Setting up colormap with {} entries, {} bits per pixel", num_entries, bits_needed);

        // Create vector of bytes for colormap data
        let mut bytes = Vec::with_capacity(colormap_data.len() * 2);
        for value in &colormap_data {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        // Set the colormap tag (external data)
        builder.set_external_data(ifd_index, tags::COLOR_MAP, bytes);

        // Set the photo interpretation to palette color
        builder.ifds[ifd_index].add_entry(IFDEntry::new(
            tags::PHOTOMETRIC_INTERPRETATION,
            field_types::SHORT,
            1,
            photometric::PALETTE as u64
        ));

        // Set the bits per sample (we need at least enough bits to address all colors)
        builder.ifds[ifd_index].add_entry(IFDEntry::new(
            tags::BITS_PER_SAMPLE,
            field_types::SHORT,
            1,
            bits_needed as u64
        ));

        // Set samples per pixel to 1 for palette images
        builder.ifds[ifd_index].add_entry(IFDEntry::new(
            tags::SAMPLES_PER_PIXEL,
            field_types::SHORT,
            1,
            1
        ));

        Ok(())
    }
}

/// ColorMap reader for handling various formats
pub struct ColorMapReader<'a> {
    /// Logger for recording operations
    logger: &'a Logger,
}

impl<'a> ColorMapReader<'a> {
    /// Create a new ColorMapReader
    ///
    /// # Arguments
    /// * `logger` - Logger for recording operations
    ///
    /// # Returns
    /// A new ColorMapReader instance
    pub fn new(logger: &'a Logger) -> Self {
        ColorMapReader {
            logger
        }
    }

    /// Read a color map from a file based on its extension
    ///
    /// # Arguments
    /// * `file_path` - Path to the colormap file
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn read_file(&self, file_path: &str) -> TiffResult<ColorMap> {
        info!("Reading color map from file: {}", file_path);

        let extension = match std::path::Path::new(file_path).extension() {
            Some(ext) => ext.to_string_lossy().to_lowercase(),
            None => "".to_string()
        };

        match extension.as_str() {
            "sld" => {
                debug!("Detected SLD format");
                ColorMap::from_sld_file(file_path)
            },
            "csv" | "txt" => {
                debug!("Detected CSV format");
                ColorMap::from_csv_file(file_path)
            },
            "tif" | "tiff" => {
                debug!("Detected TIFF format");
                self.read_from_tiff(file_path)
            },
            _ => {
                // Try to guess from content
                self.guess_format(file_path)
            }
        }
    }

    /// Read a color map from a TIFF file
    ///
    /// # Arguments
    /// * `file_path` - Path to the TIFF file
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    pub fn read_from_tiff(&self, file_path: &str) -> TiffResult<ColorMap> {
        info!("Reading color map from TIFF file: {}", file_path);

        // Create TIFF reader
        let mut reader = TiffReader::new(self.logger);
        let tiff = reader.load(file_path)?;

        if tiff.ifds.is_empty() {
            return Err(TiffError::GenericError("No IFDs found in TIFF file".to_string()));
        }

        // Use the first IFD
        let ifd = &tiff.ifds[0];

        // Need a file reader to access the actual color map data
        let mut file_reader = reader.create_reader()?;

        // Get byte order handler
        let byte_order_handler = reader.get_byte_order_handler()
            .ok_or_else(|| TiffError::GenericError("No byte order handler available".to_string()))?;

        // Extract the color map
        let colormap = ColorMap::from_tiff_ifd(ifd, &mut file_reader, byte_order_handler)?;

        info!("Successfully read color map with {} entries from TIFF", colormap.len());
        self.logger.log(&format!("Read color map with {} entries from {}", colormap.len(), file_path))?;

        Ok(colormap)
    }

    /// Try to guess the format of a color map file from its content
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    ///
    /// # Returns
    /// A Result containing the ColorMap or an error
    fn guess_format(&self, file_path: &str) -> TiffResult<ColorMap> {
        info!("Attempting to guess color map format for: {}", file_path);

        // Read first few lines to check content
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();

        for line in reader.lines().take(10) {
            if let Ok(line) = line {
                lines.push(line);
            }
        }

        // Check if it might be an SLD (XML format)
        let looks_like_xml = lines.iter()
            .any(|line| line.contains("<?xml") || line.contains("<StyledLayerDescriptor"));

        if looks_like_xml {
            debug!("Content appears to be XML/SLD format");
            return ColorMap::from_sld_file(file_path);
        }

        // Check if it might be CSV format (comma-separated values)
        let looks_like_csv = lines.iter()
            .any(|line| line.contains(',') && !line.contains('<') && !line.contains('>'));

        if looks_like_csv {
            debug!("Content appears to be CSV format");
            return ColorMap::from_csv_file(file_path);
        }

        // Default to trying CSV as it's simplest
        warn!("Could not determine format, trying CSV as fallback");
        ColorMap::from_csv_file(file_path)
    }
}

/// Read color map data from the reader
fn read_colormap_data<R: SeekableReader>(
    reader: &mut R,
    byte_order_handler: &Box<dyn ByteOrderHandler>,
    num_entries: u16
) -> TiffResult<(Vec<u16>, Vec<u16>, Vec<u16>)> {
    // Read the color map data
    let mut r_values = Vec::with_capacity(num_entries as usize);
    let mut g_values = Vec::with_capacity(num_entries as usize);
    let mut b_values = Vec::with_capacity(num_entries as usize);

    // TIFF colormaps are stored as all red values, then all green values, then all blue values
    for _ in 0..num_entries {
        r_values.push(byte_order_handler.read_u16(reader)?);
    }

    for _ in 0..num_entries {
        g_values.push(byte_order_handler.read_u16(reader)?);
    }

    for _ in 0..num_entries {
        b_values.push(byte_order_handler.read_u16(reader)?);
    }

    Ok((r_values, g_values, b_values))
}

/// Parse a single ColorMapEntry from SLD attributes
fn parse_sld_entry_attributes(colormap: &mut ColorMap, line: &str) {
    // Extract required attributes, returning early if any are missing
    let quantity = match extract_attribute(line, "quantity") {
        Some(qty) => qty,
        None => return, // Missing quantity attribute, skip this entry
    };

    let color_hex = match extract_attribute(line, "color") {
        Some(clr) => clr,
        None => return, // Missing color attribute, skip this entry
    };

    // Parse the quantity value
    let value = match quantity.parse::<f64>() {
        Ok(val) => val as u16,
        Err(_) => return, // Invalid quantity value, skip this entry
    };

    // Parse the color
    let rgb_color = match RgbColor::from_hex(&color_hex) {
        Ok(clr) => clr,
        Err(_) => return, // Invalid color hex code, skip this entry
    };

    // Get optional label
    let label = extract_attribute(line, "label");

    // Create and add the entry
    let entry = ColorMapEntry {
        value,
        label,
        color: rgb_color
    };

    colormap.add_entry(entry);
}

/// Parse a CSV line into a ColorMapEntry
fn parse_csv_line(parts: &[&str]) -> Option<ColorMapEntry> {
    match parts.len() {
        2 => parse_csv_value_hex(parts),
        3 => parse_csv_three_parts(parts),
        4 => parse_csv_value_rgb(parts),
        5 => parse_csv_value_rgb_label(parts),
        _ => None,
    }
}

/// Parse a CSV line with format: value,hexcolor
fn parse_csv_value_hex(parts: &[&str]) -> Option<ColorMapEntry> {
    let value = parts[0].parse::<f64>().ok()?;
    let color = RgbColor::from_hex(parts[1]).ok()?;

    Some(ColorMapEntry::new(value as u16, color))
}

/// Parse a CSV line with 3 parts
fn parse_csv_three_parts(parts: &[&str]) -> Option<ColorMapEntry> {
    let value = parts[0].parse::<f64>().ok()?;

    // Try to parse as hexcolor,label
    if let Ok(color) = RgbColor::from_hex(parts[1]) {
        return Some(ColorMapEntry::with_label(
            value as u16, color, parts[2].to_string()
        ));
    }

    // Try to parse as r,g
    let r = parts[1].parse::<u8>().ok()?;
    let g = parts[2].parse::<u8>().ok()?;

    Some(ColorMapEntry::new(value as u16, RgbColor::new(r, g, 0)))
}

/// Parse a CSV line with format: value,r,g,b
fn parse_csv_value_rgb(parts: &[&str]) -> Option<ColorMapEntry> {
    let value = parts[0].parse::<f64>().ok()?;
    let r = parts[1].parse::<u8>().ok()?;
    let g = parts[2].parse::<u8>().ok()?;
    let b = parts[3].parse::<u8>().ok()?;

    Some(ColorMapEntry::new(value as u16, RgbColor::new(r, g, b)))
}

/// Parse a CSV line with format: value,r,g,b,label
fn parse_csv_value_rgb_label(parts: &[&str]) -> Option<ColorMapEntry> {
    let value = parts[0].parse::<f64>().ok()?;
    let r = parts[1].parse::<u8>().ok()?;
    let g = parts[2].parse::<u8>().ok()?;
    let b = parts[3].parse::<u8>().ok()?;

    Some(ColorMapEntry::with_label(
        value as u16, RgbColor::new(r, g, b), parts[4].to_string()
    ))
}

/// Helper function to interpolate between color components
fn interpolate_color_component(start: u8, end: u8, t: f32) -> u16 {
    ((start as f32 * (1.0 - t) + end as f32 * t) as u16 * 257)
}

/// Helper function to extract an attribute value from an XML element string
///
/// # Arguments
/// * `line` - The XML element string
/// * `attr_name` - The name of the attribute to extract
///
/// # Returns
/// The attribute value, or None if not found
fn extract_attribute(line: &str, attr_name: &str) -> Option<String> {
    let attr_pattern = format!("{}=\"", attr_name);

    if let Some(start_pos) = line.find(&attr_pattern) {
        let start_val = start_pos + attr_pattern.len();
        if let Some(end_pos) = line[start_val..].find('"') {
            return Some(line[start_val..(start_val + end_pos)].to_string());
        }
    }

    None
}

/// Helper function to extract the ColorMap type from SLD content
///
/// # Arguments
/// * `content` - The full SLD XML content
///
/// # Returns
/// The type attribute value, or None if not found
fn extract_colormap_type(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.contains("sld:ColorMap") || line.contains("ColorMap") {
            return extract_attribute(line, "type");
        }
    }

    None
}

/// Helper function to escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}