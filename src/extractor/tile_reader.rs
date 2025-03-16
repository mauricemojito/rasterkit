//! Tile-based image data extraction
//!
//! This module implements a reader for extracting image data from tiled TIFF files.
//! Tiled TIFF files organize image data in rectangular tiles of equal size, which
//! allows for more efficient access to specific regions of large images without
//! reading the entire file.

use log::{debug, info, warn};
use std::io::SeekFrom;
use image::{ImageBuffer, Rgb};

use crate::io::seekable::SeekableReader;
use crate::tiff::{TiffReader, TiffError};
use crate::tiff::errors::TiffResult;
use crate::tiff::ifd::IFD;
use crate::tiff::constants::{tags, predictor as pred_consts};
use crate::compression::CompressionFactory;
use crate::utils::image_extraction_utils;

use super::region::Region;

/// Reads image data from tiled TIFF files
///
/// This reader handles the extraction of pixel data from TIFFs that use
/// the tiled data organization, including handling of various compression
/// methods and coordinate mapping.
pub struct TileReader<'a, R: SeekableReader> {
    /// Reader for accessing the TIFF file
    reader: R,
    /// IFD containing the image metadata
    ifd: &'a IFD,
    /// TIFF reader for accessing tag values
    tiff_reader: &'a TiffReader<'a>,
}

impl<'a, R: SeekableReader> TileReader<'a, R> {
    /// Create a new tile reader
    ///
    /// # Arguments
    /// * `reader` - Seekable reader for the TIFF file
    /// * `ifd` - IFD containing the image metadata
    /// * `tiff_reader` - TIFF reader for accessing tag values
    ///
    /// # Returns
    /// A new TileReader instance
    pub fn new(reader: R, ifd: &'a IFD, tiff_reader: &'a TiffReader<'a>) -> Self {
        TileReader {
            reader,
            ifd,
            tiff_reader
        }
    }

    /// Get tile dimensions from the IFD
    ///
    /// Reads the tile width and height from the IFD, or uses default values
    /// of 256x256 if these tags are not present.
    ///
    /// # Returns
    /// Tile width and height in pixels as a tuple
    fn get_tile_dimensions(&self) -> (u32, u32) {
        let tile_width = self.ifd.get_tag_value(tags::TILE_WIDTH)
            .unwrap_or(256) as u32;
        let tile_height = self.ifd.get_tag_value(tags::TILE_LENGTH)
            .unwrap_or(256) as u32;

        (tile_width, tile_height)
    }

    /// Read a single tile from the TIFF file
    ///
    /// Reads and decompresses a tile from the TIFF file, applying
    /// the appropriate predictor if needed.
    ///
    /// # Arguments
    /// * `offset` - File offset where the tile data starts
    /// * `byte_count` - Size of the tile data in bytes
    /// * `compression_handler` - Handler for the compression method used
    /// * `predictor` - Predictor used for the image data
    /// * `tile_width` - Width of the tile in pixels
    /// * `tile_height` - Height of the tile in pixels
    ///
    /// # Returns
    /// Tile data as a byte vector, or an error
    fn read_tile(
        &mut self,
        offset: u64,
        byte_count: u64,
        compression_handler: &dyn crate::compression::CompressionHandler,
        predictor: usize,
        tile_width: usize,
        tile_height: usize
    ) -> TiffResult<Vec<u8>> {
        // Read the compressed tile data
        self.reader.seek(SeekFrom::Start(offset))?;
        let mut compressed_data = vec![0u8; byte_count as usize];
        self.reader.read_exact(&mut compressed_data)?;

        // Decompress the tile data
        let mut tile_data = compression_handler.decompress(&compressed_data)?;

        // Apply predictor if needed
        if predictor == pred_consts::HORIZONTAL_DIFFERENCING as usize {
            image_extraction_utils::apply_horizontal_predictor(&mut tile_data, tile_width, tile_height);
        }

        Ok(tile_data)
    }

