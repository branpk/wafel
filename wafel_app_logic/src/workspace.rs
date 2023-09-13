use wafel_api::{Emu, Error};

use crate::emu_selector::EmuAttachInfo;

#[derive(Debug)]
pub struct Workspace {
    emu: Emu,
}

impl Workspace {
    pub fn with_emu(emu: Emu) -> Self {
        Self { emu }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Hello, world!");
            ui.label("This is a test.");
        });
    }
}
