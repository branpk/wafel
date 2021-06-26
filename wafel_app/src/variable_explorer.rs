use imgui::{self as ig, im_str};
use wafel_core::{ObjectBehavior, ObjectSlot, Pipeline, SurfaceSlot, Variable};

use crate::{
    object_slots::render_object_slots,
    variable_value::{VariableFormatter, VariableValueUi},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TabId {
    Input,
    Mario,
    Misc,
    Subframe,
    Objects,
    Object(ObjectSlot),
    Surface(SurfaceSlot),
}

impl TabId {
    fn id(&self) -> String {
        format!("tab-{:?}", self)
    }
}

#[derive(Debug)]
pub(crate) struct VariableExplorer {
    open_tabs: Vec<TabId>,
    current_tab: TabId,
}

impl Default for VariableExplorer {
    fn default() -> Self {
        Self {
            open_tabs: [
                TabId::Input,
                TabId::Mario,
                TabId::Misc,
                TabId::Subframe,
                TabId::Objects,
            ]
            .to_vec(),
            current_tab: TabId::Input,
        }
    }
}

impl VariableExplorer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn open_tab(&mut self, tab: TabId) {
        if !self.open_tabs.contains(&tab) {
            self.open_tabs.push(tab);
        }
        self.current_tab = tab;
    }

    fn open_object_tab(&mut self, object: ObjectSlot) {
        self.open_tab(TabId::Object(object));
    }

    fn open_surface_tab(&mut self, surface: SurfaceSlot) {
        self.open_tab(TabId::Surface(surface));
    }

    fn close_tab(&mut self, tab: TabId) {
        self.open_tabs.retain(|t| *t != tab);
    }

    fn tab_label(&self, tab: TabId) -> String {
        // TODO: Tab label
        tab.id()
    }

    fn render_objects_tab(&mut self, ui: &ig::Ui<'_>, pipeline: &Pipeline, frame: u32) {
        let behaviors: Vec<Option<ObjectBehavior>> = (0..240)
            .map(|slot| pipeline.object_behavior(frame, ObjectSlot(slot)))
            .collect();
        let selected_slot = render_object_slots(ui, "object-slots", &behaviors, |behavior| {
            pipeline.object_behavior_name(behavior)
        });
        if let Some(selected_slot) = selected_slot {
            self.open_object_tab(selected_slot);
        }
    }

    fn variables_for_tab(&self, pipeline: &Pipeline, frame: u32, tab: TabId) -> Vec<Variable> {
        match tab {
            TabId::Input => pipeline.variable_group("Input"),
            TabId::Mario => pipeline.variable_group("Mario"),
            TabId::Misc => pipeline.variable_group("Misc"),
            TabId::Subframe => unimplemented!(),
            TabId::Objects => unimplemented!(),
            TabId::Object(object) => match pipeline.object_behavior(frame, object) {
                Some(behavior) => pipeline
                    .variable_group("Object")
                    .into_iter()
                    .filter(|var| pipeline.label(var).is_some())
                    .map(|var| {
                        var.with_object(object)
                            .with_object_behavior(behavior.clone())
                    })
                    .collect(),
                None => todo!(),
            },
            TabId::Surface(surface) => {
                let num_surfaces = pipeline
                    .timeline()
                    .read(frame, "gSurfacesAllocated")
                    .as_usize();
                if surface.0 >= num_surfaces {
                    Vec::new()
                } else {
                    pipeline
                        .variable_group("Surface")
                        .into_iter()
                        .filter(|var| pipeline.label(var).is_some())
                        .map(|var| var.with_surface(surface))
                        .collect()
                }
            }
        }
    }

    fn render_variable(
        &self,
        ui: &ig::Ui<'_>,
        pipeline: &mut Pipeline,
        tab: TabId,
        variable: &Variable,
        label_width: f32,
        value_width: f32,
    ) {
        let value = pipeline.read(variable);

        // TODO: Persist state
        let result = VariableValueUi::default().render_labeled(
            ui,
            &format!("var-{}-{}", tab.id(), variable),
            &pipeline.label(variable).expect("missing variable label"),
            variable,
            &value,
            VariableFormatter::for_value(pipeline, variable, &value),
            false,
            label_width,
            value_width,
        );

        if let Some(new_value) = result.changed_value {
            pipeline.write(variable, new_value);
        }
        if result.clear_edit {
            pipeline.reset(variable);
        }
    }
}
