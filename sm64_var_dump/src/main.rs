use std::{
    fs,
    io::{stdout, BufRead, BufReader, BufWriter, Write},
    process,
};

use clap::{App, Arg, ArgGroup, ArgMatches};
use wafel_api::{try_load_m64, Game, Value};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

// TODO: None, arrays, structs

pub fn main() {
    let matches = App::new("sm64_var_dump")
        .about("Reads SM64 variables in an emulator or libsm64")
        .after_help(
            "
To use libsm64: --libsm64 <FILE> --m64 <FILE>
To attach to an emulator: --pid <PID> --base <ADDR> --version <VERSION>
"
            .trim(),
        )
        // Game options
        .arg(
            Arg::with_name("libsm64")
                .long("libsm64")
                .value_name("FILE")
                .help("path to sm64_xx.dll (NOT the .locked file)")
                .requires("m64"),
        )
        .arg(
            Arg::with_name("m64")
                .long("m64")
                .value_name("FILE")
                .help("path to .m64 TAS to replay (only for --libsm64)")
                .requires("libsm64"),
        )
        .arg(
            Arg::with_name("pid")
                .long("pid")
                .value_name("PID")
                .help("PID of emulator process to attach to")
                .requires_all(&["base", "version"]),
        )
        .arg(
            Arg::with_name("base")
                .long("base")
                .value_name("ADDR")
                .help("base address (in hex) of the emulator process")
                .requires("pid"),
        )
        .arg(
            Arg::with_name("version")
                .long("version")
                .value_name("VERSION")
                .help("SM64 version to use (us/jp/eu/sh). Only needed for emulator attachment"),
        )
        .group(
            ArgGroup::with_name("game-option")
                .args(&["libsm64", "pid"])
                .required(true)
                .multiple(false),
        )
        // Variable options
        .arg(
            Arg::with_name("vars")
                .long("vars")
                .alias("var")
                .value_name("VARS")
                .help("variables to dump in wafel path format")
                .min_values(1),
        )
        .arg(
            Arg::with_name("var_file")
                .long("var_file")
                .alias("vars_file")
                .value_name("FILE")
                .help("path to file containing one variable per line"),
        )
        .group(
            ArgGroup::with_name("var-options")
                .args(&["vars", "var_file"])
                .required(true)
                .multiple(true),
        )
        // Output options
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("path to the output CSV file"),
        )
        .arg(
            Arg::with_name("stdout")
                .long("stdout")
                .help("print the CSV text to stdout"),
        )
        .group(
            ArgGroup::with_name("output-option")
                .args(&["output", "stdout"])
                .required(true)
                .multiple(true),
        )
        .get_matches();

    run(&matches).unwrap_or_else(|error| {
        eprintln!("Error: {}", error);
        process::exit(1);
    });
}

fn run(matches: &ArgMatches) -> Result<()> {
    let mut streams = open_streams(matches)?;
    let vars = get_vars(matches)?;

    print_headers(&mut streams, &vars)?;

    if let Some(libsm64_path) = matches.value_of("libsm64") {
        eprintln!("Loading {}", libsm64_path);
        let mut game = unsafe { Game::try_new(libsm64_path)? };

        let m64_path = matches.value_of("m64").unwrap();
        let (_, inputs) = try_load_m64(m64_path)?;

        eprintln!("Running {} ({} frames)", m64_path, inputs.len());

        print_vars(&mut streams, &vars, |var| game.try_read(var))?;

        for input in &inputs {
            game.try_write("gControllerPads[0].button", input.buttons.into())?;
            game.try_write("gControllerPads[0].stick_x", input.stick_x.into())?;
            game.try_write("gControllerPads[0].stick_y", input.stick_y.into())?;
            game.advance();

            print_vars(&mut streams, &vars, |var| game.try_read(var))?;
        }

        eprintln!("Finished");
    }

    Ok(())
}

fn open_streams(matches: &ArgMatches) -> Result<Vec<Box<dyn Write>>> {
    let mut streams: Vec<Box<dyn Write>> = Vec::new();
    if let Some(filename) = matches.value_of("output") {
        let file = fs::File::create(filename)?;
        let writer = BufWriter::new(file);
        streams.push(Box::new(writer));
    }
    if matches.is_present("stdout") {
        streams.push(Box::new(stdout()))
    }
    Ok(streams)
}

fn get_vars(matches: &ArgMatches) -> Result<Vec<String>> {
    let mut var_names = Vec::new();
    if let Some(vars) = matches.values_of("vars") {
        for var in vars {
            var_names.push(var.to_string());
        }
    }
    if let Some(var_file) = matches.value_of("var_file") {
        let file = fs::File::open(var_file)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let var = line?.trim().to_string();
            if !var.is_empty() {
                var_names.push(var);
            }
        }
    }
    Ok(var_names)
}

fn print_headers(streams: &mut [Box<dyn Write>], vars: &[String]) -> Result<()> {
    for f in streams {
        writeln!(f, "{}", format_csv_line(vars))?;
        f.flush()?;
    }
    Ok(())
}

fn print_vars(
    streams: &mut [Box<dyn Write>],
    vars: &[String],
    mut read: impl FnMut(&str) -> std::result::Result<Value, wafel_api::Error>,
) -> Result<()> {
    let mut values = Vec::new();
    for var in vars {
        let value = read(var)?;
        let formatted = match value {
            Value::None => String::new(),
            v => v.to_string(),
        };
        values.push(formatted);
    }

    for f in streams {
        writeln!(f, "{}", format_csv_line(&values))?;
        f.flush()?;
    }

    Ok(())
}

fn format_csv_line(cells: &[String]) -> String {
    cells
        .iter()
        .map(|s| escape_csv(s))
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_csv(s: &str) -> String {
    let s = s.replace('"', "\"\"");
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s)
    } else {
        s
    }
}
