//! CLI command implementations
//!
//! This module contains implementations of various commands
//! supported by the CLI application using the Command pattern.

pub mod command_traits;
pub mod analyze_command;
pub mod extract_command;
pub mod convert_command;

pub use command_traits::{Command, CommandFactory};
pub use analyze_command::AnalyzeCommand;
pub use extract_command::ExtractCommand;
pub use convert_command::ConvertCommand;

use clap::ArgMatches;
use crate::utils::logger::Logger;
use crate::tiff::errors::TiffResult;

/// Factory for creating command instances based on CLI arguments
///
/// This factory examines the command-line arguments and creates
/// the appropriate command instance for execution.
pub struct RasterkitCommandFactory;

impl RasterkitCommandFactory {
    /// Create a new factory instance
    pub fn new() -> Self {
        RasterkitCommandFactory
    }
}

impl<'a> CommandFactory<'a> for RasterkitCommandFactory {
    fn create_command(&self, args: &ArgMatches, logger: &'a Logger) -> TiffResult<Box<dyn Command + 'a>> {
        // Determine which command to run based on args
        if args.get_flag("extract") || args.get_flag("extract-array") {
            // Both regular extraction and array extraction use the ExtractCommand
            Ok(Box::new(ExtractCommand::new(args, logger)?))
        } else if args.get_flag("convert") {
            Ok(Box::new(ConvertCommand::new(args, logger)?))
        } else {
            // Default to analyze command
            Ok(Box::new(AnalyzeCommand::new(args, logger)?))
        }
    }
}