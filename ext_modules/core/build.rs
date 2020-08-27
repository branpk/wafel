use std::{
    collections::HashSet,
    error::Error,
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
    process::Command,
    str,
};
use walkdir::WalkDir;

fn main() {
    compile_shaders().unwrap();
}

fn compile_shaders() -> Result<(), Box<dyn Error>> {
    let bin_dir = Path::new("bin");
    let mut output_files = HashSet::new();

    for src_path in shader_files()? {
        println!(
            "cargo:rerun-if-changed={}",
            src_path.to_str().expect("path is not unicode")
        );

        let mut dst_path = bin_dir.join(&src_path);
        dst_path.set_extension(match src_path.extension() {
            Some(ext) => format!("{}.spv", ext.to_str().unwrap()),
            None => "spv".to_owned(),
        });

        fs::create_dir_all(dst_path.parent().unwrap())?;

        let output = Command::new("glslc")
            .arg("-c")
            .arg(src_path)
            .arg("-o")
            .arg(&dst_path)
            .output()?;

        if !output.status.success() {
            panic!("{}", str::from_utf8(&output.stderr)?);
        }

        output_files.insert(dst_path);
    }

    if bin_dir.is_dir() {
        remove_unexpected_files(bin_dir, &output_files)?;
        remove_empty_dirs(bin_dir)?;
    }

    Ok(())
}

fn shader_files() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let index_filename = "shaders/index.txt";
    println!("cargo:rerun-if-changed={}", index_filename);

    let file = fs::File::open(index_filename)?;
    Ok(io::BufReader::new(file)
        .lines()
        .collect::<Result<Vec<String>, _>>()?
        .iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .map(|s| Path::new("shaders").join(s))
        .collect())
}

fn remove_unexpected_files(
    bin_dir: &Path,
    output_files: &HashSet<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let shaders_dir = bin_dir.join("shaders");
    if !shaders_dir.is_dir() {
        return Ok(());
    }
    for file in WalkDir::new(shaders_dir) {
        let file = file?;
        let path = file.path();
        if !path.is_file() {
            continue;
        }

        if !output_files.contains(path) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn remove_empty_dirs(bin_dir: &Path) -> Result<(), Box<dyn Error>> {
    for dir in WalkDir::new(bin_dir) {
        let dir = dir?;
        let path = dir.path();
        if !path.is_dir() {
            continue;
        }

        // Fails if directory is nonempty
        let _ = fs::remove_dir(path);
    }
    Ok(())
}
