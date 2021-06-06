use std::{fs, process};

use clap::{App, Arg, ArgGroup};
use wafel_layout::{load_layout_from_dll, load_sm64_extras};

fn main() {
    let matches = App::new("libsm64_layout")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("path to sm64_xx.dll (NOT the .locked file)")
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("path to the output JSON file"),
        )
        .arg(
            Arg::with_name("stdout")
                .long("stdout")
                .help("print the JSON text to stdout"),
        )
        .group(
            ArgGroup::with_name("output-option")
                .args(&["output", "stdout"])
                .required(true)
                .multiple(true),
        )
        .get_matches();

    let input_filename = matches.value_of("input").unwrap();

    let mut layout = load_layout_from_dll(input_filename).unwrap_or_else(|error| {
        eprintln!("Error while parsing {}: {}", input_filename, error);
        process::exit(1);
    });

    load_sm64_extras(&mut layout.data_layout).unwrap_or_else(|error| {
        eprintln!("Error while loading SM64 extensions: {}", error);
        process::exit(1);
    });

    let layout_json = serde_json::to_string_pretty(&layout).unwrap_or_else(|error| {
        eprintln!("Error while serializing: {}", error);
        process::exit(1);
    });

    if let Some(output_filename) = matches.value_of("output") {
        fs::write(output_filename, &layout_json).unwrap_or_else(|error| {
            eprintln!("Error while writing {}: {}", output_filename, error);
            process::exit(1);
        });
    }
    if matches.is_present("stdout") {
        print!("{}", layout_json);
    }
}
