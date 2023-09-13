use wafel_api::{Emu, VizConfig, VizRenderData};

#[derive(Debug)]
pub struct Workspace {
    emu: Emu,
    prev_viz_render_data: Vec<VizRenderData>,
}

impl Workspace {
    pub fn with_emu(emu: Emu) -> Self {
        Self {
            emu,
            prev_viz_render_data: Vec::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Vec<VizRenderData> {
        let viz_rect = ui.available_rect_before_wrap();

        // TODO: error handling
        let global_timer_addr = self
            .emu
            .address("gGlobalTimer")
            .expect("no gGlobalTimer address");
        self.emu
            .memory
            .load_cache(global_timer_addr)
            .expect("failed to sync to emulator");

        ui.label(format!("pos.y = {}", self.emu.read("gMarioState.pos[1]")));

        let viz_render_data = self
            .emu
            .try_render(&VizConfig {
                screen_top_left: [viz_rect.left() as u32, viz_rect.top() as u32],
                screen_size: [viz_rect.width() as u32, viz_rect.height() as u32],
                ..Default::default()
            })
            .ok()
            .map(|data| vec![data])
            .unwrap_or_else(|| self.prev_viz_render_data.clone());

        self.prev_viz_render_data = viz_render_data.clone();

        viz_render_data
    }
}
