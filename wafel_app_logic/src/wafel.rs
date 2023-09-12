use crate::{root::Root, Env};

/// State of the Wafel application.
#[derive(Debug)]
pub struct Wafel {
    root: Root,
}

impl Default for Wafel {
    fn default() -> Self {
        Self { root: Root::new() }
    }
}

impl Wafel {
    /// Render the Wafel UI and respond to user input events.
    pub fn show(&mut self, env: &dyn Env, ctx: &egui::Context) {
        self.root.show(env, ctx);
    }
}
