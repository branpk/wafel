use wafel_api::{Emu, VizRenderData};

use crate::{data_explorer::DataExplorer, pane::Pane, Env};

#[derive(Debug)]
pub struct Workspace {
    emu: Emu,
    tree: egui_dock::Tree<Pane>,
}

impl Workspace {
    pub fn with_emu(emu: Emu) -> Self {
        let mut tree = egui_dock::Tree::new(vec![Pane::DataExplorer(DataExplorer::new())]);
        Self { emu, tree }
    }

    pub fn show(&mut self, env: &dyn Env, ui: &mut egui::Ui) -> Vec<VizRenderData> {
        let mut viz_render_data = Vec::new();
        let mut tab_viewer = TabViewer {
            env,
            emu: &mut self.emu,
            viz_render_data: &mut viz_render_data,
        };
        egui_dock::DockArea::new(&mut self.tree)
            .style(egui_dock::Style::from_egui(ui.style()))
            .show_close_buttons(false)
            .tab_context_menus(false)
            .scroll_area_in_tabs(false)
            .show_inside(ui, &mut tab_viewer);
        viz_render_data
    }
}

struct TabViewer<'a> {
    env: &'a dyn Env,
    emu: &'a mut Emu,
    viz_render_data: &'a mut Vec<VizRenderData>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Pane;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Pane) {
        let viz_render_data = tab.show(self.env, self.emu, ui);
        self.viz_render_data.extend(viz_render_data);
    }

    fn title(&mut self, tab: &mut Pane) -> egui::WidgetText {
        tab.title().into()
    }
}
