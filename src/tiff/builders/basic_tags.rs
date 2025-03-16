//! Basic TIFF tag strategies
//!
//! This module provides functionality for adding common TIFF tags
//! like dimensions, color spaces, and sample properties.

use crate::tiff::ifd::{IFD, IFDEntry};
use crate::tiff::constants::{tags, field_types, photometric, compression, planar_config};
use log::{debug, info, warn};

/// Adds basic TIFF tags to an IFD
pub struct BasicTagsBuilder;

impl BasicTagsBuilder {
    /// Add bits per sample values for an IFD
    ///
    /// This is critical for defining how many bits are used per color channel.
    /// For example, typical RGB images use 8 bits per channel, while high-depth
    /// images might use 16 bits per channel.
    pub fn add_bits_per_sample(
        ifd: &mut IFD,
        external_data: &mut std::collections::HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        bits_per_channel: &[u16]
    ) {
        debug!("Adding BitsPerSample: {:?}", bits_per_channel);

        let count = bits_per_channel.len() as u64;

        // For single-channel images like grayscale, we can store the bits
        // value directly in the tag entry
        if count == 1 {
            ifd.add_entry(IFDEntry::new(
                tags::BITS_PER_SAMPLE,
                field_types::SHORT,
                1,
                bits_per_channel[0] as u64)
            );
        } else {
            // For multi-channel images (RGB, RGBA, etc.), we need to store an array
            // of values as external data since they won't fit in the tag value
            let mut data = Vec::with_capacity(bits_per_channel.len() * 2);
            for &bits in bits_per_channel {
                data.extend_from_slice(&bits.to_le_bytes());
            }

            // Tag 258 is BitsPerSample - indicates how many bits are used for each sample
            // Type 3 means SHORT (16-bit unsigned)
            ifd.add_entry(IFDEntry::new(
                tags::BITS_PER_SAMPLE,
                field_types::SHORT,
                count,
                0)
            );
            external_data.insert((ifd_index, tags::BITS_PER_SAMPLE), data);
        }
    }

    /// Setup single strip for an IFD
    ///
    /// TIFF files store image data in strips. This function sets up a basic
    /// single-strip configuration, which is simpler but only practical for
    /// smaller images. For large images, multiple strips would be better.
    pub fn setup_single_strip(
        ifd: &mut IFD,
        image_data: &mut std::collections::HashMap<usize, Vec<u8>>,
        ifd_index: usize,
        strip_data: Vec<u8>
    ) {
        info!("Setting up single strip: {} bytes", strip_data.len());

        // StripOffsets tells where in the file the strip data starts
        // We'll set actual value later during write - for now it's just a placeholder
        ifd.add_entry(IFDEntry::new(
            tags::STRIP_OFFSETS,
            field_types::LONG,
            1,
            0)
        );

        // StripByteCounts tells how many bytes are in each strip
        // This is important for parsers to know how much data to read
        ifd.add_entry(IFDEntry::new(
            tags::STRIP_BYTE_COUNTS,
            field_types::LONG,
            1,
            strip_data.len() as u64)
        );

        // RowsPerStrip defines how many rows are in each strip
        // For a single-strip image, this equals the image height
        if let Some((_, height)) = ifd.get_dimensions() {
            ifd.add_entry(IFDEntry::new(
                tags::ROWS_PER_STRIP,
                field_types::LONG,
                1,
                height)
            );
        }

        // Store the actual pixel data for later writing
        image_data.insert(ifd_index, strip_data);
    }

    /// Add common tags for a basic RGB image
    ///
    /// Sets up all the required tags for an uncompressed RGB image.
    /// This is the most common color format for regular photos.
    pub fn add_basic_rgb_tags(
        ifd: &mut IFD,
        external_data: &mut std::collections::HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        width: u32,
        height: u32
    ) {
        info!("Adding basic RGB tags for {}x{} image", width, height);

        // Basic image dimensions - these are mandatory for any TIFF
        ifd.add_entry(IFDEntry::new(
            tags::IMAGE_WIDTH,
            field_types::LONG,
            1,
            width as u64)
        );

        ifd.add_entry(IFDEntry::new(
            tags::IMAGE_LENGTH,
            field_types::LONG,
            1,
            height as u64)
        );

        // Standard 8-bit per channel RGB
        Self::add_bits_per_sample(ifd, external_data, ifd_index, &[8, 8, 8]);

        // No compression - easier to work with but results in larger files
        ifd.add_entry(IFDEntry::new(
            tags::COMPRESSION,
            field_types::SHORT,
            1,
            compression::NONE as u64)
        );

        // RGB color interpretation
        ifd.add_entry(IFDEntry::new(
            tags::PHOTOMETRIC_INTERPRETATION,
            field_types::SHORT,
            1,
            photometric::RGB as u64)
        );

        // RGB has 3 samples (channels) per pixel
        ifd.add_entry(IFDEntry::new(
            tags::SAMPLES_PER_PIXEL,
            field_types::SHORT,
            1,
            3)
        );

        // Use a single strip for the entire image
        // For large images, we'd want multiple strips instead
        ifd.add_entry(IFDEntry::new(
            tags::ROWS_PER_STRIP,
            field_types::LONG,
            1,
            height as u64)
        );

        // Chunky format means RGB values are interleaved (RGBRGBRGB)
        // rather than planar (RRR...GGG...BBB)
        ifd.add_entry(IFDEntry::new(
            tags::PLANAR_CONFIGURATION,
            field_types::SHORT,
            1,
            planar_config::CHUNKY as u64)
        );
    }

