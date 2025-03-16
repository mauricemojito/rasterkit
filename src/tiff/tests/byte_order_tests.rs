//! Tests for the byte order module

extern crate std;

use std::io::Cursor;
use byteorder::{LittleEndian, BigEndian, WriteBytesExt};
use crate::io::byte_order::{ByteOrder, ByteOrderHandler, LittleEndianHandler, BigEndianHandler};

#[test]
fn test_byte_order_detection_little_endian() {
    let mut buffer = Vec::new();
    buffer.write_u16::<LittleEndian>(0x4949).unwrap(); // II
    let mut cursor = Cursor::new(buffer);

    let result = ByteOrder::detect(&mut cursor);
    std::assert!(result.is_ok());
    std::assert_eq!(result.unwrap(), ByteOrder::LittleEndian);
}

#[test]
fn test_byte_order_detection_big_endian() {
    let mut buffer = Vec::new();
    buffer.write_u16::<BigEndian>(0x4D4D).unwrap(); // MM
    let mut cursor = Cursor::new(buffer);

    let result = ByteOrder::detect(&mut cursor);
    std::assert!(result.is_ok());
    std::assert_eq!(result.unwrap(), ByteOrder::BigEndian);
}

#[test]
fn test_byte_order_detection_invalid() {
    let mut buffer = Vec::new();
    buffer.write_u16::<LittleEndian>(0x1234).unwrap(); // Invalid
    let mut cursor = Cursor::new(buffer);

    let result = ByteOrder::detect(&mut cursor);
    std::assert!(result.is_err());
}

#[test]
fn test_little_endian_handler() {
    let mut buffer = Vec::new();
    buffer.write_u16::<LittleEndian>(0x1234).unwrap();
    buffer.write_u32::<LittleEndian>(0x12345678).unwrap();
    buffer.write_u64::<LittleEndian>(0x1234567890ABCDEF).unwrap();
    let mut cursor = Cursor::new(buffer);

    let handler = LittleEndianHandler;

    std::assert_eq!(handler.read_u16(&mut cursor).unwrap(), 0x1234);
    std::assert_eq!(handler.read_u32(&mut cursor).unwrap(), 0x12345678);
    std::assert_eq!(handler.read_u64(&mut cursor).unwrap(), 0x1234567890ABCDEF);
}

#[test]
fn test_big_endian_handler() {
    let mut buffer = Vec::new();
    buffer.write_u16::<BigEndian>(0x1234).unwrap();
    buffer.write_u32::<BigEndian>(0x12345678).unwrap();
    buffer.write_u64::<BigEndian>(0x1234567890ABCDEF).unwrap();
    let mut cursor = Cursor::new(buffer);

    let handler = BigEndianHandler;

    std::assert_eq!(handler.read_u16(&mut cursor).unwrap(), 0x1234);
    std::assert_eq!(handler.read_u32(&mut cursor).unwrap(), 0x12345678);
    std::assert_eq!(handler.read_u64(&mut cursor).unwrap(), 0x1234567890ABCDEF);
}