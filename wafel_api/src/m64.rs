use std::{
    fmt, fs,
    io::{self, BufWriter, Write},
    path::Path,
    str,
    sync::Arc,
};

use crate::Error;

/// SM64 game versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SM64Version {
    /// The original Japanese release.
    JP,
    /// US version.
    US,
    /// PAL version.
    EU,
    /// Shindou version.
    SH,
}

impl SM64Version {
    /// Return all game versions.
    pub fn all() -> &'static [SM64Version] {
        &[Self::JP, Self::US, Self::EU, Self::SH]
    }

    fn crc_code(self) -> u32 {
        match self {
            SM64Version::JP => 0x0e3daa4e,
            SM64Version::US => 0xff2b5a63,
            SM64Version::EU => 0x36f03ca0,
            SM64Version::SH => 0xa8a4fbd6,
        }
    }

    fn country_code(self) -> u8 {
        match self {
            SM64Version::JP => b'J',
            SM64Version::US => b'E',
            SM64Version::EU => b'P',
            SM64Version::SH => b'J',
        }
    }
}

impl fmt::Display for SM64Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SM64Version::JP => write!(f, "JP"),
            SM64Version::US => write!(f, "US"),
            SM64Version::EU => write!(f, "EU"),
            SM64Version::SH => write!(f, "SH"),
        }
    }
}

/// Metadata for a .m64 TAS.
#[derive(Debug, Clone)]
pub struct M64Metadata {
    /// The rom CRC code.
    crc_code: u32,
    /// The game country code.
    country_code: u8,
    /// The TAS authors - UTF-8 with max size 222 bytes.
    author: String,
    /// The TAS description - UTF-8 with max size 256 bytes.
    description: String,
    /// The number of rerecords.
    rerecords: u32,
}

impl M64Metadata {
    /// Create a new metadata object with the given CRC and country code.
    pub fn new(crc_code: u32, country_code: u8) -> Self {
        Self {
            crc_code,
            country_code,
            author: String::new(),
            description: String::new(),
            rerecords: 0,
        }
    }

    /// Create a new metadata object using the CRC and country code for the given SM64
    /// version.
    pub fn with_version(version: SM64Version) -> Self {
        Self::new(version.crc_code(), version.country_code())
    }

    /// Get the CRC code.
    pub fn crc_code(&self) -> u32 {
        self.crc_code
    }

    /// Set the CRC code.
    pub fn set_crc_code(&mut self, crc_code: u32) -> &mut Self {
        self.crc_code = crc_code;
        self
    }

    /// Get the country code.
    pub fn country_code(&self) -> u8 {
        self.country_code
    }

    /// Set the country code.
    pub fn set_country_code(&mut self, country_code: u8) -> &mut Self {
        self.country_code = country_code;
        self
    }

    /// Return the SM64 version with matching CRC and country code, if it exists.
    pub fn version(&self) -> Option<SM64Version> {
        SM64Version::all().iter().copied().find(|version| {
            version.crc_code() == self.crc_code && version.country_code() == self.country_code
        })
    }

    /// Set the CRC and country code to match the given SM64 version.
    pub fn set_version(&mut self, version: SM64Version) -> &mut Self {
        self.crc_code = version.crc_code();
        self.country_code = version.country_code();
        self
    }

    /// Get the author field.
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Set the author field (max 222 bytes).
    ///
    /// # Panics
    ///
    /// Panics if the given string is longer than 222 bytes.
    #[track_caller]
    pub fn set_author(&mut self, author: &str) -> &mut Self {
        match self.try_set_author(author) {
            Ok(this) => this,
            Err(error) => panic!("Error:\n  {}\n", error),
        }
    }

    /// Set the author field (max 222 bytes).
    ///
    /// Returns an error if the given string is longer than 222 bytes.
    pub fn try_set_author(&mut self, author: &str) -> Result<&mut Self, Error> {
        if author.len() > 222 {
            return Err(Error::M64AuthorTooLong);
        }
        self.author = author.to_string();
        Ok(self)
    }

    /// Get the description field.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Set the description field (max 256 bytes).
    ///
    /// # Panics
    ///
    /// Panics if the given string is longer than 256 bytes.
    #[track_caller]
    pub fn set_description(&mut self, description: &str) -> &mut Self {
        match self.try_set_description(description) {
            Ok(this) => this,
            Err(error) => panic!("Error:\n  {}\n", error),
        }
    }

    /// Set the description field (max 256 bytes).
    ///
    /// Returns an error if the given string is longer than 256 bytes.
    pub fn try_set_description(&mut self, description: &str) -> Result<&mut Self, Error> {
        if description.len() > 256 {
            return Err(Error::M64DescriptionTooLong);
        }
        self.description = description.to_string();
        Ok(self)
    }

    /// Get the number of rerecords.
    pub fn rerecords(&self) -> u32 {
        self.rerecords
    }

    /// Set the number of rerecords.
    pub fn set_rerecords(&mut self, rerecords: u32) -> &mut Self {
        self.rerecords = rerecords;
        self
    }

    /// Add a number of rerecords, saturating on overflow.
    pub fn add_rerecords(&mut self, rerecords: u32) -> &mut Self {
        self.rerecords = self.rerecords.saturating_add(rerecords);
        self
    }
}

impl fmt::Display for M64Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "M64Metadata(")?;
        writeln!(f, "  crc_code = {:#010X},", self.crc_code)?;
        writeln!(f, "  country_code = {:?},", self.country_code as char)?;
        writeln!(f, "  author = {:?},", self.author)?;
        writeln!(f, "  description = {:?},", self.description)?;
        writeln!(f, "  rerecords = {},", self.rerecords)?;
        write!(f, ")")?;
        Ok(())
    }
}

