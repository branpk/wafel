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
    tabs::{TabInfo, Tabs},
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
    fn id(self) -> String {
        format!("tab-{:?}", self)
    }

    fn closable(self) -> bool {
        matches!(self, TabId::Object(_) | TabId::Surface(_))
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
        rotational_camera_yaw: Angle,
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
                .option()
                .unwrap_or(&Value::Int(0))
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
            "3d view" => rotational_camera_yaw,
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

    fn render_input_tab(
        &self,
        ui: &ig::Ui<'_>,
        tab: TabId,
        pipeline: &mut Pipeline,
        frame: u32,
        rotational_camera_yaw: Angle,
    ) {
        let column_sizes = [170.0, 370.0, 200.0];

        ig::ChildWindow::new(im_str!("##input"))
            .flags(ig::WindowFlags::HORIZONTAL_SCROLLBAR)
            .content_size([column_sizes.iter().copied().sum(), 0.0])
            .build(ui, || {
                ui.columns(3, im_str!("input-columns"), true);

                for (i, &w) in column_sizes.iter().enumerate() {
                    ui.set_column_width(i as i32, w);
                }

                let mut render_button = |button: &str| {
                    self.render_variable(
                        ui,
                        pipeline,
                        tab,
                        &Variable::new(&format!("input-button-{}", button)).with_frame(frame),
                        10.0,
                        25.0,
                    )
                };

                ui.dummy([1.0, 3.0]);
                {
                    render_button("a");
                    ui.same_line(0.0);
                    render_button("b");
                    ui.same_line(0.0);
                    render_button("z");
                }
                ui.dummy([1.0, 5.0]);
                {
                    render_button("s");
                    ui.same_line(0.0);
                    ui.dummy([43.0, 1.0]);
                    ui.same_line(0.0);
                    render_button("r");
                }
                ui.dummy([1.0, 5.0]);
                {
                    ui.dummy([43.0, 1.0]);
                    ui.same_line(0.0);
                    render_button("cu");
                }
                {
                    ui.dummy([17.0, 1.0]);
                    ui.same_line(0.0);
                    render_button("cl");
                    ui.same_line(0.0);
                    render_button("cr");
                }
                {
                    ui.dummy([43.0, 1.0]);
                    ui.same_line(0.0);
                    render_button("cd");
                }

                ui.next_column();
                self.render_intended_stick_control(
                    ui,
                    "intended",
                    tab,
                    pipeline,
                    frame,
                    rotational_camera_yaw,
                );

                ui.next_column();
                self.render_stick_control(ui, "joystick", tab, pipeline, frame);

                ui.columns(1, im_str!("input-columns-end"), true);
            });
    }

    fn render_frame_log_tab(&self, ui: &ig::Ui<'_>, pipeline: &Pipeline, frame: u32) {
        let mut frame_offset = 1; // TODO: Persist
        let mut round_numbers = true; // TODO: Persist

        ui.set_next_item_width(210.0);
        ig::ComboBox::new(im_str!("##frame-offset")).build_simple_string(
            ui,
            &mut frame_offset,
            &[
                im_str!("previous -> current frame"),
                im_str!("current -> next frame"),
            ],
        );

        ui.checkbox(im_str!("Round##round-numbers"), &mut round_numbers);
        ui.dummy([1.0, 10.0]);

        let events = pipeline.timeline().frame_log(frame + frame_offset as u32);

        let string = |addr: &Value| {
            String::from_utf8(pipeline.timeline().read_string_at(frame, addr.as_address()))
                .expect("invalid string")
        };
        let float = |number: &Value| {
            let value = number.as_f32();
            if round_numbers {
                format!("{:.3}", value)
            } else {
                format!("{}", value)
            }
        };
        let vec3f = |vector: &Value| {
            let coords = vector.as_array_with_len(3);
            format!(
                "({}, {}, {})",
                float(&coords[0]),
                float(&coords[1]),
                float(&coords[2]),
            )
        };
        let action = |value: &Value| {
            // TODO: return self.model.action_names[dcast(int, action)]
            value.to_string()
        };

        let mut indent = 0;
        let mut action_indent = 0;

        let show_text = |indent: usize, text: &str| {
            ui.text(format!("{:indent$}{}", " ", text, indent = indent * 4));
        };

        for event in &events {
            match event["type"].as_str() {
                "FLT_CHANGE_ACTION" => {
                    show_text(
                        indent,
                        &format!(
                            "change action: {} -> {}",
                            action(&event["from"]),
                            action(&event["to"]),
                        ),
                    );
                }
                "FLT_CHANGE_FORWARD_VEL" => {
                    show_text(
                        indent,
                        &format!(
                            "change f vel: {} -> {} ({})",
                            float(&event["from"]),
                            float(&event["to"]),
                            string(&event["reason"]),
                        ),
                    );
                }
                "FLT_WALL_PUSH" => {
                    show_text(
                        indent,
                        &format!(
                            "wall push: {} -> {} (surface {})",
                            vec3f(&event["from"]),
                            vec3f(&event["to"]),
                            event["surface"],
                        ),
                    );
                }
                "FLT_BEGIN_MOVEMENT_STEP" => {
                    let step_type = match event["stepType"].as_int() {
                        1 => "air",
                        2 => "ground",
                        3 => "water",
                        _ => unimplemented!(),
                    };
                    show_text(indent, &format!("{} step {}:", step_type, event["stepNum"]));
                    indent += 1;
                }
                "FLT_END_MOVEMENT_STEP" => {
                    indent -= 1;
                }
                "FLT_EXECUTE_ACTION" => {
                    indent -= action_indent;
                    action_indent = 0;
                    show_text(
                        indent,
                        &format!("execute action: {}", action(&event["action"])),
                    );
                    indent += 1;
                    action_indent += 1;
                }
                _ => {
                    let mut items: Vec<(String, Value)> = event.clone().into_iter().collect();
                    items.sort_by_key(|(k, _)| k.clone());
                    items.retain(|(k, _)| k != "type");
                    items.insert(0, ("type".to_string(), event["type"].clone()));
                    show_text(
                        indent,
                        &format!(
                            "{{ {} }}",
                            items
                                .iter()
                                .map(|(k, v)| format!("{}: {}", k, v))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                }
            }
        }
    }

    fn render_variable_tab(
        &self,
        ui: &ig::Ui<'_>,
        tab: TabId,
        pipeline: &mut Pipeline,
        frame: u32,
    ) {
        let variables = self.variables_for_tab(pipeline, frame, tab);
        for variable in &variables {
            self.render_variable(ui, pipeline, tab, &variable.with_frame(frame), 80.0, 80.0);
        }
    }

    fn render_tab_contents(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        tab: TabId,
        pipeline: &mut Pipeline,
        frame: u32,
        rotational_camera_yaw: Angle,
    ) {
        let id_token = ui.push_id(id);
        match tab {
            TabId::Input => self.render_input_tab(ui, tab, pipeline, frame, rotational_camera_yaw),
            TabId::Subframe => self.render_frame_log_tab(ui, pipeline, frame),
            TabId::Objects => self.render_objects_tab(ui, pipeline, frame),
            TabId::Mario | TabId::Misc | TabId::Object(_) | TabId::Surface(_) => {
                self.render_variable_tab(ui, tab, pipeline, frame)
            }
        }
        id_token.pop(ui);
    }

    pub(crate) fn render(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        pipeline: &mut Pipeline,
        frame: u32,
        rotational_camera_yaw: Angle,
    ) {
        let id_token = ui.push_id(id);

        let open_tab_index = self.open_tabs.iter().position(|&id| id == self.current_tab);

        let tab_info: Vec<TabInfo> = self
            .open_tabs
            .iter()
            .map(|&tab| TabInfo {
                id: tab.id(),
                label: self.tab_label(tab),
                closable: tab.closable(),
            })
            .collect();

        let result = Tabs::new().render(ui, "tabs", &tab_info, open_tab_index, true, |tab_index| {
            let tab = self.open_tabs[tab_index];
            self.render_tab_contents(ui, id, tab, pipeline, frame, rotational_camera_yaw);
        });

        if let Some(selected_tab_index) = result.selected_tab_index {
            self.current_tab = self.open_tabs[selected_tab_index];
        }
        if let Some(closed_tab_index) = result.closed_tab_index {
            self.open_tabs.remove(closed_tab_index);
        }

        id_token.pop(ui);
    }
}
