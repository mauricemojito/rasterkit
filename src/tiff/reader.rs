//! TIFF file reader implementation
//!
//! This module implements the TIFF/BigTIFF file reader that uses the
//! Strategy pattern to handle different byte orders.

use log::{debug, info, warn};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::io::seekable::SeekableReader;
use crate::io::byte_order::ByteOrderHandler;
use crate::tiff::errors::{TiffError, TiffResult};
use crate::tiff::ifd::{IFD, IFDEntry};
use crate::tiff::types::TIFF;
use crate::tiff::validation;
use crate::utils::format_utils;
use crate::utils::ifd_utils;
use crate::utils::tag_utils;
use crate::utils::string_utils;
use crate::utils::logger::Logger;

/// Builder for TiffReader
///
/// Provides a clean way to construct a TiffReader with various configurations.
pub struct TiffReaderBuilder<'a> {
    /// Logger to use
    logger: &'a Logger,
}

impl<'a> TiffReaderBuilder<'a> {
    /// Create a new TiffReaderBuilder
    pub fn new(logger: &'a Logger) -> Self {
        TiffReaderBuilder { logger }
    }

    /// Build the TiffReader
    pub fn build(self) -> TiffReader<'a> {
        TiffReader::new(self.logger)
    }
}

/// Reader for TIFF and BigTIFF files
pub struct TiffReader<'a> {
    /// Current byte order handler
    pub(crate) byte_order_handler: Option<Box<dyn ByteOrderHandler>>,
    /// Logger instance
    logger: &'a Logger,
    /// Current file path
    current_file: Option<String>,
    /// Whether currently reading BigTIFF format
    pub(crate) is_big_tiff: bool,
}

impl<'a> TiffReader<'a> {
    /// Creates a new TIFF reader
    pub fn new(logger: &'a Logger) -> Self {
        TiffReader {
            byte_order_handler: None,
            logger,
            current_file: None,
            is_big_tiff: false,
        }
    }

    /// Creates a file reader for the current file
    ///
    /// This is an internal utility to open the current file for reading.
    /// It's used by various methods that need to access file content.
    pub(crate) fn create_reader(&self) -> TiffResult<File> {
        match &self.current_file {
            Some(path) => {
                let file = File::open(path)?;
                Ok(file)
            },
            None => Err(TiffError::GenericError("No file path specified".to_string()))
        }
    }

    /// Returns the byte order handler, with proper error handling for None case
    ///
    /// This centralizes the error handling for byte_order_handler access
    fn get_byte_order_handler_unwrapped(&self) -> TiffResult<&Box<dyn ByteOrderHandler>> {
        self.byte_order_handler.as_ref()
            .ok_or_else(|| TiffError::GenericError("Byte order not yet determined".to_string()))
    }

    /// Loads a TIFF file from the given path
    ///
    /// This is the main entry point for loading a TIFF file.
    /// It opens the file and delegates to the read() method.
    ///
    /// # Arguments
    /// * `filepath` - Path to the TIFF file to load
    ///
    /// # Returns
    /// A TIFF structure containing the file's contents
    pub fn load(&mut self, filepath: &str) -> TiffResult<TIFF> {
        info!("Loading TIFF file: {}", filepath);
        self.current_file = Some(filepath.to_string());

        let path = Path::new(filepath);
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(1024 * 1024, file); // 1MB buffer

        self.read(&mut reader)
    }

