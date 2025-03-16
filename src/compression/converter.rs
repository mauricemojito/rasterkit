//! Compression conversion functionality

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write, Seek, SeekFrom};
use log::info;

use crate::tiff::TiffReader;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::utils::logger::Logger;
use super::factory::CompressionFactory;
use super::handler::CompressionHandler;

/// Converter for changing compression formats
pub struct CompressionConverter<'a> {
    logger: &'a Logger,
    reader: TiffReader<'a>,
}

impl<'a> CompressionConverter<'a> {
    /// Create a new compression converter
    pub fn new(logger: &'a Logger) -> Self {
        CompressionConverter {
            logger,
            reader: TiffReader::new(logger),
        }
    }

    /// Convert a single data block between compression formats
    pub fn convert_data(&self, data: &[u8],
                        source_compression: u64,
                        target_compression: u64) -> TiffResult<Vec<u8>> {
        // Get handlers for source and target compression
        let source_handler = CompressionFactory::create_handler(source_compression)?;
        let target_handler = CompressionFactory::create_handler(target_compression)?;

        info!("Converting data from {} to {} compression",
              source_handler.name(), target_handler.name());

        // Decompress with source handler
        let decompressed = source_handler.decompress(data)?;

        // Compress with target handler
        let recompressed = target_handler.compress(&decompressed)?;

        Ok(recompressed)
    }

    /// Convert a TIFF file from one compression format to another
    pub fn convert_file(&mut self, input_path: &str, output_path: &str,
                        target_compression: u64) -> TiffResult<()> {
        // Get target compression handler
        let target_handler = CompressionFactory::create_handler(target_compression)?;
        info!("Converting file {} to {} with {} compression",
          input_path, output_path, target_handler.name());

        // Load the source TIFF file to get its structure
        let source_tiff = self.reader.load(input_path)?;

        if source_tiff.ifds.is_empty() {
            return Err(TiffError::GenericError("No IFDs found in TIFF file".to_string()));
        }

        // Open the source file for reading binary data
        let source_file = File::open(input_path)?;
        let mut source_reader = BufReader::with_capacity(1024 * 1024, source_file);

        // Create the output file
        let output_file = File::create(output_path)?;
        let mut output_writer = BufWriter::with_capacity(1024 * 1024, output_file);

        // Write TIFF header
        self.write_tiff_header(&mut output_writer, source_tiff.is_big_tiff)?;

        // Keep track of the current write position
        let mut current_offset = if source_tiff.is_big_tiff { 16 } else { 8 };

        // Position to write the first IFD offset (we'll come back to this)
        let first_ifd_offset_pos = if source_tiff.is_big_tiff { 8 } else { 4 };

        // IFD chain information
        let mut ifd_offsets = Vec::new();
        let mut updated_ifds = Vec::new();

        // Create a multi-progress display
        let multi_progress = indicatif::MultiProgress::new();

        // Create the main progress bar for IFDs
        let ifd_progress = multi_progress.add(indicatif::ProgressBar::new(source_tiff.ifds.len() as u64));
        ifd_progress.set_style(indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) Processing IFDs")
            .unwrap()
            .progress_chars("#>-"));

        // Process each IFD
        for (ifd_index, ifd) in source_tiff.ifds.iter().enumerate() {
            info!("Processing IFD {} of {}", ifd_index + 1, source_tiff.ifds.len());

            // Update the progress bar
            ifd_progress.inc(1);
            ifd_progress.set_message(format!("IFD {} of {}", ifd_index + 1, source_tiff.ifds.len()));

            // Get the original compression type
            let source_compression = ifd.get_tag_value(259).unwrap_or(1);
            let source_handler = CompressionFactory::create_handler(source_compression)?;

            info!("Converting from {} to {} compression",
              source_handler.name(), target_handler.name());

            // Create a new IFD that will hold updated entries
            let mut new_ifd = ifd.clone();

            // Record the new IFD offset
            ifd_offsets.push(current_offset);

            // We'll update this offset after we process all IFDs
            current_offset += self.calculate_ifd_size(&new_ifd, source_tiff.is_big_tiff);

            // Process strips or tiles
            if ifd.has_tag(322) && ifd.has_tag(323) {
                // Tiled image
                self.process_tiles(&mut source_reader, &mut output_writer, ifd,
                                   source_compression, target_compression,
                                   &mut new_ifd, &mut current_offset, &multi_progress)?;
            } else {
                // Stripped image
                self.process_strips(&mut source_reader, &mut output_writer, ifd,
                                    source_compression, target_compression,
                                    &mut new_ifd, &mut current_offset, &multi_progress)?;
            }

            // Update the compression tag to the new compression type
            for entry in &mut new_ifd.entries {
                if entry.tag == 259 { // Compression tag
                    entry.value_offset = target_compression;
                    break;
                }
            }

            // If there's no compression tag, add one
            if !new_ifd.has_tag(259) {
                let compression_entry = crate::tiff::ifd::IFDEntry::new(
                    259, 3, 1, target_compression);
                new_ifd.add_entry(compression_entry);
            }

            updated_ifds.push(new_ifd);
        }

