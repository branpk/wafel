//! A program for calculating SM64 variable layouts and types.
//!
//! The output of this program is json in the format of [DataLayout](wafel_layout).
//!
//! It can either parse a libsm64 DLL, or output the layout for an N64 version
//! of the game.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use std::{fs, process};

use clap::{App, Arg, ArgGroup};
use wafel_layout::{load_sm64_n64_layout, DllLayout};

fn main() {
    let matches = App::new("sm64_layout")
        .about("Outputs SM64 data layout as json")
        .arg(
            Arg::with_name("libsm64")
                .long("libsm64")
                .value_name("FILE")
                .help("path to sm64_xx.dll (NOT the .locked file)"),
        )
        .arg(
            Arg::with_name("n64")
                .long("n64")
                .value_name("VERSION")
                .help("version of SM64 (us, jp, eu, or sh)"),
        )
        .group(
            ArgGroup::with_name("input-option")
                .args(&["libsm64", "n64"])
                .required(true)
                .multiple(false),
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

    let layout_json;
    if let Some(libsm64_filename) = matches.value_of("libsm64") {
        let mut layout = DllLayout::read(libsm64_filename).unwrap_or_else(|error| {
            eprintln!("Error while parsing {}: {}", libsm64_filename, error);
            process::exit(1);
        });

        layout
            .data_layout
            .add_sm64_extras()
            .unwrap_or_else(|error| {
                eprintln!("Error while loading SM64 extensions: {}", error);
                process::exit(1);
            });

        layout_json = serde_json::to_string_pretty(&layout).unwrap_or_else(|error| {
            eprintln!("Error while serializing: {}", error);
            process::exit(1);
        });
    } else if let Some(version) = matches.value_of("n64") {
        let layout = load_sm64_n64_layout(version).unwrap_or_else(|error| {
            eprintln!("Error while fetching N64 layout: {}", error);
            process::exit(1);
        });

        layout_json = serde_json::to_string_pretty(&layout).unwrap_or_else(|error| {
            eprintln!("Error while serializing: {}", error);
            process::exit(1);
        });
    } else {
        unreachable!();
    }

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
