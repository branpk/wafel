use std::fmt;

use itertools::Itertools;
use wafel_api::{Emu, SM64Version};

use crate::{workspace::Workspace, Env, ProcessInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmuAttachInfo {
    pub pid: u32,
    pub base_address: usize,
    pub sm64_version: SM64Version,
}

impl fmt::Display for EmuAttachInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[pid={}, {:#010X}, {}]",
            self.pid, self.base_address, self.sm64_version
        )
    }
}

#[derive(Debug)]
pub struct EmuSelector {
    show_all_processes: bool,
    selected_pid: Option<u32>,
    base_address: String,
    sm64_version: Option<SM64Version>,
    connect_error: Option<String>,
}

impl EmuSelector {
    pub fn new() -> Self {
        Self {
            show_all_processes: false,
            selected_pid: None,
            base_address: String::new(),
            sm64_version: None,
            connect_error: None,
        }
    }

    fn validate(&self) -> Option<EmuAttachInfo> {
        let pid = self.selected_pid?;
        let base_address = validate_base_address(&self.base_address)?;
        let sm64_version = self.sm64_version?;
        Some(EmuAttachInfo {
            pid,
            base_address,
            sm64_version,
        })
    }

    pub fn show(&mut self, env: &dyn Env, ui: &mut egui::Ui) -> Option<Workspace> {
        self.show_process_list(env, ui);

        if self.selected_pid.is_some() {
            ui.vertical(|ui| {
                ui.separator();

                self.show_base_address_selector(ui);
                ui.add_space(5.0);
                self.show_game_version_selector(ui);

                ui.separator();

                self.show_connect_button(ui)
            })
            .inner
        } else {
            None
        }
    }

    fn show_process_list(&mut self, env: &dyn Env, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let label = if self.show_all_processes {
                "Hide non-emulators"
            } else {
                "Show all processes"
            };
            if ui.button(label).clicked() {
                self.show_all_processes = !self.show_all_processes;
            }
        });

        ui.separator();

        let mut processes: Vec<ProcessInfo> = env.processes();

        if let Some(selected_pid) = self.selected_pid {
            if !processes.iter().any(|process| process.pid == selected_pid) {
                self.selected_pid = None;
            }
        }

        processes = processes
            .into_iter()
            .filter(|process| {
                self.show_all_processes
                    || is_likely_emulator(process)
                    || self.selected_pid == Some(process.pid)
            })
            .sorted_by_key(|process| process.name.to_lowercase().clone())
            .collect();

        for process in &processes {
            if ui
                .selectable_label(
                    self.selected_pid == Some(process.pid),
                    format!("{}: {}", process.pid, process.name),
                )
                .clicked()
            {
                if self.selected_pid == Some(process.pid) {
                    self.selected_pid = None;
                } else {
                    self.selected_pid = Some(process.pid);
                    if let Some(base_address) = guess_base_address(process) {
                        self.base_address = format!("{:#010X}", base_address);
                    }
                    self.show_all_processes = false;
                }
            }
        }
        if processes.is_empty() {
            ui.label("No emulators detected");
        }
    }

    fn show_base_address_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Base address:");
            egui::TextEdit::singleline(&mut self.base_address)
                .desired_width(100.0)
                .show(ui);

            if validate_base_address(&self.base_address).is_none() && !self.base_address.is_empty()
            {
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Invalid base address").color(egui::Color32::LIGHT_RED),
                );
            }
        });
    }

    fn show_game_version_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("SM64 version:");
            for &sm64_version in SM64Version::all() {
                if ui
                    .selectable_label(
                        self.sm64_version == Some(sm64_version),
                        sm64_version.to_string(),
                    )
                    .clicked()
                {
                    self.sm64_version = Some(sm64_version);
                }
            }
        });
    }

    fn show_connect_button(&mut self, ui: &mut egui::Ui) -> Option<Workspace> {
        let mut workspace = None;

        let selected_info = self.validate();
        if selected_info.is_none() {
            self.connect_error = None;
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(selected_info.is_some(), egui::Button::new("Connect"))
                .clicked()
            {
                if let Some(info) = selected_info {
                    log::info!("Connecting to emulator: {info}");
                    match Emu::try_attach(info.pid, info.base_address, info.sm64_version) {
                        Ok(emu) => {
                            log::info!("Success!");
                            workspace = Some(Workspace::with_emu(emu));
                        }
                        Err(error) => {
                            log::error!("Error: {error}");
                            self.connect_error = Some(error.to_string());
                        }
                    }
                }
            }

            if let Some(error) = &self.connect_error {
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new(format!("Error: {error}")).color(egui::Color32::LIGHT_RED),
                );
            }
        });

        workspace
    }
}

fn is_likely_emulator(process: &ProcessInfo) -> bool {
    let name = canonicalize_process_name(&process.name);
    name.contains("mupen64") || name.contains("project64") || name.contains("nemu64")
}

fn canonicalize_process_name(name: &str) -> String {
    name.trim_end_matches(".exe")
        .replace("_", "-")
        .to_lowercase()
}

fn validate_base_address(base_address: &str) -> Option<usize> {
    if base_address.starts_with("0x") {
        usize::from_str_radix(&base_address[2..], 16).ok()
    } else {
        usize::from_str_radix(base_address, 10).ok()
    }
}

fn guess_base_address(process: &ProcessInfo) -> Option<usize> {
    let name = canonicalize_process_name(&process.name);
    match name.as_str() {
        "mupen64" => Some(0x0050B110),
        "mupen64-rerecording" => Some(0x008EBA80),
        "mupen64-pucrash" => Some(0x00912300),
        "mupen64-lua" => Some(0x00888F60),
        "mupen64-wiivc" => Some(0x00901920),
        "nemu64" => Some(0x10020000),
        "project64d" => Some(0x20000000),
        _ => None,
    }
}
