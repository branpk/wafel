use imgui::{self as ig, im_str};
use wafel_api::{Input, M64Metadata, SM64Version, Timeline};

use crate::config::libsm64_path;

#[derive(Debug)]
pub(crate) struct TasFileInfo {
    pub(crate) game_version: SM64Version,
    pub(crate) filename: String,
    pub(crate) metadata: M64Metadata,
    pub(crate) inputs: Vec<Input>,
}

#[derive(Debug)]
pub(crate) struct Project {
    filename: Option<String>,
    game_version: SM64Version,
    timeline: Timeline,
}

impl Project {
    pub(crate) fn empty(game_version: SM64Version) -> Self {
        let timeline = unsafe { Timeline::new(&libsm64_path(game_version)) };
        Self {
            filename: None,
            game_version,
            timeline,
        }
    }

    pub(crate) fn with_m64(tas_data: &TasFileInfo) -> Self {
        todo!()
    }

    pub(crate) fn game_version(&self) -> SM64Version {
        self.game_version
    }

    pub(crate) fn change_game_version(&mut self, game_version: SM64Version) {
        self.game_version = game_version;
        self.timeline = unsafe { Timeline::new(&libsm64_path(game_version)) };
    }

    pub(crate) fn filename(&self) -> &Option<String> {
        &self.filename
    }

    pub(crate) fn set_filename(&mut self, filename: &str) {
        self.filename = Some(filename.to_string());
    }

    pub(crate) fn save_m64(&self) {
        let filename = self.filename.as_ref().expect("project filename not set");
        todo!()
    }

    pub(crate) fn render(&mut self, ui: &ig::Ui<'_>) {
        ui.text(im_str!("project {}", self.game_version));
    }
}
