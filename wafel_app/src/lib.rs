//! Logic and UI for the Wafel application.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

mod config;

use imgui::{self as ig, im_str};
use wafel_api::{SM64Version, Timeline};

use crate::config::{
    default_unlocked_game_version, libsm64_path, locked_game_versions, unlocked_game_versions,
};

#[derive(Debug)]
pub struct App {
    project: Option<Project>,
}

impl App {
    pub fn open() -> Self {
        Self { project: None }
    }

    pub fn render(&mut self, ui: &ig::Ui<'_>) {
        if self.project.is_none() {
            if let Some(game_version) = default_unlocked_game_version() {
                let dll_path = libsm64_path(game_version);
                let timeline = unsafe { Timeline::try_new(&dll_path).unwrap() }; // TODO: Error handling
                self.project = Some(Project {
                    game_version,
                    timeline,
                });
            }
        }

        ig::Window::new(im_str!("Main"))
            .position([0.0, 0.0], ig::Condition::Always)
            .size(
                [ui.io().display_size[0], ui.io().display_size[1]],
                ig::Condition::Always,
            )
            .resizable(false)
            .title_bar(false)
            .menu_bar(true)
            .bring_to_front_on_focus(false)
            .build(&ui, || {
                self.render_menu_bar(ui);
                if let Some(project) = &mut self.project {
                    project.render(ui);
                }
            });
    }

    fn render_menu_bar(&mut self, ui: &ig::Ui<'_>) {
        let mut open_popup: Option<&str> = None;

        ui.main_menu_bar(|| {
            ui.menu(im_str!("File"), true, || {
                if ig::MenuItem::new(im_str!("New")).build(ui) {}
                if ig::MenuItem::new(im_str!("Open")).build(ui) {}
                if ig::MenuItem::new(im_str!("Save")).build(ui) {}
                if ig::MenuItem::new(im_str!("Save as")).build(ui) {}
                ui.menu(im_str!("Game version"), true, || {
                    let loaded_version = self.project.as_ref().map(|p| p.game_version);
                    for version in unlocked_game_versions() {
                        if ig::MenuItem::new(&im_str!("{}", version))
                            .selected(loaded_version == Some(version))
                            .build(ui)
                        {
                            self.change_version(version);
                        }
                    }
                    if !locked_game_versions().is_empty() {
                        // Only show Other option if locked game versions exist
                        if ig::MenuItem::new(im_str!("Other")).build(ui) {
                            open_popup = Some("Game versions##game-versions");
                        }
                    }
                });
            });
            ui.menu(im_str!("Settings"), true, || {
                if ig::MenuItem::new(im_str!("Controller")).build(ui) {}
                if ig::MenuItem::new(im_str!("Key bindings")).build(ui) {}
            });
        });

        if let Some(name) = open_popup {
            ui.open_popup(&im_str!("{}", name));
        }
    }

    fn change_version(&mut self, version: SM64Version) {}
}

#[derive(Debug)]
struct Project {
    game_version: SM64Version,
    timeline: Timeline,
}

impl Project {
    fn render(&mut self, ui: &ig::Ui<'_>) {
        ui.text(im_str!("project {}", self.game_version));
    }
}
