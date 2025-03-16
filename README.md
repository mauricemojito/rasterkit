# 🛠️ RasterKit

> A powerful Rust toolkit for working with geospatial raster data

A powerful Rust toolkit for analyzing, manipulating, and extracting data from TIFF and GeoTIFF files. RasterKit provides both a command-line interface and a programmer-friendly API for working with raster geospatial data.

## ✨ Features

-   📊 **Data Analysis**: Inspect and analyze TIFF/GeoTIFF file structures and metadata

-   🗺️ **Data Extraction**: Extract regions using pixel coordinates or geographic boundaries

-   🎨 **Colormap Support**: Apply and extract colormaps to visualize raster data

-   📈 **Array Data**: Extract raw numeric data for analysis in CSV, JSON, or NumPy formats

-   🗜️ **Compression Conversion**: Convert between different compression formats

-   🚀 **High Performance**: Built with Rust for efficient processing of large files

-   🧩 **Extensible**: Plugin architecture for adding new formats and features


## 📦 Installation

Clone the repository and build with Cargo:

```
git clone https://github.com/mauricemojito/rasterkit.git
cd rasterkit
cargo build --release
```

## 🚀 Usage

### Analyzing a TIFF File

Get detailed information about a TIFF file's structure:

```
rasterkit input.tif
```

### Image Extraction

Extract a region from a TIFF file:

```
# Extract entire image
rasterkit input.tif --extract --output extracted.tif

# Extract a region using a bounding box (Web Mercator coordinates)
rasterkit input.tif --extract --output region.tif --bbox=-12626828,7529611,-12603877,7508004 --epsg=3857
```

### Array Data Extraction

Extract raw data as arrays for analysis:

```
# Extract as CSV
rasterkit input.tif --extract-array --output data.csv

# Extract as JSON
rasterkit input.tif --extract-array --array-format=json --output data.json

# Extract as NumPy array
rasterkit input.tif --extract-array --array-format=npy --output data.npy
```

### Working with Colormaps

Add or extract color schemes for raster visualization:

```
# Extract a colormap from a TIFF
rasterkit input.tif --extract --output output.tif --colormap-output colormap.sld

# Apply a colormap during extraction
rasterkit input.tif --extract --output colored.tif --colormap-input colormap.sld
```

### Converting Compression

Transform between different compression formats:

```
# Convert to uncompressed format
rasterkit input.tif --convert --output uncompressed.tif --compression-name=none

# Convert to ZStd compression
rasterkit input.tif --convert --output compressed.tif --compression-name=zstd
```

## 🧠 API Usage

RasterKit can also be used as a library in your Rust projects:

```
use rasterkit::{RasterKit, Region, BoundingBox};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new RasterKit instance
    let kit = RasterKit::new(None)?;

    // Analyze a TIFF file
    let analysis = kit.analyze("input.tif")?;
    println!("{}", analysis);

    // Extract a region from the TIFF
    kit.extract(
        "input.tif",
        "output.tif",
        Some((100, 100, 500, 500)), // region: x, y, width, height
        None, // bbox
        None, // epsg
    )?;

    // Extract raw array data
    kit.extract_to_array(
        "input.tif",
        "data.csv",
        "csv",
        None, // extract entire image
    )?;

    Ok(())
}
```

## 🛣️ Roadmap

-   🌈 Support for more raster formats (GeoPackage, NetCDF, etc.)

-   🔮 Data visualization features

-   ⚡ Parallel processing for even faster performance

-   🧪 Machine learning integration


## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](https://opensource.org/licenses/MIT) file for details.