    /// Reads a TIFF file from the given reader
    ///
    /// This method handles the core process of reading a TIFF file:
    /// 1. Detect byte order (little/big endian)
    /// 2. Check for TIFF or BigTIFF format
    /// 3. Read all IFDs (Image File Directories)
    ///
    /// # Arguments
    /// * `reader` - Any struct implementing the SeekableReader trait
    ///
    /// # Returns
    /// A TIFF structure containing the file's contents
    pub fn read(&mut self, reader: &mut dyn SeekableReader) -> TiffResult<TIFF> {
        debug!("Reader::read starting");

        // Detect and set up byte order
        self.byte_order_handler = Some(format_utils::detect_byte_order(reader)?);

        // Check for BigTIFF format and validate header
        let handler = self.byte_order_handler.as_ref().unwrap();
        let (is_big_tiff, _) = format_utils::detect_tiff_format(reader, handler)?;
        self.is_big_tiff = is_big_tiff;

        // Read the IFDs
        let mut tiff = TIFF::new(self.is_big_tiff);

        // Get a fresh reference to the handler after modifying self
        let handler = self.byte_order_handler.as_ref().unwrap();

        // Read the first IFD offset
        let first_ifd_offset = ifd_utils::read_first_ifd_offset(reader, self.is_big_tiff, handler)?;
        debug!("First IFD offset: {}", first_ifd_offset);

        // Validate the first IFD offset
        let file_size = validation::get_file_size(reader)?;
        validation::validate_ifd_offset(first_ifd_offset, file_size)?;

        // Read all IFDs in the chain
        tiff.ifds = self.read_ifd_chain(reader, first_ifd_offset)?;

        info!("Read {} IFDs from TIFF file", tiff.ifds.len());
        Ok(tiff)
    }

    /// Reads a chain of IFDs starting from the given offset
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    /// * `first_ifd_offset` - Offset of the first IFD in the chain
    ///
    /// # Returns
    /// A vector of IFDs
    fn read_ifd_chain(&self, reader: &mut dyn SeekableReader, first_ifd_offset: u64) -> TiffResult<Vec<IFD>> {
        let mut ifds = Vec::new();
        let mut ifd_offset = first_ifd_offset;
        let mut ifd_number = 0;
        let max_ifds = 100; // Reasonable limit to prevent infinite loops
        let handler = self.get_byte_order_handler_unwrapped()?;

        while ifd_offset != 0 && ifd_number < max_ifds {
            debug!("Reading IFD at offset: {}", ifd_offset);

            // Get the file size for validation
            let file_size = validation::get_file_size(reader)?;

            // Validate the current IFD offset
            if ifd_offset >= file_size {
                warn!("IFD offset {} exceeds file size {}, stopping IFD chain",
                  ifd_offset, file_size);
                break;
            }

            // Try to read the IFD
            match self.read_ifd(reader, ifd_offset, ifd_number) {
                Ok(ifd) => {
                    debug!("Successfully read IFD with {} entries", ifd.entries.len());

                    // Get next IFD offset
                    let next_offset_position = ifd_offset + ifd_utils::calculate_ifd_size(&ifd, self.is_big_tiff);

                    // Validate next offset position
                    if next_offset_position >= file_size {
                        warn!("Next IFD offset position {} exceeds file size {}",
                          next_offset_position, file_size);
                        ifds.push(ifd);
                        break;
                    }

                    if let Err(e) = reader.seek(SeekFrom::Start(next_offset_position)) {
                        warn!("Error seeking to next IFD offset: {}", e);
                        ifds.push(ifd);
                        break;
                    }

                    // Read next IFD offset
                    let next_ifd_offset = match ifd_utils::read_next_ifd_offset(reader, self.is_big_tiff, handler) {
                        Ok(offset) => offset,
                        Err(e) => {
                            warn!("Error reading next IFD offset: {}", e);
                            ifds.push(ifd);
                            break;
                        }
                    };

                    debug!("Next IFD offset: {}", next_ifd_offset);

                    // Sanity check for next IFD offset
                    if next_ifd_offset != 0 && (next_ifd_offset >= file_size || next_ifd_offset < 8) {
                        warn!("Invalid next IFD offset: {}, stopping IFD chain", next_ifd_offset);
                        ifds.push(ifd);
                        break;
                    }

                    ifds.push(ifd);
                    ifd_offset = next_ifd_offset;
                    ifd_number += 1;
                },
                Err(e) => {
                    warn!("Error reading IFD {}: {}", ifd_number, e);
                    break;
                }
            }
        }

        Ok(ifds)
    }

