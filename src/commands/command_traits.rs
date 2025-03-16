//! Command pattern interfaces
//!
//! This module defines the core Command pattern interfaces
//! for the CLI application, enabling a clean separation of concerns.

use crate::utils::logger::Logger;
use crate::tiff::errors::TiffResult;

/// Represents an executable command in the application
///
/// Command objects encapsulate the logic for a specific CLI operation,
/// allowing for separation of concerns and better testability.
pub trait Command {
    /// Execute the command
    ///
    /// # Returns
    /// Result indicating success or an error
    fn execute(&self) -> TiffResult<()>;
}

/// Factory for creating commands from CLI arguments
///
/// This trait defines the interface for command factories
/// which can parse CLI arguments and create the appropriate Command.
pub trait CommandFactory<'a> {
    /// Create a new Command instance based on CLI arguments
    ///
    /// # Arguments
    /// * `args` - CLI argument matches from clap
    /// * `logger` - Logger for recording operations
    ///
    /// # Returns
    /// A command that implements the Command trait, or an error
    fn create_command(&self, args: &clap::ArgMatches, logger: &'a Logger) -> TiffResult<Box<dyn Command + 'a>>;
}