//! Logger utility for application-wide logging
//!
//! This module provides a custom logger implementation that works alongside
//! the standard log crate, but adds file output capabilities.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use log::{Log, Record, Level, Metadata, LevelFilter};

/// Custom logger implementation
pub struct Logger {
    /// File handle for log output
    file: Mutex<Option<File>>,
}

impl Logger {
    /// Creates a new logger instance
    ///
    /// # Arguments
    ///
    /// * `log_file` - Path to the log file
    ///
    /// # Returns
    ///
    /// A new Logger instance or an error if the file cannot be created
    pub fn new(log_file: &str) -> io::Result<Self> {
        let file = File::create(Path::new(log_file))?;
        Ok(Logger {
            file: Mutex::new(Some(file)),
        })
    }

    /// Logs a message to the log file
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log
    pub fn log(&self, message: &str) -> io::Result<()> {
        if let Some(file) = &mut *self.file.lock().unwrap() {
            writeln!(file, "{}", message)?;
            file.flush()?;
        }
        Ok(())
    }

    /// Logs GeoKey directory information in a formatted way
    ///
    /// # Arguments
    ///
    /// * `geo_key_data` - Vector of GeoKey data tuples
    pub fn print_geo_key_directory(&self, geo_key_data: Vec<(u16, &str, u16, u16, u16, String)>) -> io::Result<()> {
        self.log("GeoKey Directory:")?;

        for (key_id, key_name, tiff_tag_location, count, _, value_str) in geo_key_data {
            let message = format!(
                "  Key ID: {} ({}), Location: {}, Count: {}, Value: {}",
                key_id, key_name, tiff_tag_location, count, value_str
            );
            self.log(&message)?;
        }

        Ok(())
    }

    /// Static method to initialize the global logger
    pub fn init_global_logger(log_file: &str) -> io::Result<()> {
        // Create a dedicated logger for the log crate
        let global_logger = Logger::new(log_file)?;

        // Set up the global logger - we'll ignore the SetLoggerError
        // since we only call this once at startup
        if let Err(_) = log::set_boxed_logger(Box::new(global_logger)) {
            // Logger was already set - this should not happen in normal usage
            eprintln!("Warning: Global logger was already initialized");
        }

        log::set_max_level(LevelFilter::Debug);
        Ok(())
    }
}

// Implement the Log trait to make our Logger work with the log crate
impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("[{}] {}", record.level(), record.args());
            let _ = self.log(&message);

            // Also print to console
            println!("{}", message);
        }
    }

    fn flush(&self) {
        // Already flushing in the log method
    }
}