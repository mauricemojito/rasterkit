//! Integration tests for the TIFF module

extern crate std;

use std::io::Cursor;
use std::io::Write;

// Import crate items
use rasterkit::tiff::Reader;
use rasterkit::tiff::ByteOrder;
use rasterkit::utils::logger::Logger;
use rasterkit::io::seekable::SeekableReader;

#[test]
fn test_complete_tiff_workflow() {
    // Create a sample TIFF file in memory
    let mut buffer = Vec::new();

    // TIFF header (little-endian)
    buffer.extend_from_slice(&[0x49, 0x49]); // "II" for little-endian
    buffer.extend_from_slice(&[42, 0]);      // TIFF magic number
    buffer.extend_from_slice(&[8, 0, 0, 0]); // Offset to first IFD

    // IFD with two entries
    buffer.extend_from_slice(&[2, 0]);       // Number of entries

    // Entry 1: ImageWidth (tag 256)
    buffer.extend_from_slice(&[0, 1]);       // Tag (256)
    buffer.extend_from_slice(&[4, 0]);       // Type (LONG)
    buffer.extend_from_slice(&[1, 0, 0, 0]); // Count
    buffer.extend_from_slice(&[200, 0, 0, 0]); // Value (width = 200)

    // Entry 2: ImageLength (tag 257)
    buffer.extend_from_slice(&[1, 1]);       // Tag (257)
    buffer.extend_from_slice(&[4, 0]);       // Type (LONG)
    buffer.extend_from_slice(&[1, 0, 0, 0]); // Count
    buffer.extend_from_slice(&[100, 0, 0, 0]); // Value (height = 100)

    // Next IFD offset (0 = no more IFDs)
    buffer.extend_from_slice(&[0, 0, 0, 0]);

    let mut cursor = Cursor::new(buffer);
    let logger = Logger::new("integration_test.log").unwrap();
    let mut reader = Reader::new(&logger);

    // Read the TIFF
    let result = reader.read(&mut cursor);
    std::assert!(result.is_ok());

    let tiff = result.unwrap();
    std::assert!(!tiff.is_big_tiff);
    std::assert_eq!(tiff.ifds.len(), 1);

    // Check IFD contents
    let ifd = &tiff.ifds[0];
    std::assert_eq!(ifd.entries.len(), 2);

    // Verify dimensions
    std::assert_eq!(ifd.get_dimensions(), Some((200, 100)));
}