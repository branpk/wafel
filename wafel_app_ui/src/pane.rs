use wafel_api::{Emu, VizScene};

use crate::{data_explorer::DataExplorer, Env};

#[derive(Debug)]
pub enum Pane {
    DataExplorer(DataExplorer),
}

impl Pane {
    pub fn title(&self) -> String {
        match self {
            Pane::DataExplorer(_) => "Data Explorer".to_owned(),
        }
    }

    pub fn show(&mut self, env: &dyn Env, emu: &mut Emu, ui: &mut egui::Ui) -> Vec<VizScene> {
        match self {
            Self::DataExplorer(data_explorer) => data_explorer.show(emu, ui),
        }
        Vec::new()
    }
}
