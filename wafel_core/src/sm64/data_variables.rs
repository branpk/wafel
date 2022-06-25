use indexmap::IndexMap;
use wafel_api::{DataType, IntValue, Timeline, Value};

use crate::error::Error;

use super::{util, SM64ErrorCause, Variable};

#[rustfmt::skip]
fn build_variables(builder: &mut Builder) {
    builder.group("Input", |group| {
        group.variable("input-stick-x").path("gControllerPads[0].stick_x").label("stick x");
        group.variable("input-stick-y").path("gControllerPads[0].stick_y").label("stick y");
        group.variable("input-buttons").path("gControllerPads[0].button");
        group.variable("input-button-a").path("gControllerPads[0].button").flag("A_BUTTON").label("A");
        group.variable("input-button-b").path("gControllerPads[0].button").flag("B_BUTTON").label("B");
        group.variable("input-button-z").path("gControllerPads[0].button").flag("Z_TRIG").label("Z");
        group.variable("input-button-s").path("gControllerPads[0].button").flag("START_BUTTON").label("S");
        group.variable("input-button-l").path("gControllerPads[0].button").flag("L_TRIG").label("L");
        group.variable("input-button-r").path("gControllerPads[0].button").flag("R_TRIG").label("R");
        group.variable("input-button-cu").path("gControllerPads[0].button").flag("U_CBUTTONS").label("C^");
        group.variable("input-button-cl").path("gControllerPads[0].button").flag("L_CBUTTONS").label("C<");
        group.variable("input-button-cr").path("gControllerPads[0].button").flag("R_CBUTTONS").label("C>");
        group.variable("input-button-cd").path("gControllerPads[0].button").flag("D_CBUTTONS").label("Cv");
        group.variable("input-button-du").path("gControllerPads[0].button").flag("U_JPAD").label("D^");
        group.variable("input-button-dl").path("gControllerPads[0].button").flag("L_JPAD").label("D<");
        group.variable("input-button-dr").path("gControllerPads[0].button").flag("R_JPAD").label("D>");
        group.variable("input-button-dd").path("gControllerPads[0].button").flag("D_JPAD").label("Dv");
    });
    builder.group("Mario", |group| {
        group.variable("mario-pos-x").path("gMarioState->pos[0]").label("pos x");
        group.variable("mario-pos-y").path("gMarioState->pos[1]").label("pos y");
        group.variable("mario-pos-z").path("gMarioState->pos[2]").label("pos z");
        group.variable("mario-vel-f").path("gMarioState->forwardVel").label("vel f");
        group.variable("mario-vel-x").path("gMarioState->vel[0]").label("vel x");
        group.variable("mario-vel-y").path("gMarioState->vel[1]").label("vel y");
        group.variable("mario-vel-z").path("gMarioState->vel[2]").label("vel z");
        group.variable("mario-face-pitch").path("gMarioState->faceAngle[0]").label("face pitch");
        group.variable("mario-face-yaw").path("gMarioState->faceAngle[1]").label("face yaw");
        group.variable("mario-face-roll").path("gMarioState->faceAngle[2]").label("face roll");
        group.variable("mario-action").path("gMarioState->action").label("action");
    });
    builder.group("Misc", |group| {
        group.variable("global-timer").path("gGlobalTimer").label("global timer");
        group.variable("camera-yaw").path("gMarioState->area?->camera?->yaw").label("camera yaw");
        group.variable("level-num").path("gCurrLevelNum").label("level");
        group.variable("area-index").path("gCurrAreaIndex").label("area");
    });
    builder.group("Object", |group| {
        group.variable("obj-active-flags-active").path("struct Object.activeFlags").flag("ACTIVE_FLAG_ACTIVE").label("active");
        group.variable("obj-behavior-ptr").path("struct Object.behavior");
        group.variable("obj-hitbox-radius").path("struct Object.hitboxRadius").label("hitbox radius");
        group.variable("obj-hitbox-height").path("struct Object.hitboxHeight").label("hitbox height");
        group.variable("obj-pos-x").path("struct Object.oPosX").label("pos x");
        group.variable("obj-pos-y").path("struct Object.oPosY").label("pos y");
        group.variable("obj-pos-z").path("struct Object.oPosZ").label("pos z");
        group.variable("obj-vel-f").path("struct Object.oForwardVel").label("vel f");
        group.variable("obj-vel-x").path("struct Object.oVelX").label("vel x");
        group.variable("obj-vel-y").path("struct Object.oVelY").label("vel y");
        group.variable("obj-vel-z").path("struct Object.oVelZ").label("vel z");
    });
    builder.group("Surface", |group| {
        group.variable("surface-normal-x").path("struct Surface.normal.x").label("normal x");
        group.variable("surface-normal-y").path("struct Surface.normal.y").label("normal y");
        group.variable("surface-normal-z").path("struct Surface.normal.z").label("normal z");
        group.variable("surface-vertex1-x").path("struct Surface.vertex1[0]").label("x1");
        group.variable("surface-vertex1-y").path("struct Surface.vertex1[1]").label("y1");
        group.variable("surface-vertex1-z").path("struct Surface.vertex1[2]").label("z1");
        group.variable("surface-vertex2-x").path("struct Surface.vertex2[0]").label("x2");
        group.variable("surface-vertex2-y").path("struct Surface.vertex2[1]").label("y2");
        group.variable("surface-vertex2-z").path("struct Surface.vertex2[2]").label("z2");
        group.variable("surface-vertex3-x").path("struct Surface.vertex3[0]").label("x3");
        group.variable("surface-vertex3-y").path("struct Surface.vertex3[1]").label("y3");
        group.variable("surface-vertex3-z").path("struct Surface.vertex3[2]").label("z3");
    });
}

