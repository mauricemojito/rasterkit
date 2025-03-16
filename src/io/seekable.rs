//! Seekable reader trait and implementations
//!
//! This module provides a unified trait for readers that support both
//! reading and seeking operations.

use std::io::{Read, Seek};

/// Trait for readers that can both read and seek
///
/// This trait combines the Read and Seek traits for use with
/// various readers throughout the application.
pub trait SeekableReader: Read + Seek + Send + Sync {}

// Blanket implementation for any type that implements the required traits
impl<T: Read + Seek + Send + Sync> SeekableReader for T {}