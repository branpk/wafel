use imgui::{self as ig, im_str};
use wafel_api::load_m64;

use crate::{
    config::{
        default_unlocked_game_version, is_game_version_unlocked, locked_game_versions,
        unlocked_game_versions,
    },
    project::{Project, TasFileInfo},
};

/// The top level Wafel app state.
#[derive(Debug)]
pub struct App {
    pending_tas: Option<TasFileInfo>,
    project: Option<Project>,
}

impl App {
    /// Create a new app state.
    pub fn open() -> Self {
        Self {
            pending_tas: None,
            project: None,
        }
    }

    /// Render the app.
    pub fn render(&mut self, ui: &ig::Ui<'_>) {
        // If no project is open and at least one libsm64 version has been unlocked, create an
        // empty project.
        if self.project.is_none() && self.pending_tas.is_none() {
            if let Some(game_version) = default_unlocked_game_version() {
                self.project = Some(Project::empty(game_version));
            }
        }

        // If a TAS is waiting to be opened
        if let Some(pending_tas) = &self.pending_tas {
            if is_game_version_unlocked(pending_tas.game_version) {
                // If the required game version is unlocked, load the TAS
                self.project = Some(Project::with_m64(pending_tas));
                self.pending_tas = None;
            } else {
                // Otherwise show game version popup so that user can unlock it
                // TODO: User should be able to select a version to fall back to and/or cancel opening the TAS
                ui.open_popup(im_str!("Game versions##game-versions"));
            }
        }

        // Main app UI
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
                if ig::MenuItem::new(im_str!("New")).build(ui) {
                    self.new_project();
                }
                if ig::MenuItem::new(im_str!("Open")).build(ui) {
                    self.open_m64();
                }
                if ig::MenuItem::new(im_str!("Save"))
                    .enabled(self.project.is_some())
                    .build(ui)
                {
                    self.save_m64();
                }
                if ig::MenuItem::new(im_str!("Save as"))
                    .enabled(self.project.is_some())
                    .build(ui)
                {
                    self.save_m64_as();
                }
                ui.menu(im_str!("Game version"), true, || {
                    if let Some(name) = self.game_version_menu(ui) {
                        open_popup = Some(name);
                    }
                });
            });

            ui.menu(im_str!("Settings"), true, || {
                if ig::MenuItem::new(im_str!("Controller")).build(ui) {
                    open_popup = Some("Controller##settings-controller");
                }
                if ig::MenuItem::new(im_str!("Key bindings")).build(ui) {
                    open_popup = Some("Key bindings##settings-key-bindings");
                }
            });
        });

        if let Some(name) = open_popup {
            ui.open_popup(&im_str!("{}", name));
        }
    }

    fn new_project(&mut self) {
        // Use the current project's game version if possible
        let game_version = self
            .project
            .take()
            .map(|p| p.game_version())
            .or_else(default_unlocked_game_version);
        if let Some(game_version) = game_version {
            self.project = Some(Project::empty(game_version));
        }
    }

    fn open_m64(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Mupen64 TAS", &["m64"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            // TODO: Error handling for invalid M64 file and unknown game version
            let filename = path.as_os_str().to_str().expect("invalid filename");
            let (metadata, inputs) = load_m64(filename);
            let game_version = metadata.version().expect("unknown game version");

            self.pending_tas = Some(TasFileInfo {
                game_version,
                filename: filename.to_string(),
                metadata,
                inputs,
            });
        }
    }

    fn save_m64(&mut self) {
        let project = self.project.as_ref().expect("no open project");
        if project.filename().is_some() {
            project.save_m64();
        } else {
            self.save_m64_as();
        }
    }

    fn save_m64_as(&mut self) {
        let project = self.project.as_mut().expect("no open project");
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Mupen64 TAS", &["m64"])
            .add_filter("All Files", &["*"])
            .save_file()
        {
            project.set_filename(path.as_os_str().to_str().expect("invalid filename"));
            project.save_m64();
        }
    }

    fn game_version_menu(&mut self, ui: &ig::Ui<'_>) -> Option<&'static str> {
        let mut open_popup = None;

        // Show unlocked game versions in the menu
        let loaded_version = self.project.as_ref().map(|p| p.game_version());
        for version in unlocked_game_versions() {
            if ig::MenuItem::new(&im_str!("{}", version))
                .selected(loaded_version == Some(version))
                .build(ui)
            {
                if let Some(project) = &mut self.project {
                    project.change_game_version(version);
                }
            }
        }
        // If other locked versions exist, show an option to open the game version
        // popup
        if !locked_game_versions().is_empty() && ig::MenuItem::new(im_str!("Other")).build(ui) {
            open_popup = Some("Game versions##game-versions");
        }

        open_popup
    }
}
