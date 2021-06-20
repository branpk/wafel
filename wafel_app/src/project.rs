use imgui::{self as ig, im_str};
use wafel_api::{Input, M64Metadata, SM64Version, Timeline};

use crate::config::libsm64_path;

#[derive(Debug)]
pub struct TasFileInfo {
    pub game_version: SM64Version,
    pub filename: String,
    pub metadata: M64Metadata,
    pub inputs: Vec<Input>,
}

#[derive(Debug)]
pub struct Project {
    filename: Option<String>,
    game_version: SM64Version,
    timeline: Timeline,
}

impl Project {
    pub fn empty(game_version: SM64Version) -> Self {
        let timeline = unsafe { Timeline::new(&libsm64_path(game_version)) };
        Self {
            filename: None,
            game_version,
            timeline,
        }
    }

    pub fn with_m64(tas_data: &TasFileInfo) -> Self {
        todo!()
    }

    pub fn game_version(&self) -> SM64Version {
        self.game_version
    }

    pub fn change_game_version(&mut self, game_version: SM64Version) {
        self.game_version = game_version;
        self.timeline = unsafe { Timeline::new(&libsm64_path(game_version)) };
    }

    pub fn filename(&self) -> &Option<String> {
        &self.filename
    }

    pub fn set_filename(&mut self, filename: &str) {
        self.filename = Some(filename.to_string());
    }

    pub fn save_m64(&self) {
        let filename = self.filename.as_ref().expect("project filename not set");
        todo!()
    }

    pub fn render(&mut self, ui: &ig::Ui<'_>) {
        ui.text(im_str!("project {}", self.game_version));
    }
}
