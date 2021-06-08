//! A standalone executable for parsing a libsm64 DLL and printing its
//! [DataLayout](wafel_layout) as json.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use std::{fs, process};

use clap::{App, Arg};
use wafel_layout::DllLayout;

fn main() {
    let matches = App::new("libsm64_layout")
        .about("Parses a libsm64 DLL and prints its data layout as json")
        .arg(
            Arg::with_name("input")
                .short("i")
                .value_name("FILE")
                .help("path to sm64_xx.dll (NOT the .locked file)")
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .value_name("FILE")
                .help("path to the output JSON file")
                .required(true),
        )
        .get_matches();

    let input_filename = matches.value_of("input").unwrap();
    let output_filename = matches.value_of("output").unwrap();

    let mut layout = DllLayout::read(input_filename).unwrap_or_else(|error| {
        eprintln!("Error while parsing {}: {}", input_filename, error);
        process::exit(1);
    });

    layout
        .data_layout
        .add_sm64_extras()
        .unwrap_or_else(|error| {
            eprintln!("Error while loading SM64 extensions: {}", error);
            process::exit(1);
        });

    let layout_json = serde_json::to_string_pretty(&layout).unwrap_or_else(|error| {
        eprintln!("Error while serializing: {}", error);
        process::exit(1);
    });

    fs::write(output_filename, &layout_json).unwrap_or_else(|error| {
        eprintln!("Error while writing {}: {}", output_filename, error);
        process::exit(1);
    });
}