    /// Reads an IFD from the reader
    ///
    /// An IFD (Image File Directory) contains all the metadata for a single image.
    /// It consists of a count followed by a series of entries, each describing
    /// an aspect of the image (dimensions, color space, compression, etc.)
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    /// * `offset` - Offset in the file where the IFD starts
    /// * `number` - The index of this IFD in the file
    ///
    /// # Returns
    /// The parsed IFD structure
    pub fn read_ifd(&self, reader: &mut dyn SeekableReader, offset: u64, number: usize) -> TiffResult<IFD> {
        reader.seek(SeekFrom::Start(offset))?;

        let entry_count = self.read_ifd_entry_count(reader)?;
        debug!("IFD entry count: {}", entry_count);

        let mut ifd = IFD::new(number, offset);

        for _ in 0..entry_count {
            let entry = self.read_ifd_entry(reader)?;
            debug!("Read IFD entry: tag={}, type={}, count={}, offset={}",
                   entry.tag, entry.field_type, entry.count, entry.value_offset);

            ifd.add_entry(entry);
        }

        info!("Read IFD with {} entries", ifd.entries.len());
        Ok(ifd)
    }

    /// Reads the entry count from an IFD
    fn read_ifd_entry_count(&self, reader: &mut dyn SeekableReader) -> TiffResult<u64> {
        let handler = self.get_byte_order_handler_unwrapped()?;
        if self.is_big_tiff {
            handler.read_u64(reader).map_err(TiffError::IoError)
        } else {
            handler.read_u16(reader)
                .map(|v| v as u64)
                .map_err(TiffError::IoError)
        }
    }

    /// Reads a single IFD entry
    fn read_ifd_entry(&self, reader: &mut dyn SeekableReader) -> TiffResult<IFDEntry> {
        let handler = self.get_byte_order_handler_unwrapped()?;

        let tag = handler.read_u16(reader)?;
        let field_type = handler.read_u16(reader)?;
        let count = if self.is_big_tiff {
            handler.read_u64(reader)?
        } else {
            handler.read_u32(reader)? as u64
        };

        let value_offset = if self.is_big_tiff {
            handler.read_u64(reader)?
        } else {
            handler.read_u32(reader)? as u64
        };

        Ok(IFDEntry::new(tag, field_type, count, value_offset))
    }

    /// Reads a tag's value as a vector of u64
    ///
    /// This is a utility method for extracting tag values from an IFD.
    /// It handles different field types and automatically converts them to u64.
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    /// * `ifd` - The IFD containing the tag
    /// * `tag` - The tag number to read
    ///
    /// # Returns
    /// A vector of u64 values
    pub fn read_tag_values(&self, reader: &mut dyn SeekableReader, ifd: &IFD, tag: u16) -> TiffResult<Vec<u64>> {
        let entry = ifd.get_entry(tag)
            .ok_or_else(|| TiffError::TagNotFound(tag))?;

        let mut values = Vec::with_capacity(entry.count as usize);

        // Check if the value is stored inline
        if tag_utils::is_value_inline(entry, self.is_big_tiff) {
            values.push(entry.value_offset);
        } else {
            reader.seek(SeekFrom::Start(entry.value_offset))?;
            let handler = self.get_byte_order_handler_unwrapped()?;
            tag_utils::read_tag_value_array(reader, entry, handler, &mut values)?;
        }

        Ok(values)
    }

    /// Reads a rational value (numerator/denominator pair)
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    ///
    /// # Returns
    /// A tuple with numerator and denominator
    pub fn read_rational(&self, reader: &mut dyn SeekableReader) -> TiffResult<(u32, u32)> {
        let handler = self.get_byte_order_handler_unwrapped()?;
        handler.read_rational(reader).map_err(TiffError::IoError)
    }

