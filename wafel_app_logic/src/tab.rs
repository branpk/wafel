use wafel_api::VizRenderData;

use crate::Env;

#[derive(Debug)]
pub enum Tab {
    Test1,
    Test2,
}

impl Tab {
    pub fn title(&self) -> String {
        match self {
            Tab::Test1 => "Test 1 title".to_string(),
            Tab::Test2 => "Test 2 title".to_string(),
        }
    }

    pub fn show(&mut self, env: &dyn Env, ui: &mut egui::Ui) -> Vec<VizRenderData> {
        match self {
            Self::Test1 => {
                ui.label("Test1");
            }
            Self::Test2 => {
                ui.label("Test2");
            }
        }
        Vec::new()
    }
}
