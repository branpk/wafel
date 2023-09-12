#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WorkspaceButtonResponse {
    pub selected: bool,
}

#[derive(Debug)]
pub struct Workspace {
    name: String,
    renaming: bool,
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            renaming: false,
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
}