    /// Reads a u16 value using the current byte order handler
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    ///
    /// # Returns
    /// The read u16 value
    pub fn read_u16(&self, reader: &mut dyn SeekableReader) -> TiffResult<u16> {
        let handler = self.get_byte_order_handler_unwrapped()?;
        handler.read_u16(reader).map_err(TiffError::IoError)
    }

    /// Reads a u32 value using the current byte order handler
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    ///
    /// # Returns
    /// The read u32 value
    pub fn read_u32(&self, reader: &mut dyn SeekableReader) -> TiffResult<u32> {
        let handler = self.get_byte_order_handler_unwrapped()?;
        handler.read_u32(reader).map_err(TiffError::IoError)
    }

    /// Reads a u64 value using the current byte order handler
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    ///
    /// # Returns
    /// The read u64 value
    pub fn read_u64(&self, reader: &mut dyn SeekableReader) -> TiffResult<u64> {
        let handler = self.get_byte_order_handler_unwrapped()?;
        handler.read_u64(reader).map_err(TiffError::IoError)
    }

    /// Reads an f64 value using the current byte order handler
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    ///
    /// # Returns
    /// The read f64 value
    pub fn read_f64(&self, reader: &mut dyn SeekableReader) -> TiffResult<f64> {
        let handler = self.get_byte_order_handler_unwrapped()?;
        handler.read_f64(reader).map_err(TiffError::IoError)
    }

    /// Reads an ASCII string
    ///
    /// # Arguments
    /// * `reader` - The seekable reader to use
    /// * `count` - Number of bytes to read
    ///
    /// # Returns
    /// The string value, with trailing null characters removed
    pub fn read_ascii_string(&self, reader: &mut dyn SeekableReader, count: u64) -> TiffResult<String> {
        let mut buffer = vec![0u8; count as usize];
        reader.read_exact(&mut buffer)?;

        // Trim trailing nulls
        string_utils::trim_trailing_nulls(&mut buffer);

        match String::from_utf8(buffer) {
            Ok(s) => Ok(s),
            Err(e) => Err(TiffError::GenericError(format!("Invalid UTF-8 string: {}", e))),
        }
    }

    /// Reads an ASCII string at a specific file offset
    ///
    /// # Arguments
    /// * `offset` - File offset where the string starts
    /// * `count` - Number of bytes to read
    ///
    /// # Returns
    /// The string value
    pub fn read_ascii_string_at_offset(&self, offset: u64, count: u64) -> TiffResult<String> {
        let mut file = self.create_reader()?;
        file.seek(SeekFrom::Start(offset))?;
        self.read_ascii_string(&mut file, count)
    }

    /// Gets the file path if available
    ///
    /// # Returns
    /// The current file path or None
    pub fn get_file_path(&self) -> Option<&str> {
        self.current_file.as_deref()
    }

    /// Returns whether the current file is a BigTIFF
    ///
    /// # Returns
    /// true if the file is BigTIFF, false if it's standard TIFF
    pub fn is_big_tiff(&self) -> bool {
        self.is_big_tiff
    }

    /// Read IFD overviews (reduced resolution subfiles)
    ///
    /// Overviews are lower-resolution versions of the main image,
    /// used for faster display at reduced zoom levels.
    ///
    /// # Arguments
    /// * `filepath` - Path to the TIFF file
    ///
    /// # Returns
    /// A vector of IFDs representing the overviews
    pub fn read_overviews(&mut self, filepath: &str) -> TiffResult<Vec<IFD>> {
        let tiff = self.load(filepath)?;

        let mut result = Vec::new();
        for overview in tiff.overviews() {
            result.push(overview.clone());
        }

        Ok(result)
    }

    /// Gets the current byte order handler
    ///
    /// # Returns
    /// A reference to the current byte order handler, or None if not yet set
    pub fn get_byte_order_handler(&self) -> Option<&Box<dyn ByteOrderHandler>> {
        self.byte_order_handler.as_ref()
    }
}