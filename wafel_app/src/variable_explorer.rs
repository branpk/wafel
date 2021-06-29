use std::{f32::consts::PI, num::Wrapping};

use imgui::{self as ig, im_str};
use wafel_api::Value;
use wafel_core::{
    stick_adjusted_to_intended, stick_intended_to_raw_heuristic, stick_raw_to_adjusted, Angle,
    IntendedStick, ObjectBehavior, ObjectSlot, Pipeline, SurfaceSlot, Variable,
};

use crate::{
    joystick_control::{JoystickControlShape, JoystickControlUi},
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

    fn render_stick_control(
        &self,
        ui: &ig::Ui<'_>,
        id: &str,
        tab: TabId,
        pipeline: &mut Pipeline,
        frame: u32,
    ) {
        let stick_x_var = Variable::new("input-stick-x").with_frame(frame);
        let stick_y_var = Variable::new("input-stick-y").with_frame(frame);

        self.render_variable(ui, pipeline, tab, &stick_x_var, 60.0, 50.0);
        self.render_variable(ui, pipeline, tab, &stick_y_var, 60.0, 50.0);

        let stick_x = pipeline.read(&stick_x_var).as_int() as i8;
        let stick_y = pipeline.read(&stick_y_var).as_int() as i8;

        let n = [
            2.0 * ((stick_x as f32 + 128.0) / 255.0) - 1.0,
            2.0 * ((stick_y as f32 + 128.0) / 255.0) - 1.0,
        ];
        // TODO: Persist joystick state
        let new_n = JoystickControlUi::new().render(ui, id, n, JoystickControlShape::Square);

        if let Some(new_n) = new_n {
            let new_stick_x = (0.5 * (new_n[0] + 1.0) * 255.0 - 128.0) as i8;
            let new_stick_y = (0.5 * (new_n[1] + 1.0) * 255.0 - 128.0) as i8;

            pipeline.write(&stick_x_var, new_stick_x.into());
            pipeline.write(&stick_y_var, new_stick_y.into());
        }
    }

    fn render_intended_stick_control(
        &self,
        ui: &ig::Ui<'_>,
        id: &str,
        tab: TabId,
        pipeline: &mut Pipeline,
        frame: u32,
    ) {
        let up_options = ["3d view", "mario yaw", "stick y", "world x"];
        let mut up_option = 0; // TODO: persist

        ui.text("up =");
        ui.same_line(0.0);
        ui.set_next_item_width(100.0);
        ig::ComboBox::new(im_str!("##up-option")).build_simple(
            ui,
            &mut up_option,
            &up_options,
            &|option| im_str!("{}", option).into(),
        );
        ui.dummy([1.0, 10.0]);

        let stick_x_var = Variable::new("input-stick-x").with_frame(frame);
        let stick_y_var = Variable::new("input-stick-y").with_frame(frame);

        let face_yaw = Wrapping(
            pipeline
                .read(&Variable::new("mario-face-yaw").with_frame(frame))
                .as_int() as i16,
        );
        let camera_yaw = Wrapping(
            pipeline
                .read(&Variable::new("camera-yaw").with_frame(frame))
                .as_int() as i16,
        );
        let squish_timer = pipeline
            .timeline()
            .read(frame, "gMarioState->squishTimer")
            .as_int();
        let mut active_face_yaw = face_yaw;

        let events = pipeline.timeline().frame_log(frame + 1);

        let mut active_face_yaw_action = None;
        for event in &events {
            if event["type"].as_str() == "FLT_EXECUTE_ACTION" {
                let action_name = "TODO"; // TODO: self.model.action_names[event['action']]
                active_face_yaw = Wrapping(event["faceAngle"].as_i16_3()[1]);
                active_face_yaw_action = Some(action_name);
                if action_name == "idle" {
                    break;
                }
            }
        }

        let up_angle = match up_options[up_option] {
            "mario yaw" => active_face_yaw,
            "stick y" => camera_yaw + Wrapping(-0x8000),
            "world x" => Wrapping(0x4000),
            "3d view" => todo!(), // TODO: self.model.rotational_camera_yaw
            _ => unimplemented!(),
        };
        let input_up_yaw = up_angle;

        let raw_stick_x = pipeline.read(&stick_x_var).as_int() as i8;
        let raw_stick_y = pipeline.read(&stick_y_var).as_int() as i8;

        let adjusted = stick_raw_to_adjusted(raw_stick_x.into(), raw_stick_y.into());
        let intended =
            stick_adjusted_to_intended(adjusted, face_yaw, camera_yaw, squish_timer != 0);

        fn render_value(
            ui: &ig::Ui<'_>,
            label: &str,
            value: &Value,
            formatter: VariableFormatter,
        ) -> Option<Value> {
            let label_width = 60.0;
            let value_size = [
                if label == "dyaw" { 60.0 } else { 80.0 },
                ui.text_line_height() + 2.0 * ui.clone_style().frame_padding[1],
            ];
            ui.set_next_item_width(label_width);
            ig::Selectable::new(&im_str!("{}", label))
                .size([label_width, 0.0])
                .build(ui);
            ui.same_line(0.0);
            // TODO: Persist
            let result = VariableValueUi::new().render_value(
                ui,
                &format!("value-{}", label),
                value,
                formatter,
                value_size,
                false,
            );
            result.changed_value
        }

        let target_mag = render_value(
            ui,
            "int mag",
            &intended.mag.into(),
            VariableFormatter::Float,
        )
        .map(|v| v.as_f32());
        let mut target_yaw = render_value(
            ui,
            "int yaw",
            &intended.yaw.0.into(),
            VariableFormatter::DecimalInt,
        )
        .map(|v| Wrapping(v.as_int() as i16));
        let dyaw = intended.yaw - active_face_yaw;
        let mut target_dyaw =
            render_value(ui, "dyaw", &dyaw.0.into(), VariableFormatter::DecimalInt)
                .map(|v| Wrapping(v.as_int() as i16));

        ui.same_line(0.0);
        if ui.button(im_str!("?"), [0.0, 0.0]) {
            ui.open_popup(im_str!("active-yaw-expl"));
        }
        ui.popup(im_str!("active-yaw-expl"), || {
            ui.text(&im_str!(
                "{} - {} = {}",
                intended.yaw,
                active_face_yaw,
                dyaw
            ));
            ui.text(&im_str!("intended yaw = {}", intended.yaw));
            if active_face_yaw == face_yaw {
                ui.text(&im_str!("face yaw = {}", face_yaw));
            } else {
                ui.text(&im_str!(
                    "face yaw = {} at start of {} action",
                    active_face_yaw,
                    active_face_yaw_action.expect("missing action"),
                ));
                ui.text(&im_str!("(face yaw = {} at start of frame)", face_yaw));
            }
        });

        if !(0..16).contains(&dyaw.0) && ui.button(im_str!("dyaw = 0"), [0.0, 0.0]) {
            target_dyaw = Some(Wrapping(0));
        }

        if target_yaw.is_some() || target_dyaw.is_some() || target_mag.is_some() {
            let relative_to = if target_yaw.is_some() {
                Wrapping(0)
            } else {
                active_face_yaw
            };

            if let Some(target_dyaw) = target_dyaw {
                target_yaw = Some(active_face_yaw + target_dyaw);
            }
            let target_yaw = target_yaw.unwrap_or(intended.yaw);
            let target_mag = target_mag.unwrap_or(intended.mag).clamp(0.0, 32.0);

            let (new_stick_x, new_stick_y) = stick_intended_to_raw_heuristic(
                IntendedStick {
                    yaw: target_yaw,
                    mag: target_mag,
                },
                face_yaw,
                camera_yaw,
                squish_timer != 0,
                relative_to,
            );

            pipeline.write(&stick_x_var, new_stick_x.into());
            pipeline.write(&stick_y_var, new_stick_y.into());
        }

        let n_a = intended.yaw - up_angle;
        let n = [
            intended.mag / 32.0 * (-n_a.0 as f32 * PI / 0x8000 as f32).sin(),
            intended.mag / 32.0 * (n_a.0 as f32 * PI / 0x8000 as f32).cos(),
        ];

        ui.set_cursor_pos([ui.cursor_pos()[0] + 155.0, 0.0]);
        // TODO: Persist joystick state
        let new_n = JoystickControlUi::new().render(ui, id, n, JoystickControlShape::Circle);

        if let Some(new_n) = new_n {
            let new_n_a = Wrapping((f32::atan2(-new_n[0], new_n[1]) * 0x8000 as f32 / PI) as i16);
            let new_intended_yaw = up_angle + new_n_a;
            let new_intended_mag =
                (32.0 * (new_n[0].powi(2) + new_n[1].powi(2)).sqrt()).clamp(0.0, 32.0);

            let (new_raw_stick_x, new_raw_stick_y) = stick_intended_to_raw_heuristic(
                IntendedStick {
                    yaw: new_intended_yaw,
                    mag: new_intended_mag,
                },
                face_yaw,
                camera_yaw,
                squish_timer != 0,
                Wrapping(0),
            );

            pipeline.write(&stick_x_var, new_raw_stick_x.into());
            pipeline.write(&stick_y_var, new_raw_stick_y.into());
        }
    }
}