        // Mark IFD processing as complete
        ifd_progress.finish_with_message("IFD processing complete");

        // Create progress bar for writing IFDs
        let write_progress = multi_progress.add(indicatif::ProgressBar::new(updated_ifds.len() as u64));
        write_progress.set_style(indicatif::ProgressStyle::default_bar()
            .template("{spinner:.blue} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) Writing IFDs")
            .unwrap()
            .progress_chars("#>-"));

        // Now write the updated IFDs to the output file
        for (i, (ifd, offset)) in updated_ifds.iter().zip(ifd_offsets.iter()).enumerate() {
            // Seek to the IFD position
            output_writer.seek(SeekFrom::Start(*offset))?;

            // Write the IFD
            self.write_ifd(&mut output_writer, ifd, source_tiff.is_big_tiff,
                           if i < updated_ifds.len() - 1 {
                               Some(ifd_offsets[i + 1])
                           } else {
                               None
                           })?;

            // Update progress
            write_progress.inc(1);
            write_progress.set_message(format!("Writing IFD {} of {}", i + 1, updated_ifds.len()));
        }

        write_progress.finish_with_message("IFD writing complete");

        // Go back and write the first IFD offset
        output_writer.seek(SeekFrom::Start(first_ifd_offset_pos))?;
        if source_tiff.is_big_tiff {
            output_writer.write_all(&ifd_offsets[0].to_le_bytes())?;
        } else {
            output_writer.write_all(&(ifd_offsets[0] as u32).to_le_bytes())?;
        }

        // Ensure all data is written
        output_writer.flush()?;

        info!("Successfully converted TIFF file to {} compression",
          target_handler.name());

