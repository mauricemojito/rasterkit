//! Compression conversion command
//!
//! This module implements the command for converting TIFF files
//! between different compression formats.

use clap::ArgMatches;
use log::{info, error};

use crate::commands::command_traits::Command;
use crate::tiff::errors::{TiffResult, TiffError};
use crate::utils::logger::Logger;
use crate::compression::{CompressionFactory, CompressionConverter};

/// Command for converting TIFF compression format
pub struct ConvertCommand<'a> {
    /// Path to the input file
    input_file: String,
    /// Path to the output file
    output_file: String,
    /// Target compression code
    target_compression: u64,
    /// Logger for recording operations
    logger: &'a Logger,
}

impl<'a> ConvertCommand<'a> {
    /// Create a new convert command
    ///
    /// # Arguments
    /// * `args` - CLI argument matches from clap
    /// * `logger` - Logger for recording operations
    ///
    /// # Returns
    /// A new ConvertCommand instance or an error
    pub fn new(args: &ArgMatches, logger: &'a Logger) -> TiffResult<Self> {
        let input_file = args.get_one::<String>("input")
            .ok_or_else(|| TiffError::GenericError("Missing input file".to_string()))?
            .clone();

        let output_file = args.get_one::<String>("output")
            .ok_or_else(|| TiffError::GenericError("Missing output file path for conversion".to_string()))?
            .clone();

        // Determine target compression
        let target_compression = if let Some(compression_str) = args.get_one::<String>("compression") {
            // Try to parse the compression code
            compression_str.parse::<u64>()
                .map_err(|_| TiffError::GenericError(format!("Invalid compression code: {}", compression_str)))?
        } else if let Some(compression_name) = args.get_one::<String>("compression-name") {
            // Try to get compression by name
            match CompressionFactory::get_handler_by_name(compression_name) {
                Ok(handler) => handler.code(),
                Err(_) => return Err(TiffError::GenericError(format!("Unknown compression name: {}", compression_name)))
            }
        } else {
            return Err(TiffError::GenericError("Missing compression specification. Use --compression or --compression-name".to_string()));
        };

        // Validate the compression is supported
        match CompressionFactory::create_handler(target_compression) {
            Ok(handler) => info!("Using compression: {}", handler.name()),
            Err(_) => return Err(TiffError::GenericError(format!("Unsupported compression code: {}", target_compression)))
        }

        Ok(ConvertCommand {
            input_file,
            output_file,
            target_compression,
            logger,
        })
    }
}

impl<'a> Command for ConvertCommand<'a> {
    fn execute(&self) -> TiffResult<()> {
        info!("Converting file {} to {} with compression code {}",
              self.input_file, self.output_file, self.target_compression);

        // Create compression converter
        let mut converter = CompressionConverter::new(self.logger);

        // Convert the file
        converter.convert_file(&self.input_file, &self.output_file, self.target_compression)?;

        info!("Compression conversion successful");
        self.logger.log("Compression conversion successful")?;

        Ok(())
    }
}