#[derive(Debug, Clone)]
enum Path {
    Global(String),
    Object(String),
    Surface(String),
}

#[derive(Debug, Clone)]
struct DataVariableSpec {
    group: String,
    path: Path,
    label: Option<String>,
    flag: Option<String>,
    flag_mask: Option<usize>,
}

#[derive(Debug)]
pub struct DataVariables {
    specs: IndexMap<String, DataVariableSpec>,
}

impl DataVariables {
    pub fn all(timeline: &Timeline) -> Result<Self, Error> {
        let mut builder = Builder::new();
        build_variables(&mut builder);
        let specs = builder.build(timeline)?;
        Ok(Self { specs })
    }

    pub fn group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = Variable> + 'a {
        self.specs
            .iter()
            .filter(move |(_, spec)| spec.group == group)
            .map(|(name, _)| Variable::new(name))
    }

    fn variable_spec(&self, variable: &str) -> Result<&DataVariableSpec, Error> {
        self.specs.get(variable).ok_or_else(|| {
            SM64ErrorCause::UnhandledVariable {
                variable: variable.to_owned(),
            }
            .into()
        })
    }

    fn path(
        &self,
        timeline: &Timeline,
        frame: u32,
        variable: &Variable,
    ) -> Result<Option<String>, Error> {
        let spec = self.specs.get(variable.name.as_ref()).ok_or_else(|| {
            SM64ErrorCause::UnhandledVariable {
                variable: variable.to_string(),
            }
        })?;

        let path = match &spec.path {
            Path::Global(path) => Some(path.clone()),
            Path::Object(path) => {
                let object = variable.try_object()?;
                match util::object_path(timeline, frame, object)? {
                    Some(object_path) => {
                        if let Some(expected_behavior) = &variable.object_behavior {
                            let actual_behavior =
                                util::object_behavior(timeline, frame, &object_path)?;
                            if &actual_behavior != expected_behavior {
                                return Ok(None);
                            }
                        }

                        Some(util::concat_object_path(&object_path, path))
                    }
                    None => None,
                }
            }
            Path::Surface(path) => {
                let surface = variable.try_surface()?;
                util::surface_path(timeline, frame, surface)?
                    .map(|surface_path| util::concat_surface_path(&surface_path, path))
            }
        };

        match (path, spec.flag.clone()) {
            (Some(path), Some(flag)) => Ok(Some(format!("{} & {}", path, flag))),
            (path, _) => Ok(path),
        }
    }

    pub fn get(
        &self,
        timeline: &Timeline,
        frame: u32,
        variable: &Variable,
    ) -> Result<Value, Error> {
        assert!(variable.frame.is_none() || variable.frame == Some(frame));

        let spec = self.variable_spec(&variable.name)?;
        let path = self.path(timeline, frame, variable)?;
        match path {
            Some(path) => {
                let mut value = timeline.try_read(frame, &path)?;

                if spec.flag.is_some() {
                    // Convert to a boolean value
                    value = Value::Int((value.try_as_int()? != 0) as IntValue);
                }

                Ok(value)
            }
            None => Ok(Value::None),
        }
    }

    pub fn set(
        &self,
        timeline: &mut Timeline,
        frame: u32,
        variable: &Variable,
        mut value: Value,
    ) -> Result<(), Error> {
        assert!(variable.frame.is_none() || variable.frame == Some(frame));

        let spec = self.variable_spec(&variable.name)?;
        if let Some(path) = self.path(timeline, frame, variable)? {
            if let Some(flag_mask) = spec.flag_mask {
                if value.try_as_int()? != 0 {
                    value = flag_mask.into();
                }
            }
            timeline.try_write(frame, &path, value)?;
        }

        Ok(())
    }

    pub fn reset(
        &self,
        timeline: &mut Timeline,
        frame: u32,
        variable: &Variable,
    ) -> Result<(), Error> {
        assert!(variable.frame.is_none() || variable.frame == Some(frame));
        if let Some(path) = self.path(timeline, frame, variable)? {
            timeline.try_reset(frame, &path)?;
        }
        Ok(())
    }

    /// Get the label for the given variable if it has one.
    pub fn label(&self, variable: &Variable) -> Result<Option<&str>, Error> {
        let spec = self.variable_spec(&variable.name)?;
        Ok(spec.label.as_ref().map(String::as_ref))
    }

    /// Get the concrete data type for the given variable.
    pub fn data_type(&self, timeline: &Timeline, variable: &Variable) -> Result<DataType, Error> {
        let spec = self.variable_spec(&variable.name)?;
        match &spec.path {
            Path::Global(path) | Path::Object(path) | Path::Surface(path) => {
                Ok(timeline.try_data_type(path)?)
            }
        }
    }

    /// Get the bit flag for the given variable if it has one.
    pub fn flag(&self, variable: &Variable) -> Result<Option<usize>, Error> {
        let spec = self.variable_spec(&variable.name)?;
        Ok(spec.flag_mask)
    }
}