        Ok(())
    }

    // Helper method to write a TIFF header
    fn write_tiff_header(&self, writer: &mut impl Write, is_big_tiff: bool) -> TiffResult<()> {
        // Write byte order (Little Endian for now)
        writer.write_all(&[0x49, 0x49])?; // "II"

        if is_big_tiff {
            // BigTIFF header
            writer.write_all(&[43, 0])?;  // Version 43
            writer.write_all(&[8, 0])?;   // Offset size
            writer.write_all(&[0, 0])?;   // Reserved
            // First IFD offset will be filled in later
            writer.write_all(&[0, 0, 0, 0, 0, 0, 0, 0])?;
        } else {
            // Standard TIFF header
            writer.write_all(&[42, 0])?;  // Version 42
            // First IFD offset will be filled in later
            writer.write_all(&[0, 0, 0, 0])?;
        }

        Ok(())
    }

    // Helper method to calculate IFD size
    fn calculate_ifd_size(&self, ifd: &crate::tiff::ifd::IFD, is_big_tiff: bool) -> u64 {
        if is_big_tiff {
            // IFD entry count (8 bytes) + entries (20 bytes each) + next IFD offset (8 bytes)
            8 + (20 * ifd.entries.len() as u64) + 8
        } else {
            // IFD entry count (2 bytes) + entries (12 bytes each) + next IFD offset (4 bytes)
            2 + (12 * ifd.entries.len() as u64) + 4
        }
    }

    // Helper method to write an IFD
    fn write_ifd(&self, writer: &mut impl Write, ifd: &crate::tiff::ifd::IFD,
                 is_big_tiff: bool, next_ifd_offset: Option<u64>) -> TiffResult<()> {
        // Write entry count
        if is_big_tiff {
            writer.write_all(&(ifd.entries.len() as u64).to_le_bytes())?;
        } else {
            writer.write_all(&(ifd.entries.len() as u16).to_le_bytes())?;
        }

        // Write each entry
        for entry in &ifd.entries {
            // Tag
            writer.write_all(&entry.tag.to_le_bytes())?;
            // Type
            writer.write_all(&entry.field_type.to_le_bytes())?;
            // Count
            if is_big_tiff {
                writer.write_all(&entry.count.to_le_bytes())?;
            } else {
                writer.write_all(&(entry.count as u32).to_le_bytes())?;
            }
            // Value/Offset
            if is_big_tiff {
                writer.write_all(&entry.value_offset.to_le_bytes())?;
            } else {
                writer.write_all(&(entry.value_offset as u32).to_le_bytes())?;
            }
        }

        // Write next IFD offset
        let next_offset = next_ifd_offset.unwrap_or(0);
        if is_big_tiff {
            writer.write_all(&next_offset.to_le_bytes())?;
        } else {
            writer.write_all(&(next_offset as u32).to_le_bytes())?;
        }

        Ok(())
    }

    // Process strips in a TIFF file
    fn process_strips(&self, reader: &mut (impl Read + Seek + Send + Sync),
                      writer: &mut (impl Write + Seek + Send + Sync),
                      ifd: &crate::tiff::ifd::IFD,
                      source_compression: u64,
                      target_compression: u64,
                      new_ifd: &mut crate::tiff::ifd::IFD,
                      current_offset: &mut u64,
                      multi_progress: &indicatif::MultiProgress) -> TiffResult<()> {
        // Get strip offsets and byte counts
        let strip_offsets = self.reader.read_tag_values(reader, ifd, 273)?;
        let strip_byte_counts = self.reader.read_tag_values(reader, ifd, 279)?;

        if strip_offsets.len() != strip_byte_counts.len() {
            return Err(TiffError::GenericError(
                "Mismatch between strip offsets and byte counts".to_string()));
        }

        // Create handlers
        let source_handler = CompressionFactory::create_handler(source_compression)?;
        let target_handler = CompressionFactory::create_handler(target_compression)?;

        // Create vectors for new strip offsets and byte counts
        let mut new_strip_offsets = Vec::with_capacity(strip_offsets.len());
        let mut new_strip_byte_counts = Vec::with_capacity(strip_byte_counts.len());

        // Allocate space for strip offsets and byte counts data
        let strip_data_offset = *current_offset;

        // Skip past the space we'll use for strip offset/bytecount values
        let strips_count = strip_offsets.len() as u64;
        let values_size_per_strip = 8; // 4 bytes for offset + 4 bytes for byte count
        *current_offset += strips_count * values_size_per_strip;

        // Create progress bar for strip processing
        let strip_progress = multi_progress.add(indicatif::ProgressBar::new(strip_offsets.len() as u64));
        strip_progress.set_style(indicatif::ProgressStyle::default_bar()
            .template("{spinner:.red} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) Converting strips")
            .unwrap()
            .progress_chars("#>-"));

        // Process each strip
        for i in 0..strip_offsets.len() {
            let offset = strip_offsets[i];
            let byte_count = strip_byte_counts[i] as usize;

            // Read the strip data
            reader.seek(SeekFrom::Start(offset))?;
            let mut compressed_data = vec![0u8; byte_count];
            reader.read_exact(&mut compressed_data)?;

            // Update progress message with size information
            strip_progress.set_message(format!("Strip {}/{} - {} bytes",
                                               i + 1, strip_offsets.len(), byte_count));

            // Decompress
            let decompressed_data = source_handler.decompress(&compressed_data)?;

            // Recompress with target compression
            let recompressed_data = target_handler.compress(&decompressed_data)?;

            // Update progress with compression ratio
            let ratio = if compressed_data.len() > 0 {
                recompressed_data.len() as f32 / compressed_data.len() as f32 * 100.0
            } else {
                0.0
            };

            strip_progress.set_message(format!("Strip {}/{} - {}→{} bytes ({:.1}%)",
                                               i + 1, strip_offsets.len(),
                                               byte_count, recompressed_data.len(), ratio));

            // Write to the output file
            writer.seek(SeekFrom::Start(*current_offset))?;
            writer.write_all(&recompressed_data)?;

            // Record new offset and byte count
            new_strip_offsets.push(*current_offset);
            new_strip_byte_counts.push(recompressed_data.len() as u64);

            // Update current offset
            *current_offset += recompressed_data.len() as u64;

            // Align to 4-byte boundary (TIFF recommendation)
            if *current_offset % 4 != 0 {
                let padding = 4 - (*current_offset % 4);
                *current_offset += padding;
                // Write padding bytes
                writer.write_all(&vec![0u8; padding as usize])?;
            }

            // Update progress
            strip_progress.inc(1);
        }

        strip_progress.finish_with_message("Strip conversion complete");

        // Now write the strip offsets and byte counts
        writer.seek(SeekFrom::Start(strip_data_offset))?;
        for offset in &new_strip_offsets {
            writer.write_all(&(*offset as u32).to_le_bytes())?;
        }
        for byte_count in &new_strip_byte_counts {
            writer.write_all(&(*byte_count as u32).to_le_bytes())?;
        }

        // Update IFD entries for strip offsets and byte counts
        for entry in &mut new_ifd.entries {
            if entry.tag == 273 {  // StripOffsets
                entry.value_offset = strip_data_offset;
            } else if entry.tag == 279 {  // StripByteCounts
                entry.value_offset = strip_data_offset + (strips_count * 4);
            }
        }

        Ok(())
    }


    // Process tiles in a TIFF file
    fn process_tiles(&self, reader: &mut (impl Read + Seek + Send + Sync),
                     writer: &mut (impl Write + Seek + Send + Sync),
                     ifd: &crate::tiff::ifd::IFD,
                     source_compression: u64,
                     target_compression: u64,
                     new_ifd: &mut crate::tiff::ifd::IFD,
                     current_offset: &mut u64,
                     multi_progress: &indicatif::MultiProgress) -> TiffResult<()> {
        // Get tile offsets and byte counts
        let tile_offsets = self.reader.read_tag_values(reader, ifd, 324)?;
        let tile_byte_counts = self.reader.read_tag_values(reader, ifd, 325)?;

        if tile_offsets.len() != tile_byte_counts.len() {
            return Err(TiffError::GenericError(
                "Mismatch between tile offsets and byte counts".to_string()));
        }

        // Create handlers
        let source_handler = CompressionFactory::create_handler(source_compression)?;
        let target_handler = CompressionFactory::create_handler(target_compression)?;

        // Create vectors for new tile offsets and byte counts
        let mut new_tile_offsets = Vec::with_capacity(tile_offsets.len());
        let mut new_tile_byte_counts = Vec::with_capacity(tile_byte_counts.len());

        // Allocate space for tile offsets and byte counts data
        let tile_data_offset = *current_offset;

        // Skip past the space we'll use for tile offset/bytecount values
        let tiles_count = tile_offsets.len() as u64;
        let values_size_per_tile = 8; // 4 bytes for offset + 4 bytes for byte count
        *current_offset += tiles_count * values_size_per_tile;

        // Create progress bar for tile processing
        let tile_progress = multi_progress.add(indicatif::ProgressBar::new(tile_offsets.len() as u64));
        tile_progress.set_style(indicatif::ProgressStyle::default_bar()
            .template("{spinner:.yellow} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) Converting tiles")
            .unwrap()
            .progress_chars("#>-"));

        // Process each tile
        for i in 0..tile_offsets.len() {
            let offset = tile_offsets[i];
            let byte_count = tile_byte_counts[i] as usize;

            // Read the tile data
            reader.seek(SeekFrom::Start(offset))?;
            let mut compressed_data = vec![0u8; byte_count];
            reader.read_exact(&mut compressed_data)?;

            // Update progress message with size information
            tile_progress.set_message(format!("Tile {}/{} - {} bytes",
                                              i + 1, tile_offsets.len(), byte_count));

            // Decompress
            let decompressed_data = source_handler.decompress(&compressed_data)?;

            // Recompress with target compression
            let recompressed_data = target_handler.compress(&decompressed_data)?;

            // Update progress with compression ratio
            let ratio = if compressed_data.len() > 0 {
                recompressed_data.len() as f32 / compressed_data.len() as f32 * 100.0
            } else {
                0.0
            };

            tile_progress.set_message(format!("Tile {}/{} - {}→{} bytes ({:.1}%)",
                                              i + 1, tile_offsets.len(),
                                              byte_count, recompressed_data.len(), ratio));

            // Write to the output file
            writer.seek(SeekFrom::Start(*current_offset))?;
            writer.write_all(&recompressed_data)?;

            // Record new offset and byte count
            new_tile_offsets.push(*current_offset);
            new_tile_byte_counts.push(recompressed_data.len() as u64);

            // Update current offset
            *current_offset += recompressed_data.len() as u64;

            // Align to 4-byte boundary (TIFF recommendation)
            if *current_offset % 4 != 0 {
                let padding = 4 - (*current_offset % 4);
                *current_offset += padding;
                // Write padding bytes
                writer.write_all(&vec![0u8; padding as usize])?;
            }

            // Update progress
            tile_progress.inc(1);
        }

        tile_progress.finish_with_message("Tile conversion complete");

        // Now write the tile offsets and byte counts
        writer.seek(SeekFrom::Start(tile_data_offset))?;
        for offset in &new_tile_offsets {
            writer.write_all(&(*offset as u32).to_le_bytes())?;
        }
        for byte_count in &new_tile_byte_counts {
            writer.write_all(&(*byte_count as u32).to_le_bytes())?;
        }

        // Update IFD entries for tile offsets and byte counts
        for entry in &mut new_ifd.entries {
            if entry.tag == 324 {  // TileOffsets
                entry.value_offset = tile_data_offset;
            } else if entry.tag == 325 {  // TileByteCounts
                entry.value_offset = tile_data_offset + (tiles_count * 4);
            }
        }

        Ok(())
    }
}