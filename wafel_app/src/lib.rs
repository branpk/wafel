//! Logic and UI for the Wafel application.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

#[derive(Debug)]
pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&mut self, ui: &imgui::Ui<'_>) {
        imgui::Window::new(imgui::im_str!("test window"))
            .size([300.0, 100.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text("Hello world");
                ui.input_text_multiline(
                    imgui::im_str!("text area"),
                    &mut imgui::ImString::with_capacity(15),
                    [200.0, 100.0],
                )
                .build();
            });
    }
}