    /// Extract image data to the provided buffer
    ///
    /// Reads all tiles that intersect with the specified region and
    /// copies their pixel data to the output image.
    ///
    /// # Arguments
    /// * `image` - Output image buffer
    /// * `region` - Region of the image to extract
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn extract(
        &mut self,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        region: Region
    ) -> TiffResult<()> {
        // Get tile dimensions
        let (tile_width, tile_height) = self.get_tile_dimensions();
        info!("Tile dimensions: {}x{}", tile_width, tile_height);

        // Get compression type
        let compression = self.ifd.get_tag_value(tags::COMPRESSION).unwrap_or(1);
        let compression_handler = CompressionFactory::create_handler(compression)?;
        info!("Using compression: {}", compression_handler.name());

        // Get predictor
        let predictor = self.ifd.get_tag_value(tags::PREDICTOR).unwrap_or(1) as usize;

        // Read tile offsets and byte counts
        let tile_offsets = self.tiff_reader.read_tag_values(&mut self.reader, self.ifd, tags::TILE_OFFSETS)?;
        let tile_byte_counts = self.tiff_reader.read_tag_values(&mut self.reader, self.ifd, tags::TILE_BYTE_COUNTS)?;

        // Calculate tile layout
        let (img_width, img_height) = self.ifd.get_dimensions()
            .ok_or_else(|| TiffError::GenericError("Missing image dimensions".to_string()))?;

        let tiles_across = (img_width as u32 + tile_width - 1) / tile_width;

        // Determine which tiles intersect with our region
        let start_tile_x = region.x / tile_width;
        let start_tile_y = region.y / tile_height;
        let end_tile_x = (region.end_x() + tile_width - 1) / tile_width;
        let end_tile_y = (region.end_y() + tile_height - 1) / tile_height;

        info!("Processing tiles from ({},{}) to ({},{})",
              start_tile_x, start_tile_y, end_tile_x - 1, end_tile_y - 1);

        // Process each tile
        for tile_y in start_tile_y..end_tile_y {
            for tile_x in start_tile_x..end_tile_x {
                let tile_index = (tile_y * tiles_across + tile_x) as usize;

                // Skip if tile index is out of bounds
                if tile_index >= tile_offsets.len() {
                    warn!("Tile index {} out of bounds (max {})",
                          tile_index, tile_offsets.len() - 1);
                    continue;
                }

                let offset = tile_offsets[tile_index];
                let byte_count = tile_byte_counts[tile_index];

                debug!("Reading tile ({},{}) at offset {} with {} bytes",
                       tile_x, tile_y, offset, byte_count);

                // Read and process the tile data
                let tile_data = match self.read_tile(
                    offset,
                    byte_count,
                    &*compression_handler,
                    predictor,
                    tile_width as usize,
                    tile_height as usize
                ) {
                    Ok(data) => data,
                    Err(e) => {
                        warn!("Error reading tile ({},{}): {:?}", tile_x, tile_y, e);
                        continue;
                    }
                };

                // Calculate tile position in pixels
                let tile_start_x = tile_x * tile_width;
                let tile_start_y = tile_y * tile_height;

                // Copy pixel data to image buffer
                self.copy_tile_to_image(
                    &tile_data,
                    image,
                    tile_width as usize,
                    tile_height as usize,
                    tile_start_x,
                    tile_start_y,
                    region
                );
            }
        }

        Ok(())
    }

    /// Copy tile data to the image buffer
    ///
    /// Maps pixels from the tile to the appropriate positions in the output image,
    /// handling coordinate conversions and boundary checks.
    ///
    /// # Arguments
    /// * `tile_data` - Decompressed tile data
    /// * `image` - Output image buffer
    /// * `tile_width` - Width of the tile in pixels
    /// * `tile_height` - Height of the tile in pixels
    /// * `tile_start_x` - X coordinate of the tile's top-left corner
    /// * `tile_start_y` - Y coordinate of the tile's top-left corner
    /// * `region` - Region being extracted
    fn copy_tile_to_image(
        &self,
        tile_data: &[u8],
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        tile_width: usize,
        tile_height: usize,
        tile_start_x: u32,
        tile_start_y: u32,
        region: Region
    ) {
        // For each row in the tile
        for y in 0..tile_height {
            let global_y = tile_start_y + y as u32;

            // Skip rows outside our region
            if global_y < region.y || global_y >= region.end_y() {
                continue;
            }

            // For each pixel in the row
            for x in 0..tile_width {
                let global_x = tile_start_x + x as u32;
                let tile_idx = y * tile_width + x;

                // Copy the pixel using the utility function
                image_extraction_utils::copy_pixel(
                    tile_data,
                    image,
                    global_x,
                    global_y,
                    tile_idx,
                    region
                );
            }
        }
    }
}