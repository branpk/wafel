use crate::{workspace::Workspace, workspace_mode::WorkspaceModeSelector, Env};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WorkspaceButtonResponse {
    pub selected: bool,
}

#[derive(Debug)]
pub struct WorkspaceRoot {
    name: String,
    renaming: bool,
    mode_selector: WorkspaceModeSelector,
    workspace: Option<Workspace>,
}

impl WorkspaceRoot {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            renaming: false,
            mode_selector: WorkspaceModeSelector::new(),
            workspace: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn show_select_button(
        &mut self,
        ui: &mut egui::Ui,
        selected: bool,
    ) -> WorkspaceButtonResponse {
        let mut response = WorkspaceButtonResponse::default();

        if self.renaming {
            let edit_response = ui.text_edit_singleline(&mut self.name);
            if edit_response.lost_focus() {
                self.renaming = false;
            } else {
                ui.memory_mut(|memory| memory.request_focus(edit_response.id));
            }
        } else {
            let label_response = ui.selectable_label(selected, &self.name);
            if label_response.clicked() {
                response.selected = true;
            }

            label_response.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    self.renaming = true;
                    ui.close_menu();
                }
            });
        }

        response
    }

    pub fn show(&mut self, env: &dyn Env, ui: &mut egui::Ui) {
        match &mut self.workspace {
            Some(workspace) => workspace.show(ui),
            None => {
                self.workspace = self.mode_selector.show(env, ui);
            }
        }
    }
}
