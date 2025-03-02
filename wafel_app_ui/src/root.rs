use wafel_api::VizScene;

use crate::{workspace_root::WorkspaceRoot, Env};

#[derive(Debug)]
pub struct Root {
    is_workspace_panel_expanded: bool,
    workspaces: Vec<WorkspaceRoot>,
    selected_workspace_index: Option<usize>,
    next_workspace_num: u32,
}

impl Root {
    pub fn new() -> Self {
        let mut this = Self {
            is_workspace_panel_expanded: false,
            workspaces: Vec::new(),
            selected_workspace_index: None,
            next_workspace_num: 1,
        };
        let index = this.new_workspace();
        this.selected_workspace_index = Some(index);
        this
    }

    fn new_workspace(&mut self) -> usize {
        let name = format!("Workspace {}", self.next_workspace_num);
        self.next_workspace_num += 1;
        self.workspaces.push(WorkspaceRoot::new(&name));
        self.workspaces.len() - 1
    }

    pub fn show(&mut self, env: &dyn Env, ctx: &egui::Context) -> Vec<VizScene> {
        let is_workspace_panel_expanded =
            self.is_workspace_panel_expanded || self.selected_workspace_index.is_none();

        egui::SidePanel::left("wafel_left_panel")
            .default_width(150.0)
            .resizable(false)
            .show_animated(ctx, self.is_workspace_panel_expanded, |ui| {
                self.show_workspace_pane_contents(ui);
            });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let label = match (self.selected_workspace_index, is_workspace_panel_expanded) {
                    (Some(index), false) => self.workspaces[index].name(),
                    (None, false) => "Show workspaces",
                    (_, true) => "Hide workspaces",
                };
                if ui.button(label).clicked() {
                    self.is_workspace_panel_expanded = !self.is_workspace_panel_expanded;
                }

                ui.separator();
                if let Some(index) = self.selected_workspace_index {
                    self.workspaces[index].show(env, ui)
                } else {
                    Vec::new()
                }
            })
            .inner
    }

    fn show_workspace_pane_contents(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.add_space(5.0);
                ui.vertical_centered_justified(|ui| {
                    ui.label("Workspaces");
                });

                ui.separator();
                self.show_workspace_list(ui);

                ui.separator();
                ui.vertical_centered_justified(|ui| {
                    if ui.button("New workspace").clicked() {
                        let index = self.new_workspace();
                        self.selected_workspace_index = Some(index);
                    }
                });
            });
        });
    }

    fn show_workspace_list(&mut self, ui: &mut egui::Ui) {
        for (index, workspace) in self.workspaces.iter_mut().enumerate() {
            let response =
                workspace.show_select_button(ui, self.selected_workspace_index == Some(index));
            if response.selected {
                self.selected_workspace_index = Some(index);
            }
        }
    }
}
