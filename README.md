
# 🛠️ RasterKit

**A powerful Rust toolkit for working with geospatial raster data**

RasterKit is your go-to toolkit for working with geospatial raster data. Built with Rust for speed and reliability, it lets you analyze, manipulate, and extract data from TIFF and GeoTIFF files with ease, whether you're using the command-line interface or the API.

## ✨ Features

-   📊 **Data Analysis:** Peek inside TIFF/GeoTIFF files to understand their structure and metadata

-   🗺️ **Flexible Extraction:** Grab exactly the region you need using pixel coordinates, bounding boxes, or even a point and radius

-   🔵 **Shape Options:** Extract square or circular regions - perfect for analyzing areas around points of interest

-   🎨 **Colormap Magic:** Apply colormaps to turn grayscale data into beautiful visualizations

-   🎯 **Value Filtering:** Show only the values that matter by filtering pixel ranges

-   📈 **Data for Analysis:** Pull out raw numeric data as CSV, JSON, or NumPy arrays for further analysis

-   🗜️ **Smart Compression:** Convert between compression formats to optimize for size or speed

-   🚀 **Blazing Fast:** Written in Rust to handle even your largest datasets efficiently

-   🧩 **Build On It:** Extensible architecture makes it easy to add new formats and capabilities


## 📦 Installation

Clone the repository and build with Cargo:

```
git clone https://github.com/mauricemojito/rasterkit.git
cd rasterkit
cargo build --release
```

## 🚀 Usage

### Analyzing a TIFF File

Take a peek at what's inside your TIFF:

```
rasterkit input.tif
```

Want more details? Just add `--verbose`:

```
rasterkit input.tif --verbose
```

### Image Extraction

Extract regions in multiple ways:

**Extract the entire image:**

```
rasterkit input.tif --extract --output extracted.tif
```

**Extract a rectangle of pixels:**

```
rasterkit input.tif --extract --output region.tif --region=100,100,500,500
```

**Extract a geographic bounding box (Web Mercator):**

```
rasterkit input.tif --extract --output region.tif --bbox=-12626828,7529611,-12603877,7508004 --crs=3857
```

**Extract area around a point (WGS84 coordinates):**

```
rasterkit input.tif --extract --output point_extract.tif --coordinate="-109.22624,56.13484" --radius=5000 --crs=4326 --shape=square
```

**Extract a circular region:**

```
rasterkit input.tif --extract --output circle.png --coordinate="-109.22624,56.13484" --radius=5000 --crs=4326 --shape=circle
```

### Value Filtering

Filter specific value ranges in your data:

**Show only values between 15 and 160:**

```
rasterkit input.tif --extract --output filtered.tif --filter="15,160"
```

**Make values outside the range transparent:**

```
rasterkit input.tif --extract --output filtered.png --filter="15,160" --filter-transparency
```

### Reprojection

Reproject your data to a different coordinate system:

```
rasterkit input.tif --extract --output reprojected.tif --coordinate="-109.22624,56.13484" --crs=4326 --proj=3857 --radius=5000
```

### Array Data Extraction

Extract raw data for external analysis:

**Export to CSV:**

```
rasterkit input.tif --extract-array --output data.csv
```

**Export to JSON:**

```
rasterkit input.tif --extract-array --array-format=json --output data.json
```

**Export to NumPy array:**

```
rasterkit input.tif --extract-array --array-format=npy --output data.npy
```

### Working with Colormaps

Apply colormaps to your raster data:

**Extract and save a colormap:**

```
rasterkit input.tif --colormap-output=colormap.sld
```

**Apply a colormap when extracting data:**

```
rasterkit input.tif --extract --output colored.tif --colormap-input=colormap.sld
```

### Converting Compression

Optimize raster file compression:

**Remove compression:**

```
rasterkit input.tif --convert --output uncompressed.tif --compression-name=none
```

**Use Deflate compression:**

```
rasterkit input.tif --convert --output compressed.tif --compression-name=deflate
```

**Use ZStd compression:**

```
rasterkit input.tif --convert --output compressed.tif --compression-name=zstd
```

## 🧠 API Usage

Use RasterKit in your Rust code:

```
use rasterkit::api::RasterKit;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kit = RasterKit::new(Some("rasterkit.log"))?;

    let analysis = kit.analyze("input.tif")?;
    println!("{}", analysis);

    kit.extract(
        "input.tif", "output.tif",
        Some((100, 100, 500, 500)), None, None, None, None, None,
        None, None, false
    )?;

    kit.extract(
        "input.tif", "geo_output.png",
        None, None, Some("-109.22624,56.13484"), Some(5000.0),
        Some("circle"), Some(4326), Some("colormap.sld"),
        Some("15,160"), true
    )?;

    kit.extract_to_array("input.tif", "data.csv", "csv", None)?;

    Ok(())
}
```

## 🛣️ Roadmap

-   🌈 Support for more raster formats (GeoPackage, NetCDF, etc.)

-   🔮 Data visualization features

-   ⚡ Parallel processing for even faster performance

-   🧪 Machine learning integration


## 🤝 Contributing

Contributions are welcome! If you find a bug, have an idea for a new feature, or want to improve the documentation, open a pull request.

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.