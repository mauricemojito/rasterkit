//! XML utility functions
//!
//! Simple XML manipulation utilities for working with metadata tags
//! and other XML content in TIFF files.

/// Replace a specific tag in XML metadata
///
/// This is a simpler approach than trying to parse XML properly
pub fn replace_xml_tag(xml: &str, item_name: &str, new_value: &str) -> String {
    // Find the tag pattern we want to replace
    let start_pattern = format!("<Item name=\"{}\"", item_name);
    let end_pattern = "</Item>";

    // If we can't find the patterns, return a freshly created XML
    if !xml.contains(&start_pattern) || !xml.contains(end_pattern) {
        return format!("<GDALMetadata>\n  <Item name=\"{}\">{}</Item>\n</GDALMetadata>",
                       item_name, new_value);
    }

    // Split by the start pattern
    let parts: Vec<&str> = xml.split(&start_pattern).collect();
    // We expect at least 2 parts (before and after)
    let before = parts[0];
    let after_start = parts[1];

    // Now find the end tag in the second part
    let after_parts: Vec<&str> = after_start.split(end_pattern).collect();
    // Again we expect at least 2 parts
    if after_parts.len() < 2 {
        return format!("<GDALMetadata>\n  <Item name=\"{}\">{}</Item>\n</GDALMetadata>",
                       item_name, new_value);
    }

    // Everything between the start and end pattern is the old value,
    // which we'll replace with our new value
    let after = &after_parts[1..].join(end_pattern);

    // Reconstruct with our new value
    format!("{}<Item name=\"{}\">{}</Item>{}", before, item_name, new_value, after)
}

/// Add an item to GDALMetadata XML, before the closing tag
pub fn add_to_gdal_metadata(xml: &str, item: &str) -> String {
    if xml.contains("</GDALMetadata>") {
        // Insert before the closing tag
        let parts: Vec<&str> = xml.split("</GDALMetadata>").collect();
        format!("{}  {}\n</GDALMetadata>", parts[0], item)
    } else {
        // Metadata is missing the closing tag, add it
        format!("{}\n  {}\n</GDALMetadata>", xml, item)
    }
}