struct Builder {
    groups: Vec<GroupBuilder>,
}

impl Builder {
    fn new() -> Self {
        Self { groups: Vec::new() }
    }

    fn group(&mut self, group: &str, func: impl FnOnce(&mut GroupBuilder)) {
        let mut group_builder = GroupBuilder::new(group);
        func(&mut group_builder);
        self.groups.push(group_builder);
    }

    fn build(self, timeline: &Timeline) -> Result<IndexMap<String, DataVariableSpec>, Error> {
        let mut specs = IndexMap::new();
        for group in self.groups {
            for variable in group.variables {
                let path_source = variable.path.unwrap();
                let path = if path_source.starts_with("struct Object") {
                    Path::Object(path_source)
                } else if path_source.starts_with("struct Surface") {
                    Path::Surface(path_source)
                } else {
                    Path::Global(path_source)
                };

                let flag_mask = match variable.flag.clone() {
                    Some(flag_name) => {
                        Some(timeline.try_constant(&flag_name)?.try_as_int()? as usize)
                    }
                    None => None,
                };

                let spec = DataVariableSpec {
                    group: group.name.clone(),
                    path,
                    label: variable.label,
                    flag: variable.flag,
                    flag_mask,
                };

                specs.insert(variable.name, spec);
            }
        }
        Ok(specs)
    }
}

struct GroupBuilder {
    name: String,
    variables: Vec<VariableBuilder>,
}

impl GroupBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            variables: Vec::new(),
        }
    }

    fn variable(&mut self, name: &str) -> &mut VariableBuilder {
        self.variables.push(VariableBuilder::new(name));
        self.variables.last_mut().unwrap()
    }
}

struct VariableBuilder {
    name: String,
    path: Option<String>,
    flag: Option<String>,
    label: Option<String>,
}

impl VariableBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            path: None,
            flag: None,
            label: None,
        }
    }

    fn path(&mut self, path: &str) -> &mut Self {
        self.path = Some(path.to_owned());
        self
    }

    fn flag(&mut self, flag: &str) -> &mut Self {
        self.flag = Some(flag.to_owned());
        self
    }

    fn label(&mut self, label: &str) -> &mut Self {
        self.label = Some(label.to_owned());
        self
    }
}
