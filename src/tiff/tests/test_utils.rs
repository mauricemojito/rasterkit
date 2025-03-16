use std::io::Cursor;
use byteorder::{LittleEndian, WriteBytesExt};

/// Creates a test buffer with TIFF header and sample data
pub fn create_test_tiff_buffer() -> Cursor<Vec<u8>> {
    let mut buffer = Vec::new();

    // TIFF header (little-endian)
    buffer.write_u16::<LittleEndian>(0x4949).unwrap(); // II for little-endian
    buffer.write_u16::<LittleEndian>(42).unwrap();     // TIFF magic number
    buffer.write_u32::<LittleEndian>(8).unwrap();      // IFD offset

    // Sample IFD (at offset 8)
    buffer.write_u16::<LittleEndian>(2).unwrap();      // Entry count (2 entries)

    // Entry 1: ImageWidth (tag 256)
    buffer.write_u16::<LittleEndian>(256).unwrap();    // Tag
    buffer.write_u16::<LittleEndian>(4).unwrap();      // Type (LONG)
    buffer.write_u32::<LittleEndian>(1).unwrap();      // Count
    buffer.write_u32::<LittleEndian>(800).unwrap();    // Value (width)

    // Entry 2: ImageLength/Height (tag 257)
    buffer.write_u16::<LittleEndian>(257).unwrap();    // Tag
    buffer.write_u16::<LittleEndian>(4).unwrap();      // Type (LONG)
    buffer.write_u32::<LittleEndian>(1).unwrap();      // Count
    buffer.write_u32::<LittleEndian>(600).unwrap();    // Value (height)

    // Next IFD offset (0 = no more IFDs)
    buffer.write_u32::<LittleEndian>(0).unwrap();

    println!("TIFF buffer created, size: {} bytes", buffer.len());
    println!("TIFF buffer contents: {:?}", buffer);

    // Return cursor at position 0
    Cursor::new(buffer)
}

/// Creates a test buffer with BigTIFF header and sample data
pub fn create_test_bigtiff_buffer() -> Cursor<Vec<u8>> {
    let mut buffer = Vec::new();

    // BigTIFF header (little-endian)
    buffer.write_u16::<LittleEndian>(0x4949).unwrap(); // II for little-endian
    buffer.write_u16::<LittleEndian>(43).unwrap();     // BigTIFF version
    buffer.write_u16::<LittleEndian>(8).unwrap();      // Offset size
    buffer.write_u16::<LittleEndian>(0).unwrap();      // Reserved
    buffer.write_u64::<LittleEndian>(16).unwrap();     // IFD offset

    // Sample IFD (at offset 16)
    buffer.write_u64::<LittleEndian>(2).unwrap();      // Entry count (2 entries)

    // Entry 1: ImageWidth (tag 256)
    buffer.write_u16::<LittleEndian>(256).unwrap();    // Tag
    buffer.write_u16::<LittleEndian>(4).unwrap();      // Type (LONG)
    buffer.write_u64::<LittleEndian>(1).unwrap();      // Count
    buffer.write_u64::<LittleEndian>(1024).unwrap();   // Value (width)

    // Entry 2: ImageLength/Height (tag 257)
    buffer.write_u16::<LittleEndian>(257).unwrap();    // Tag
    buffer.write_u16::<LittleEndian>(4).unwrap();      // Type (LONG)
    buffer.write_u64::<LittleEndian>(1).unwrap();      // Count
    buffer.write_u64::<LittleEndian>(768).unwrap();    // Value (height)

    // Next IFD offset (0 = no more IFDs)
    buffer.write_u64::<LittleEndian>(0).unwrap();

    println!("BigTIFF buffer created, size: {} bytes", buffer.len());
    println!("BigTIFF buffer contents: {:?}", buffer);

    // Return cursor at position 0
    Cursor::new(buffer)
}