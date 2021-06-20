//! A standalone executable for locking/unlocking a libsm64 DLL.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use std::process;

use clap::{App, Arg, ArgGroup};
use wafel_api::{try_lock_libsm64, try_unlock_libsm64};

fn main() {
    let matches = App::new("libsm64_lock")
        .about("Locks or unlocks a libsm64 DLL")
        .arg(Arg::with_name("lock").long("lock"))
        .arg(Arg::with_name("unlock").long("unlock"))
        .group(
            ArgGroup::with_name("mode")
                .args(&["lock", "unlock"])
                .required(true)
                .multiple(false),
        )
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("path to the input file")
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("path to the output file")
                .required(true),
        )
        .arg(
            Arg::with_name("rom")
                .short("r")
                .long("rom")
                .value_name("FILE")
                .help("path to the SM64 ROM")
                .required(true),
        )
        .get_matches();

    let input_filename = matches.value_of("input").unwrap();
    let output_filename = matches.value_of("output").unwrap();
    let rom_filename = matches.value_of("rom").unwrap();

    let result = if matches.is_present("lock") {
        try_lock_libsm64(input_filename, output_filename, rom_filename)
    } else {
        try_unlock_libsm64(input_filename, output_filename, rom_filename)
    };

    if let Err(error) = result {
        eprintln!("Error:\n  {}", error);
        process::exit(1);
    }
}
