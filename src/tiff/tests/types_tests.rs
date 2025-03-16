//! Tests for the TIFF types module

extern crate std;

use crate::tiff::types::TIFF;
use crate::tiff::ifd::{IFD, IFDEntry};

#[test]
fn test_tiff_creation() {
    let tiff = TIFF::new(false);
    std::assert!(!tiff.is_big_tiff);
    std::assert_eq!(tiff.ifd_count(), 0);
    std::assert!(tiff.main_ifd().is_none());
}

#[test]
fn test_tiff_with_ifds() {
    let mut tiff = TIFF::new(true);

    // Create main IFD
    let mut main_ifd = IFD::new(0, 16);
    main_ifd.add_entry(IFDEntry::new(256, 4, 1, 1024));
    main_ifd.add_entry(IFDEntry::new(257, 4, 1, 768));
    tiff.ifds.push(main_ifd);

    // Create an overview IFD (subfile type 1)
    let mut overview_ifd = IFD::new(1, 100);
    overview_ifd.add_entry(IFDEntry::new(254, 4, 1, 1)); // Subfile type = 1 (reduced resolution)
    overview_ifd.add_entry(IFDEntry::new(256, 4, 1, 512));
    overview_ifd.add_entry(IFDEntry::new(257, 4, 1, 384));
    tiff.ifds.push(overview_ifd);

    // Test TIFF properties
    std::assert!(tiff.is_big_tiff);
    std::assert_eq!(tiff.ifd_count(), 2);
    std::assert!(tiff.main_ifd().is_some());

    // Test overviews
    let overviews = tiff.overviews();
    std::assert_eq!(overviews.len(), 1);
    std::assert_eq!(overviews[0].get_dimensions(), Some((512, 384)));
}