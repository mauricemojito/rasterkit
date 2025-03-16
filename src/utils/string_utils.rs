//! String utility functions
//!
//! Utilities for working with strings and text data.

/// Trims trailing null characters from a byte buffer
pub fn trim_trailing_nulls(buffer: &mut Vec<u8>) {
    while !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
        buffer.pop();
    }
}