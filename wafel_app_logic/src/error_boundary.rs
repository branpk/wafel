use std::panic::{self, AssertUnwindSafe};

use crate::Env;

#[derive(Debug)]
pub struct ErrorBoundary {
    error: Option<String>,
    copied: bool,
}

impl ErrorBoundary {
    pub fn new() -> Self {
        Self {
            error: None,
            copied: false,
        }
    }

    pub fn catch_panic<R>(&mut self, env: &dyn Env, func: impl FnOnce() -> R) -> Option<R> {
        if self.error.is_some() {
            return None;
        }

        // Since func is usually not going to be unwind safe, we instead give the
        // user an option to choose whether or not to continue after the panic.
        match panic::catch_unwind(AssertUnwindSafe(func)) {
            Ok(result) => Some(result),
            Err(cause) => {
                let details = env.take_recent_panic_details().unwrap_or_else(|| {
                    let cause_str = if let Some(s) = cause.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = cause.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic cause".to_string()
                    };
                    format!("Panic cause: {cause_str}\nBacktrace not captured.")
                });
                self.error = Some(details);
                self.copied = false;
                None
            }
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn show_error(&mut self, env: &dyn Env, ui: &mut egui::Ui) {
        let mut try_to_continue = false;

        if let Some(error) = &self.error {
            egui::Frame::default()
                .inner_margin(egui::Vec2::new(100.0, 30.0))
                .show(ui, |ui| {
                    ui.heading("Wafel encountered an error.");

                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("Please report this issue ");
                        ui.hyperlink_to("here", "https://github.com/branpk/wafel/issues");
                        ui.label(".");
                    });

                    ui.add_space(10.0);
                    ui.label("The error can be copied below, and your log file is located at:");
                    let mut path = env
                        .log_file_path()
                        .as_os_str()
                        .to_string_lossy()
                        .to_string();
                    ui.add_sized(
                        [ui.available_width(), 0.0],
                        egui::TextEdit::singleline(&mut path),
                    );

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label("You can try to ignore the error by pressing here:");
                        if ui.button("Try to continue").clicked() {
                            try_to_continue = true;
                        }
                    });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Copy error").clicked() {
                            ui.ctx().output_mut(|output| {
                                output.copied_text = error.clone();
                                self.copied = true;
                            })
                        }
                        if self.copied {
                            ui.label("Copied.");
                        }
                    });

                    ui.add_space(5.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut text = error.clone();
                        ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut text));
                    });
                });
        }

        if try_to_continue {
            self.error = None;
        }
    }
}