/// A set of inputs for a given frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input {
    /// The standard button bit flags.
    pub buttons: u16,
    /// The joystick x coordinate.
    pub stick_x: i8,
    /// The joystick y coordinate.
    pub stick_y: i8,
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Input(buttons = {:#04X}, stick_x = {}, stick_y = {})",
            self.buttons, self.stick_x, self.stick_y
        )
    }
}

/// Load an m64 TAS from a file.
///
/// # Panics
///
/// Panics if:
/// - The file doesn't exist or can't be read
/// - The file is an invalid .m64 file
#[track_caller]
pub fn load_m64(filename: &str) -> (M64Metadata, Vec<Input>) {
    match try_load_m64(filename) {
        Ok(result) => result,
        Err(error) => panic!("Error:\n  {}\n", error),
    }
}

/// Load an m64 TAS from a file.
///
/// Returns an error if:
/// - The file doesn't exist or can't be read
/// - The file is an invalid .m64 file
pub fn try_load_m64(filename: &str) -> Result<(M64Metadata, Vec<Input>), Error> {
    let f = fs::read(filename).map_err(|error| Error::M64ReadError {
        filename: filename.to_string(),
        error: Arc::new(error),
    })?;
    if f.len() < 0x400 {
        return Err(Error::InvalidM64Error {
            filename: filename.to_string(),
        });
    }

    let rerecords = u32::from_le_bytes([f[0x10], f[0x11], f[0x12], f[0x13]]);

    let crc_code = u32::from_le_bytes([f[0xe4], f[0xe5], f[0xe6], f[0xe7]]);
    let country_code = f[0xe8];

    let author = String::from_utf8(f[0x222..0x222 + 222].to_vec())
        .map_err(|_| Error::InvalidM64Error {
            filename: filename.to_string(),
        })?
        .trim_end_matches('\x00')
        .to_string();

    let description = String::from_utf8(f[0x300..0x300 + 256].to_vec())
        .map_err(|_| Error::InvalidM64Error {
            filename: filename.to_string(),
        })?
        .trim_end_matches('\x00')
        .to_string();

    let metadata = M64Metadata {
        crc_code,
        country_code,
        author,
        description,
        rerecords,
    };

    let mut inputs: Vec<Input> = Vec::new();
    for chunk in f[0x400..].chunks_exact(4) {
        inputs.push(Input {
            buttons: u16::from_be_bytes([chunk[0], chunk[1]]),
            stick_x: chunk[2] as i8,
            stick_y: chunk[3] as i8,
        });
    }

    Ok((metadata, inputs))
}

/// Save an m64 TAS to a file.
///
/// # Panics
///
/// Panics if the file can't be written.
#[track_caller]
pub fn save_m64(filename: &str, metadata: &M64Metadata, inputs: &[Input]) {
    if let Err(error) = try_save_m64(filename, metadata, inputs) {
        panic!("Error:\n  {}\n", error);
    }
}

/// Save an m64 TAS to a file.
///
/// Returns an error if the file can't be written.
pub fn try_save_m64(filename: &str, metadata: &M64Metadata, inputs: &[Input]) -> Result<(), Error> {
    save_m64_impl(filename, metadata, inputs).map_err(|error| Error::M64WriteError {
        filename: filename.to_string(),
        error: Arc::new(error),
    })
}

fn save_m64_impl(filename: &str, metadata: &M64Metadata, inputs: &[Input]) -> io::Result<()> {
    if let Some(dir) = Path::new(filename).parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = BufWriter::new(fs::File::create(filename)?);

    f.write_all(&[0x4d, 0x36, 0x34, 0x1a])?; // magic number
    f.write_all(&[0x03, 0x00, 0x00, 0x00])?; // version number
    f.write_all(&[0x00, 0x00, 0x00, 0x00])?; // movie uid
    f.write_all(&[0xff, 0xff, 0xff, 0xff])?; // VI count

    f.write_all(&metadata.rerecords.to_le_bytes())?; // rerecords

    f.write_all(&[0x3c])?; // VIs per second
    f.write_all(&[0x01])?; // num controllers
    f.write_all(&[0x00, 0x00])?; // reserved

    f.write_all(&(inputs.len() as u32).to_le_bytes())?; // num input samples

    f.write_all(&[0x02, 0x00])?; // movie start type = power-on
    f.write_all(&[0x00, 0x00])?; // reserved
    f.write_all(&[0x01, 0x00, 0x00, 0x00])?; // controller flags = controller 1 present
    f.write_all(&[0x00; 160])?; // reserved

    let mut game_name = b"SUPER MARIO 64".to_vec();
    game_name.resize(32, 0x00);
    f.write_all(&game_name)?; // internal name of ROM

    f.write_all(&metadata.crc_code.to_le_bytes())?; // CRC code
    f.write_all(&[metadata.country_code, 0x00])?; // country code

    f.write_all(&[0x00; 56])?; // reserved

    f.write_all(&[0x00; 64])?; // video plugin
    f.write_all(&[0x00; 64])?; // sound plugin
    f.write_all(&[0x00; 64])?; // input plugin
    f.write_all(&[0x00; 64])?; // rsp plugin

    let mut author = metadata.author.as_bytes().to_vec();
    author.resize(222, 0x00);
    f.write_all(&author)?; // author name

    let mut description = metadata.description.as_bytes().to_vec();
    description.resize(256, 0x00);
    f.write_all(&description)?; // description

    for &input in inputs {
        f.write_all(&input.buttons.to_be_bytes())?;
        f.write_all(&[input.stick_x as u8])?;
        f.write_all(&[input.stick_y as u8])?;
    }

    Ok(())
}
