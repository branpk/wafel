use egui::Vec2;
use wafel_api::SM64Version;

use crate::{emu_selector::EmuSelector, Env};

#[derive(Debug)]
pub struct WorkspaceModeSelector {
    emu_selector: EmuSelector,
}

impl WorkspaceModeSelector {
    pub fn new() -> Self {
        Self {
            emu_selector: EmuSelector::new(),
        }
    }

    pub fn show(&mut self, env: &dyn Env, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.set_max_width(ui.available_width().min(500.0));
                    ui.add_space(30.0);

                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        show_mode_section(ui, "Create a new TAS", |ui| {
                            self.show_new_tas_section(ui);
                        });

                        ui.add_space(15.0);
                        show_mode_section(ui, "Open an existing TAS", |ui| {
                            self.show_open_tas_section(ui);
                        });

                        ui.add_space(15.0);
                        show_mode_section(ui, "Connect to a running emulator", |ui| {
                            self.emu_selector.show(env, ui);
                        });

                        ui.add_space(15.0);
                        show_mode_section(ui, "Connect from a script", |ui| {
                            self.show_script_section(ui);
                        });
                    });
                });
            });
    }

    fn show_new_tas_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("SM64 version:");
            for &sm64_version in SM64Version::all() {
                ui.button(sm64_version.to_string());
            }
        });
    }

    fn show_open_tas_section(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.button("Select file...");
        });
    }

    fn show_script_section(&mut self, ui: &mut egui::Ui) {}
}

fn show_mode_section(ui: &mut egui::Ui, heading: &str, show_contents: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        egui::Frame::default()
            .inner_margin(Vec2::new(20.0, 10.0))
            .show(ui, |ui| {
                ui.heading(heading);

                egui::Frame::default()
                    .inner_margin(egui::Margin {
                        left: 10.0,
                        right: 0.0,
                        top: 5.0,
                        bottom: 0.0,
                    })
                    .show(ui, |ui| {
                        show_contents(ui);
                    });
            });
    });
}
