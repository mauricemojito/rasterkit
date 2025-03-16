//! Core TIFF data structures

use crate::tiff::ifd::IFD;
use std::fmt;

/// Represents a TIFF file with its Image File Directories (IFDs)
#[derive(Debug)]
pub struct TIFF {
    /// Image File Directories in the TIFF file
    pub ifds: Vec<IFD>,
    /// Whether this is a BigTIFF format
    pub is_big_tiff: bool,
}

impl TIFF {
    /// Creates a new empty TIFF structure
    pub fn new(is_big_tiff: bool) -> Self {
        TIFF {
            ifds: Vec::new(),
            is_big_tiff,
        }
    }

    /// Returns the main (first) IFD if available
    pub fn main_ifd(&self) -> Option<&IFD> {
        self.ifds.first()
    }

    /// Returns the number of IFDs in the TIFF file
    pub fn ifd_count(&self) -> usize {
        self.ifds.len()
    }

    /// Returns a reference to all overview IFDs (subfile type 1)
    pub fn overviews(&self) -> Vec<&IFD> {
        self.ifds.iter()
            .filter(|ifd| {
                if let Some(subfile_type) = ifd.get_tag_value(254) {
                    subfile_type & 1 == 1 // Check if it's a reduced resolution subfile
                } else {
                    false
                }
            })
            .collect()
    }
}

impl fmt::Display for TIFF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TIFF File:")?;
        writeln!(f, "  Format: {}", if self.is_big_tiff { "BigTIFF" } else { "TIFF" })?;
        writeln!(f, "  Number of IFDs: {}", self.ifds.len())?;

        if let Some(ifd) = self.main_ifd() {
            write!(f, "{}", ifd)?;
        }

        Ok(())
    }
}