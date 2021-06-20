use imgui::{self as ig, im_str};
use wafel_api::{save_m64, Input, M64Metadata, SM64Version, Timeline};
use wafel_core::{Pipeline, Variable};

use crate::{config::libsm64_path, frame_slider::render_frame_slider};

#[derive(Debug)]
pub(crate) struct TasFileInfo {
    pub(crate) game_version: SM64Version,
    pub(crate) filename: String,
    pub(crate) metadata: M64Metadata,
    pub(crate) inputs: Vec<Input>,
}

#[derive(Debug)]
pub(crate) struct Project {
    game_version: SM64Version,
    filename: Option<String>,
    metadata: M64Metadata,
    pipeline: Pipeline,
    max_frame: u32,
    selected_frame: u32,
}

impl Project {
    pub(crate) fn empty(game_version: SM64Version) -> Self {
        let pipeline = unsafe { Pipeline::new(&libsm64_path(game_version)) };
        Self {
            game_version,
            filename: None,
            metadata: M64Metadata::with_version(game_version)
                .set_author("Unknown author(s)")
                .set_description("Made using Wafel")
                .clone(),
            pipeline,
            max_frame: 0,
            selected_frame: 0,
        }
    }

    pub(crate) fn with_m64(tas_data: &TasFileInfo) -> Self {
        let mut project = Self::empty(tas_data.game_version);
        project.filename = Some(tas_data.filename.clone());
        project.metadata = tas_data.metadata.clone();
        project.max_frame = tas_data.inputs.len().saturating_sub(1) as u32;

        for (frame, &input) in tas_data.inputs.iter().enumerate() {
            let frame = frame as u32;
            project.pipeline.write(
                &Variable::new("input-buttons").with_frame(frame),
                input.buttons.into(),
            );
            project.pipeline.write(
                &Variable::new("input-stick-x").with_frame(frame),
                input.stick_x.into(),
            );
            project.pipeline.write(
                &Variable::new("input-stick-y").with_frame(frame),
                input.stick_y.into(),
            );
        }

        project
    }

    pub(crate) fn change_game_version(mut self, game_version: SM64Version) -> Self {
        let edits = self.pipeline.into_edits();
        self.pipeline = unsafe { Pipeline::new(&libsm64_path(game_version)) };
        self.pipeline.set_edits(edits);

        self.game_version = game_version;
        self.metadata.set_version(game_version);

        self
    }

    pub(crate) fn game_version(&self) -> SM64Version {
        self.game_version
    }

    pub(crate) fn filename(&self) -> &Option<String> {
        &self.filename
    }

    pub(crate) fn set_filename(&mut self, filename: &str) {
        self.filename = Some(filename.to_string());
    }

    pub(crate) fn save_m64(&self) {
        let filename = self.filename.as_ref().expect("project filename not set");
        let mut inputs = Vec::new();
        for frame in 0..=self.max_frame {
            let buttons = self
                .pipeline
                .read(&Variable::new("input-buttons").with_frame(frame))
                .as_int() as u16;
            let stick_x = self
                .pipeline
                .read(&Variable::new("input-stick-x").with_frame(frame))
                .as_int() as u8;
            let stick_y = self
                .pipeline
                .read(&Variable::new("input-stick-y").with_frame(frame))
                .as_int() as u8;
            inputs.push(Input {
                buttons,
                stick_x,
                stick_y,
            });
        }
        save_m64(filename, &self.metadata, &inputs);
    }

    pub(crate) fn render(&mut self, ui: &ig::Ui<'_>) {
        ui.text(im_str!("project {}", self.game_version));
        if let Some(frame) = render_frame_slider(
            ui,
            self.selected_frame,
            self.max_frame,
            &self.pipeline.timeline().dbg_cached_frames(),
        ) {
            self.selected_frame = frame;
        }
    }
}
