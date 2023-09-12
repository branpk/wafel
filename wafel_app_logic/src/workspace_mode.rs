use egui::Vec2;

#[derive(Debug)]
pub struct WorkspaceModeSelector {}

impl WorkspaceModeSelector {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.set_max_width(ui.available_width().min(500.0));
                    ui.add_space(30.0);

                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        ui.group(|ui| {
                            egui::Frame::default()
                                .inner_margin(Vec2::new(20.0, 10.0))
                                .show(ui, |ui| {
                                    self.show_new_tas_section(ui);
                                });
                        });

                        ui.add_space(15.0);
                        ui.group(|ui| {
                            egui::Frame::default()
                                .inner_margin(Vec2::new(20.0, 10.0))
                                .show(ui, |ui| {
                                    self.show_open_tas_section(ui);
                                });
                        });

                        ui.add_space(15.0);
                        ui.group(|ui| {
                            egui::Frame::default()
                                .inner_margin(Vec2::new(20.0, 10.0))
                                .show(ui, |ui| {
                                    self.show_emu_section(ui);
                                });
                        });

                        ui.add_space(15.0);
                        ui.group(|ui| {
                            egui::Frame::default()
                                .inner_margin(Vec2::new(20.0, 10.0))
                                .show(ui, |ui| {
                                    self.show_script_section(ui);
                                });
                        });
                    });
                });
            });
    }

    fn show_new_tas_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Create a new TAS");
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("SM64 version:");
            ui.button("US");
            ui.button("JP");
            ui.button("EU");
            ui.button("SH");
        });
    }

    fn show_open_tas_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Open an existing TAS");
        ui.add_space(5.0);
        ui.vertical(|ui| {
            ui.button("Select file...");
        });
    }

    fn show_emu_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Attach to a running emulator");
        ui.add_space(5.0);
    }

    fn show_script_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Connect from a script");
        ui.add_space(5.0);
    }
}
