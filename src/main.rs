use clap::{Arg, Command as ClapCommand, ArgAction};
use std::process;
use log::error;

// Import from your library
use rasterkit::utils::logger::Logger;
use rasterkit::commands::{CommandFactory, RasterkitCommandFactory};

fn main() {
    let matches = ClapCommand::new("RasterKit")
        .version("1.0")
        .author("Maurice Schilpp")
        .about("Analyze TIFF/BigTIFF file structure")
        .arg(
            Arg::new("input")
                .help("Input TIFF file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("extract")
                .short('e')
                .long("extract")
                .help("Extract image data")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output image file")
                .value_name("FILE")
                .required(false),
        )
        .arg(
            Arg::new("bbox")
                .long("bbox")
                .help("Bounding box for extraction (minx,miny,maxx,maxy)")
                .value_name("BBOX")
                .required(false),
        )
        .arg(
            Arg::new("epsg")
                .long("epsg")
                .help("EPSG code for bounding box coordinates")
                .value_name("CODE")
                .default_value("4326")
                .required(false),
        )
        .arg(
            Arg::new("crs")
                .long("crs")
                .help("EPSG code for coordinates (e.g., 4326 for WGS84, 3857 for Web Mercator)")
                .value_name("CODE")
                .default_value("4326")
                .required(false),
        )
        .arg(
            Arg::new("proj")
                .long("proj")
                .help("Target projection EPSG code for reprojection")
                .value_name("CODE")
                .required(false),
        )
        .arg(
            Arg::new("coordinate")
                .long("coordinate")
                .help("Geographic coordinate for extraction in 'x,y' format")
                .value_name("COORDINATE")
                .required(false),
        )
        .arg(
            Arg::new("radius")
                .long("radius")
                .help("Radius in meters around coordinate")
                .value_name("METERS")
                .required(false),
        )
        .arg(
            Arg::new("shape")
                .long("shape")
                .help("Shape for coordinate extraction (circle or square)")
                .value_name("SHAPE")
                .default_value("square")
                .required(false),
        )
        .arg(
            Arg::new("extract-array")
                .long("extract-array")
                .help("Extract raw array data instead of image")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("array-format")
                .long("array-format")
                .help("Format for array output (csv, json, npy)")
                .value_name("FORMAT")
                .default_value("csv")
                .required(false),
        )
        .arg(
            Arg::new("filter")
                .long("filter")
                .help("Filter values to show only the specified range (e.g., '15,160')")
                .value_name("MIN,MAX")
                .required(false),
        )
        .arg(
            Arg::new("filter-transparency")
                .long("filter-transparency")
                .help("Make filtered pixels transparent instead of black")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("colormap-output")
                .long("colormap-output")
                .help("Extract colormap from input TIFF to this file")
                .value_name("FILE")
                .required(false),
        )
        .arg(
            Arg::new("colormap-input")
                .long("colormap-input")
                .help("Apply this colormap to the extracted image")
                .value_name("FILE")
                .required(false),
        )
        .arg(
            Arg::new("convert")
                .short('c')
                .long("convert")
                .help("Convert to different compression format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("compression")
                .long("compression")
                .help("Target compression code (1=none, 8=deflate, 14=zstd)")
                .value_name("CODE")
                .required(false),
        )
        .arg(
            Arg::new("compression-name")
                .long("compression-name")
                .help("Target compression by name (none, deflate, zstd)")
                .value_name("NAME")
                .required(false),
        )
        .get_matches();

    let log_file = "rasterkit.log";
    let logger = match Logger::new(log_file) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error initializing logger: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = Logger::init_global_logger("rasterkit-global.log") {
        eprintln!("Error setting up global logger: {}", e);
        process::exit(1);
    }

    let factory = RasterkitCommandFactory::new();

    let command_result = factory.create_command(&matches, &logger);
    match command_result {
        Ok(command) => {
            if let Err(e) = command.execute() {
                error!("Command execution error: {}", e);
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        },
        Err(e) => {
            error!("Failed to create command: {}", e);
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };
}