    /// Add common tags for a grayscale image
    ///
    /// Creates a simple grayscale (black and white) image with
    /// the specified bit depth. Great for elevation data, masks,
    /// or simple monochrome images.
    pub fn add_basic_gray_tags(
        ifd: &mut IFD,
        width: u32,
        height: u32,
        bits_per_sample: u16
    ) {
        info!("Adding basic grayscale tags for {}x{} image, {} bits", width, height, bits_per_sample);

        // Basic image dimensions
        ifd.add_entry(IFDEntry::new(
            tags::IMAGE_WIDTH,
            field_types::LONG,
            1,
            width as u64)
        );

        ifd.add_entry(IFDEntry::new(
            tags::IMAGE_LENGTH,
            field_types::LONG,
            1,
            height as u64)
        );

        // For grayscale, there's just one channel with the specified bit depth
        ifd.add_entry(IFDEntry::new(
            tags::BITS_PER_SAMPLE,
            field_types::SHORT,
            1,
            bits_per_sample as u64)
        );

        // No compression
        ifd.add_entry(IFDEntry::new(
            tags::COMPRESSION,
            field_types::SHORT,
            1,
            compression::NONE as u64)
        );

        // BlackIsZero - standard grayscale interpretation
        // where 0 is black and max value is white
        ifd.add_entry(IFDEntry::new(
            tags::PHOTOMETRIC_INTERPRETATION,
            field_types::SHORT,
            1,
            photometric::BLACK_IS_ZERO as u64)
        );

        // Just one sample (channel) per pixel
        ifd.add_entry(IFDEntry::new(
            tags::SAMPLES_PER_PIXEL,
            field_types::SHORT,
            1,
            1)
        );

        // Single strip for simplicity
        ifd.add_entry(IFDEntry::new(
            tags::ROWS_PER_STRIP,
            field_types::LONG,
            1,
            height as u64)
        );

        // Add min/max sample values if they don't already exist
        // These help viewers interpret the dynamic range correctly
        if !ifd.has_tag(tags::MIN_SAMPLE_VALUE) {
            ifd.add_entry(IFDEntry::new(
                tags::MIN_SAMPLE_VALUE,
                field_types::SHORT,
                1,
                0)
            );  // Minimum value is 0
        }

        if !ifd.has_tag(tags::MAX_SAMPLE_VALUE) {
            // Maximum value depends on bit depth: 2^bits - 1
            // For 8-bit this is 255, for 16-bit it's 65535
            let max_value = (1u64 << bits_per_sample) - 1;
            ifd.add_entry(IFDEntry::new(
                tags::MAX_SAMPLE_VALUE,
                field_types::SHORT,
                1,
                max_value)
            );
        }
    }

    /// Add color map for a palette-color image
    ///
    /// Palette images use indexed colors - each pixel is just an index
    /// into a lookup table (colormap) which defines the actual RGB colors.
    /// This is great for images with limited colors, like GIS classification maps.
    pub fn add_color_map(
        ifd: &mut IFD,
        external_data: &mut std::collections::HashMap<(usize, u16), Vec<u8>>,
        ifd_index: usize,
        color_map: &[u16]
    ) {
        // The color map needs to have values for all three channels (R,G,B)
        // so its length must be divisible by 3
        if color_map.len() % 3 != 0 {
            warn!("Color map length {} is not divisible by 3", color_map.len());
            return;
        }

        info!("Adding color map with {} entries", color_map.len() / 3);

        // If there's already a PhotometricInterpretation tag, remove it
        // since we're changing the interpretation to palette mode
        let existing_idx = ifd.entries.iter().position(|e| e.tag == tags::PHOTOMETRIC_INTERPRETATION);
        if let Some(idx) = existing_idx {
            ifd.entries.remove(idx);
        }

        // Set the palette (indexed) color mode
        ifd.add_entry(IFDEntry::new(
            tags::PHOTOMETRIC_INTERPRETATION,
            field_types::SHORT,
            1,
            photometric::PALETTE as u64)
        );

        // The color map is stored as all red values, then all green values,
        // then all blue values (not as RGB triplets)
        let mut colormap_data = Vec::with_capacity(color_map.len() * 2);
        for &value in color_map.iter() {
            colormap_data.extend_from_slice(&value.to_le_bytes());
        }

        // Add the ColorMap tag and store its data for later writing
        ifd.add_entry(IFDEntry::new(
            tags::COLOR_MAP,
            field_types::SHORT,
            color_map.len() as u64,
            0)
        );
        external_data.insert((ifd_index, tags::COLOR_MAP), colormap_data);
    }
}