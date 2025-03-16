//! Byte order handling for TIFF files
//!
//! This module implements the Strategy pattern for handling different
//! byte orders (little-endian vs big-endian) when reading TIFF data.

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Result;

use crate::io::seekable::SeekableReader;
use crate::tiff::errors::{TiffError, TiffResult};

/// Represents the byte order of a TIFF file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    /// Little-endian byte order (II)
    LittleEndian,
    /// Big-endian byte order (MM)
    BigEndian,
}

impl ByteOrder {
    /// Detects the byte order from the TIFF header
    pub fn detect(reader: &mut dyn SeekableReader) -> TiffResult<Self> {
        let byte_order = reader.read_u16::<LittleEndian>()?;
        match byte_order {
            0x4949 => Ok(ByteOrder::LittleEndian), // "II" (Intel)
            0x4D4D => Ok(ByteOrder::BigEndian),    // "MM" (Motorola)
            _ => Err(TiffError::InvalidByteOrder(byte_order)),
        }
    }

    /// Returns a string representation of this byte order
    pub fn name(&self) -> &'static str {
        match self {
            ByteOrder::LittleEndian => "Little Endian (II)",
            ByteOrder::BigEndian => "Big Endian (MM)",
        }
    }

    /// Creates the appropriate handler for this byte order
    pub fn create_handler(&self) -> Box<dyn ByteOrderHandler> {
        match self {
            ByteOrder::LittleEndian => Box::new(LittleEndianHandler),
            ByteOrder::BigEndian => Box::new(BigEndianHandler),
        }
    }
}

/// Trait for byte order handling strategies
pub trait ByteOrderHandler: Send + Sync {
    /// Read a u16 value
    fn read_u16(&self, reader: &mut dyn SeekableReader) -> Result<u16>;

    /// Read a u32 value
    fn read_u32(&self, reader: &mut dyn SeekableReader) -> Result<u32>;

    /// Read a u64 value
    fn read_u64(&self, reader: &mut dyn SeekableReader) -> Result<u64>;

    /// Read an f32 value
    fn read_f32(&self, reader: &mut dyn SeekableReader) -> Result<f32>;

    /// Read an f64 value
    fn read_f64(&self, reader: &mut dyn SeekableReader) -> Result<f64>;

    /// Read a rational value (two u32 values as numerator/denominator)
    fn read_rational(&self, reader: &mut dyn SeekableReader) -> Result<(u32, u32)>;

    /// Read a signed rational value (two i32 values as numerator/denominator)
    fn read_srational(&self, reader: &mut dyn SeekableReader) -> Result<(i32, i32)>;
}

/// Little-endian byte order handler
pub struct LittleEndianHandler;

impl ByteOrderHandler for LittleEndianHandler {
    fn read_u16(&self, reader: &mut dyn SeekableReader) -> Result<u16> {
        reader.read_u16::<LittleEndian>()
    }

    fn read_u32(&self, reader: &mut dyn SeekableReader) -> Result<u32> {
        reader.read_u32::<LittleEndian>()
    }

    fn read_u64(&self, reader: &mut dyn SeekableReader) -> Result<u64> {
        reader.read_u64::<LittleEndian>()
    }

    fn read_f32(&self, reader: &mut dyn SeekableReader) -> Result<f32> {
        reader.read_f32::<LittleEndian>()
    }

    fn read_f64(&self, reader: &mut dyn SeekableReader) -> Result<f64> {
        reader.read_f64::<LittleEndian>()
    }

    fn read_rational(&self, reader: &mut dyn SeekableReader) -> Result<(u32, u32)> {
        let numerator = reader.read_u32::<LittleEndian>()?;
        let denominator = reader.read_u32::<LittleEndian>()?;
        Ok((numerator, denominator))
    }

    fn read_srational(&self, reader: &mut dyn SeekableReader) -> Result<(i32, i32)> {
        let numerator = reader.read_i32::<LittleEndian>()?;
        let denominator = reader.read_i32::<LittleEndian>()?;
        Ok((numerator, denominator))
    }
}

/// Big-endian byte order handler
pub struct BigEndianHandler;

impl ByteOrderHandler for BigEndianHandler {
    fn read_u16(&self, reader: &mut dyn SeekableReader) -> Result<u16> {
        reader.read_u16::<BigEndian>()
    }

    fn read_u32(&self, reader: &mut dyn SeekableReader) -> Result<u32> {
        reader.read_u32::<BigEndian>()
    }

    fn read_u64(&self, reader: &mut dyn SeekableReader) -> Result<u64> {
        reader.read_u64::<BigEndian>()
    }

    fn read_f32(&self, reader: &mut dyn SeekableReader) -> Result<f32> {
        reader.read_f32::<BigEndian>()
    }

    fn read_f64(&self, reader: &mut dyn SeekableReader) -> Result<f64> {
        reader.read_f64::<BigEndian>()
    }

    fn read_rational(&self, reader: &mut dyn SeekableReader) -> Result<(u32, u32)> {
        let numerator = reader.read_u32::<BigEndian>()?;
        let denominator = reader.read_u32::<BigEndian>()?;
        Ok((numerator, denominator))
    }

    fn read_srational(&self, reader: &mut dyn SeekableReader) -> Result<(i32, i32)> {
        let numerator = reader.read_i32::<BigEndian>()?;
        let denominator = reader.read_i32::<BigEndian>()?;
        Ok((numerator, denominator))
    }
}