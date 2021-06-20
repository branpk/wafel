use std::{fs, path::Path, sync::Arc};

use pwbox::{sodium::Sodium, ErasedPwBox, Eraser, Suite};

use crate::Error;

/// Lock a libsm64 DLL so that it requires a ROM to open.
///
/// # Panics
///
/// Panics if encryption or file IO fails.
#[track_caller]
pub fn lock_libsm64(input_filename: &str, output_filename: &str, rom_filename: &str) {
    if let Err(error) = try_lock_libsm64(input_filename, output_filename, rom_filename) {
        panic!("Error:\n  {}\n", error);
    }
}

/// Lock a libsm64 DLL so that it requires a ROM to open.
///
/// Returns an error if encryption or file IO fails.
pub fn try_lock_libsm64(
    input_filename: &str,
    output_filename: &str,
    rom_filename: &str,
) -> Result<(), Error> {
    let input = read_file(input_filename)?;
    let rom = rom_to_z64(&read_file(rom_filename)?)?;

    let pwbox = Sodium::build_box(&mut rand::thread_rng())
        .seal(&rom, &input)
        .map_err(|_| Error::Libsm64EncryptionError)?;

    let mut eraser = Eraser::new();
    eraser.add_suite::<Sodium>();
    let erased_pwbox = eraser
        .erase(&pwbox)
        .map_err(|_| Error::Libsm64EncryptionError)?;

    let output = serde_json::to_vec(&erased_pwbox).expect("failed to serialize .locked");

    write_file(output_filename, &output)?;
    Ok(())
}

/// Unlock a libsm64 DLL using a ROM.
///
/// # Panics
///
/// Panics if encryption or file IO fails.
#[track_caller]
pub fn unlock_libsm64(input_filename: &str, output_filename: &str, rom_filename: &str) {
    if let Err(error) = try_unlock_libsm64(input_filename, output_filename, rom_filename) {
        panic!("Error:\n  {}\n", error);
    }
}

/// Unlock a libsm64 DLL using a ROM.
///
/// Returns an error if encryption or file IO fails.
pub fn try_unlock_libsm64(
    input_filename: &str,
    output_filename: &str,
    rom_filename: &str,
) -> Result<(), Error> {
    let input = read_file(input_filename)?;
    let rom = rom_to_z64(&read_file(rom_filename)?)?;

    let erased_pwbox: ErasedPwBox =
        serde_json::from_slice(&input).map_err(|_| Error::Libsm64DecryptionError)?;

    let mut eraser = Eraser::new();
    eraser.add_suite::<Sodium>();
    let pwbox = eraser
        .restore(&erased_pwbox)
        .map_err(|_| Error::Libsm64DecryptionError)?;

    let output = pwbox
        .open(&rom)
        .map_err(|_| Error::Libsm64DecryptionError)?
        .to_vec();

    write_file(output_filename, &output)?;
    Ok(())
}

fn read_file(filename: &str) -> Result<Vec<u8>, Error> {
    fs::read(filename).map_err(|error| Error::FileReadError {
        filename: filename.to_string(),
        error: Arc::new(error),
    })
}

fn write_file(filename: &str, bytes: &[u8]) -> Result<(), Error> {
    if let Some(dir) = Path::new(filename).parent() {
        fs::create_dir_all(dir).map_err(|error| Error::FileWriteError {
            filename: filename.to_string(),
            error: Arc::new(error),
        })?;
    }
    fs::write(filename, bytes).map_err(|error| Error::FileWriteError {
        filename: filename.to_string(),
        error: Arc::new(error),
    })
}

fn rom_to_z64(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.len() < 4 || bytes.len() % 4 != 0 {
        return Err(Error::InvalidRom);
    }
    let bom = &bytes[0..4];
    match bom {
        b"\x80\x37\x12\x40" => Ok(bytes.to_vec()),
        b"\x37\x80\x40\x12" => Ok(swap_bytes_16(bytes)),
        b"\x40\x12\x37\x80" => Ok(swap_bytes_32(bytes)),
        _ => Err(Error::InvalidRom),
    }
}

fn swap_bytes_16(bytes: &[u8]) -> Vec<u8> {
    assert_eq!(bytes.len() % 2, 0);
    let mut output = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks_exact(2) {
        output.extend([chunk[1], chunk[0]]);
    }
    output
}

fn swap_bytes_32(bytes: &[u8]) -> Vec<u8> {
    assert_eq!(bytes.len() % 4, 0);
    let mut output = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks_exact(4) {
        output.extend([chunk[3], chunk[2], chunk[1], chunk[0]]);
    }
    output
}
