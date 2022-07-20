use std::{
    collections::HashSet, iter::Peekable, mem, num::Wrapping, ops::Deref, process, sync::Arc,
};

use bytemuck::{cast_slice, cast_slice_mut};
use fast3d::{
    cmd::{F3DCommand::*, *},
    decode::decode_f3d_display_list,
    interpret::{interpret_f3d_display_list, F3DRenderData},
    util::{coss, sins, Angle, MatrixStack, Matrixf},
};
use itertools::Itertools;
use wafel_api::{Address, Error, IntType, Value};
use wafel_data_path::GlobalDataPath;
use wafel_data_type::{DataType, Namespace, TypeName};
use wafel_layout::DataLayout;
use wafel_memory::{MemoryError, MemoryRead};

use crate::{
    sm64_gfx_tree::*,
    sm64_render_mod::{get_dl_addr, Camera, F3DMemoryImpl, ObjectCull, Pointer, RawDlIter},
    SM64RenderConfig,
};

const DEBUG_PRINT: bool = false;
const DEBUG_ONE_FRAME: bool = false;
const DEBUG_CALC_TRANSFORMS: bool = true;
const CHECK_CALC_TRANSFORMS: bool = false;
const ASSERT_CALC_TRANSFORMS: bool = false;

pub fn test_render(
    memory: &impl MemoryRead,
    layout: &DataLayout,
    mut get_path: impl FnMut(&str) -> Result<Arc<GlobalDataPath>, Error>,
    config: &SM64RenderConfig,
) -> Result<F3DRenderData, Error> {
    let dl_addr = get_dl_addr(memory, &mut get_path)?;
    let dl_addr = match dl_addr {
        Some(addr) => addr,
        None => return Ok(F3DRenderData::default()),
    };

    let raw_input_dl = RawDlIter {
        memory,
        addr: dl_addr,
    }
    .map(|cmd| cmd.map_err(Error::from));

    let input_dl: Vec<F3DCommand<Pointer>> =
        decode_f3d_display_list(raw_input_dl).collect::<Result<_, Error>>()?;

    if DEBUG_PRINT {
        println!("\n\n------- FRAME -------");
        for cmd in &input_dl {
            println!("  {:?}", cmd);
        }
        println!("\n\n");
    }

    let pause_rendering = get_path("gWarpTransition.pauseRendering")?
        .read(memory)?
        .try_as_int()?
        != 0;
    let root_addr = get_path("gCurrentArea?.unk04")?.read(memory)?;

    let mut renderer =
        NodeRenderer::new(config, input_dl.into_iter(), memory, layout, &mut get_path)?;

    if let Value::Address(root_addr) = root_addr {
        renderer.render_game(root_addr, pause_rendering)?;
    }

    let mut f3d_memory = F3DMemoryImpl::new(memory, Pointer::BufferOffset(0));
    f3d_memory.set_dl_buffer(vec![renderer.display_list]);
    f3d_memory.set_u32_buffer(renderer.u32_buffer);

    let render_data = interpret_f3d_display_list(&f3d_memory, config.screen_size, true)?;

    if DEBUG_ONE_FRAME {
        process::exit(0);
    }
    Ok(render_data)
}

#[derive(Debug)]
struct NodeRenderer<'m, M, I, F>
where
    I: Iterator<Item = F3DCommand<Pointer>>,
{
    config: &'m SM64RenderConfig,
    input_display_list: Peekable<I>,
    memory: &'m M,
    layout: &'m DataLayout,
    get_path: F,
    reader: GfxNodeReader<'m>,
    mtx_stack: MatrixStack,
    mod_mtx_stack: MatrixStack,
    master_lists: [Vec<MasterListEdit>; 8],
    display_list: Vec<F3DCommand<Pointer>>,
    u32_buffer: Vec<u32>,
    anim: Option<AnimState>,
    cur_root: Option<GraphNodeRoot>,
    cur_perspective: Option<GraphNodePerspective>,
    cur_camera: Option<GraphNodeCamera>,
    cur_camera_mtx: Option<Matrixf>,
    cur_mod_camera_mtx: Option<Matrixf>,
    cur_object: Option<GraphNodeObject>,
    cur_object_addr: Option<Address>,
    cur_object_throw_mtx: Option<Matrixf>,
    cur_object_mod_throw_mtx: Option<Matrixf>,
    cur_object_is_in_view: Option<bool>,
    cur_held_object: Option<GraphNodeHeldObject>,
    cur_node_addr: Option<Address>,
    is_active: Option<bool>,
    is_in_lod_range: Option<bool>,
    indent: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum MasterListEdit {
    Copy {
        transform: Vec<i32>,
        display_list: Address,
    },
    Skip(Address),
    Insert {
        transform: Vec<i32>,
        display_list: Address,
    },
    OptDynamic,
}

#[derive(Debug, Clone)]
struct DisplayListNode {
    transform: Address,
    display_list: Address,
    next: Address,
}

#[derive(Debug, Clone)]
struct AnimState {
    ty: AnimType,
    enabled: bool,
    frame: i16,
    translation_multiplier: f32,
    attribute: Address,
    data: Address,
}

impl AnimState {
    fn index(&mut self, memory: &impl MemoryRead) -> Result<i32, Error> {
        let frame = self.frame as i32;
        let attr0 = memory.read_int(self.attribute, IntType::U16)? as u16;
        let attr1 = memory.read_int(self.attribute + 2, IntType::U16)? as u16;

        let result = if frame < attr0 as i32 {
            attr1 as i32 + frame
        } else {
            attr1.wrapping_add(attr0).wrapping_sub(1) as i32
        };

        self.attribute += 4;
        Ok(result)
    }

    fn next(&mut self, memory: &impl MemoryRead) -> Result<i16, Error> {
        let index = self.index(memory)?;
        if index < 0 {
            // e.g. when mario is behind painting
            return Ok(0);
        }
        let result = memory.read_int(self.data + 2 * index as isize as usize, IntType::S16)? as i16;
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AnimType {
    Translation,
    VerticalTranslation,
    LateralTranslation,
    NoTranslation,
    Rotation,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CameraState {
    pos: [f32; 3],
    focus: [f32; 3],
    roll: Angle,
}

impl<'m, M, I, F> NodeRenderer<'m, M, I, F>
where
    M: MemoryRead,
    I: Iterator<Item = F3DCommand<Pointer>>,
    F: FnMut(&str) -> Result<Arc<GlobalDataPath>, Error>,
{
    fn new(
        config: &'m SM64RenderConfig,
        input_display_list: I,
        memory: &'m M,
        layout: &'m DataLayout,
        get_path: F,
    ) -> Result<Self, Error> {
        let reader = get_gfx_node_reader(memory, layout)?;
        Ok(Self {
            config,
            input_display_list: input_display_list.peekable(),
            memory,
            layout,
            get_path,
            reader,
            mtx_stack: MatrixStack::default(),
            mod_mtx_stack: MatrixStack::default(),
            master_lists: Default::default(),
            display_list: Vec::new(),
            u32_buffer: Vec::new(),
            anim: None,
            cur_root: None,
            cur_perspective: None,
            cur_camera: None,
            cur_object: None,
            cur_camera_mtx: None,
            cur_mod_camera_mtx: None,
            cur_object_addr: None,
            cur_object_throw_mtx: None,
            cur_object_mod_throw_mtx: None,
            cur_object_is_in_view: None,
            cur_held_object: None,
            cur_node_addr: None,
            is_active: None,
            is_in_lod_range: None,
            indent: 0,
        })
    }

    fn push_cmd(&mut self, cmd: F3DCommand<Pointer>) {
        if DEBUG_PRINT {
            println!("{}{:?}", "  ".repeat(self.indent), cmd);
        }
        self.display_list.push(cmd);
    }

    fn dl_push_until(&mut self, mut f: impl FnMut(F3DCommand<Pointer>) -> bool) -> bool {
        loop {
            match self.input_display_list.peek().copied() {
                Some(cmd) if f(cmd) => return true,
                Some(cmd) => {
                    self.push_cmd(cmd);
                    self.input_display_list.next();
                }
                None => return false,
            }
        }
    }

    #[track_caller]
    fn dl_expect(&mut self, mut f: impl FnMut(F3DCommand<Pointer>) -> bool) -> F3DCommand<Pointer> {
        if let Some(&cmd) = self.input_display_list.peek() {
            if f(cmd) {
                self.input_display_list.next();
                return cmd;
            }
        }
        // TODO: Error handling
        panic!(
            "unexpected display list cmd: {:?}",
            self.input_display_list.peek()
        );
    }

    #[track_caller]
    fn dl_push_expect(&mut self, f: impl FnMut(F3DCommand<Pointer>) -> bool) {
        let cmd = self.dl_expect(f);
        self.push_cmd(cmd);
    }

    fn edit_master_list(&mut self, layer: i16, edit: MasterListEdit) {
        self.master_lists[layer as usize].push(edit);
    }

    fn is_node_rendered(&self) -> bool {
        self.cur_object_is_in_view != Some(false)
            && self.is_in_lod_range != Some(false)
            && self.is_active != Some(false)
    }

    fn append_display_list(&mut self, layer: i16, display_list: Address) {
        if self.is_node_rendered() {
            self.edit_master_list(
                layer,
                MasterListEdit::Copy {
                    transform: self.mod_mtx_stack.cur.to_fixed(),
                    display_list,
                },
            );
        } else {
            self.edit_master_list(
                layer,
                MasterListEdit::Insert {
                    transform: self.mod_mtx_stack.cur.to_fixed(),
                    display_list,
                },
            );
        }
    }

    fn append_opt_dynamic_list(&mut self, layer: i16) {
        if self.is_node_rendered() {
            self.edit_master_list(layer, MasterListEdit::OptDynamic);
        }
    }

    fn get_render_mode(&self, layer: i16, z_buffer: bool) -> RenderMode {
        match (z_buffer, layer) {
            (false, 0) => RenderMode::RM_OPA_SURF(),
            (false, 1) => RenderMode::RM_AA_OPA_SURF(),
            (false, 2) => RenderMode::RM_AA_OPA_SURF(),
            (false, 3) => RenderMode::RM_AA_OPA_SURF(),
            (false, 4) => RenderMode::RM_AA_TEX_EDGE(),
            (false, 5) => RenderMode::RM_AA_XLU_SURF(),
            (false, 6) => RenderMode::RM_AA_XLU_SURF(),
            (false, 7) => RenderMode::RM_AA_XLU_SURF(),
            (true, 0) => RenderMode::RM_ZB_OPA_SURF(),
            (true, 1) => RenderMode::RM_AA_ZB_OPA_SURF(),
            (true, 2) => RenderMode::RM_AA_ZB_OPA_DECAL(),
            (true, 3) => RenderMode::RM_AA_ZB_OPA_INTER(),
            (true, 4) => RenderMode::RM_AA_ZB_TEX_EDGE(),
            (true, 5) => RenderMode::RM_AA_ZB_XLU_SURF(),
            (true, 6) => RenderMode::RM_AA_ZB_XLU_DECAL(),
            (true, 7) => RenderMode::RM_AA_ZB_XLU_INTER(),
            _ => unimplemented!("z_buffer={}, layer={}", z_buffer, layer),
        }
    }

    fn set_animation_globals(&mut self, node: &AnimInfo) -> Result<(), Error> {
        let animation_struct = self.layout.data_type(&TypeName {
            namespace: Namespace::Struct,
            name: "Animation".to_string(),
        })?;
        let resolve_type = |type_name: &TypeName| self.layout.data_type(type_name).ok().cloned();
        let anim = self
            .memory
            .read_value(node.cur_anim, animation_struct, resolve_type)?;

        let flags = anim.try_field("flags")?.try_as_int()? as i16;

        let ty;
        if flags & (1 << 3) != 0 {
            ty = AnimType::VerticalTranslation;
        } else if flags & (1 << 4) != 0 {
            ty = AnimType::LateralTranslation;
        } else if flags & (1 << 6) != 0 {
            ty = AnimType::NoTranslation;
        } else {
            ty = AnimType::Translation;
        }

        let y_trans = node.anim_y_trans;
        let y_trans_divisor = anim.try_field("animYTransDivisor")?.try_as_int()? as i16;
        let translation_multiplier = if y_trans_divisor == 0 {
            1.0
        } else {
            y_trans as f32 / y_trans_divisor as f32
        };

        self.anim = Some(AnimState {
            ty,
            enabled: flags & (1 << 5) == 0,
            frame: node.anim_frame,
            translation_multiplier,
            attribute: anim.try_field("index")?.try_as_address()?, // TODO: Seg to virt
            data: anim.try_field("values")?.try_as_address()?,     // TODO: Seg to virt
        });

        Ok(())
    }

    fn render_game(&mut self, root_addr: Address, pause_rendering: bool) -> Result<(), Error> {
        // Skip init_rcp and viewport/scissor override
        self.dl_push_until(|cmd| matches!(cmd, SPViewport(_)));

        if !pause_rendering {
            self.process_node(root_addr, false)?;
        }

        // Skip hud, in-game menu etc
        while let Some(cmd) = self.input_display_list.next() {
            self.push_cmd(cmd);
        }

        Ok(())
    }

    fn process_node_and_siblings(&mut self, first_addr: Address) -> Result<(), Error> {
        self.process_node(first_addr, true)
    }

    fn process_node(&mut self, first_addr: Address, siblings: bool) -> Result<(), Error> {
        if first_addr.is_null() {
            return Ok(());
        }
        let first_node = self.reader.read(first_addr)?;

        let mut iterate_siblings = siblings;
        let mut cur_addr = first_addr;
        let mut cur_node = first_node;

        if !cur_node.node().parent.is_null() {
            let parent_type = self.memory.read_int(cur_node.node().parent, IntType::S16)?;
            if parent_type == self.layout.constant("GRAPH_NODE_TYPE_SWITCH_CASE")?.value {
                iterate_siblings = false;
            }
        }

        loop {
            let flags = cur_node.node().flags;
            let is_active = flags.contains(GraphRenderFlags::ACTIVE);
            let parent_is_active = self.is_active;
            self.is_active = Some(is_active && parent_is_active != Some(false));

            let mut render_node = is_active;

            if self.config.object_cull == ObjectCull::ShowAll && !render_node {
                if let GfxTreeNode::Object(_) = &cur_node {
                    let object_struct = self.layout.data_type(&TypeName {
                        namespace: Namespace::Struct,
                        name: "Object".to_string(),
                    })?;
                    if let DataType::Struct { fields } = object_struct.as_ref() {
                        if let Some(field) = fields.get("activeFlags") {
                            let active_flags_offset = field.offset;

                            let active_flags = self
                                .memory
                                .read_int(cur_addr + active_flags_offset, IntType::S16)?
                                as i16;

                            let active_flag_active =
                                self.layout.constant("ACTIVE_FLAG_ACTIVE")?.value as i16;
                            let active_flag_far_away =
                                self.layout.constant("ACTIVE_FLAG_FAR_AWAY")?.value as i16;

                            if (active_flags & active_flag_active) != 0
                                && (active_flags & active_flag_far_away) != 0
                            {
                                render_node = true;
                            }
                        }
                    }
                }
            }

            if render_node {
                if flags.contains(GraphRenderFlags::CHILDREN_FIRST) {
                    self.process_node_and_siblings(cur_node.node().children)?;
                } else {
                    if DEBUG_PRINT {
                        let indent_str = "  ".repeat(self.indent);
                        println!("{}{:?} {:?} {{", indent_str, cur_addr, cur_node);
                    }

                    self.indent += 1;
                    self.cur_node_addr = Some(cur_addr);
                    match &cur_node {
                        GfxTreeNode::Root(node) => self.process_root(node)?,
                        GfxTreeNode::OrthoProjection(node) => {
                            self.process_ortho_projection(node)?
                        }
                        GfxTreeNode::Perspective(node) => self.process_perspective(node)?,
                        GfxTreeNode::MasterList(node) => self.process_master_list(node)?,
                        GfxTreeNode::Start(node) => self.process_start(node)?,
                        GfxTreeNode::LevelOfDetail(node) => self.process_level_of_detail(node)?,
                        GfxTreeNode::SwitchCase(node) => self.process_switch_case(node)?,
                        GfxTreeNode::Camera(node) => self.process_camera(node)?,
                        GfxTreeNode::TranslationRotation(node) => {
                            self.process_translation_rotation(node)?
                        }
                        GfxTreeNode::Translation(node) => self.process_translation(node)?,
                        GfxTreeNode::Rotation(node) => self.process_rotation(node)?,
                        GfxTreeNode::Object(node) => {
                            self.cur_object_addr = Some(cur_addr);
                            self.process_object(node)?;
                            self.cur_object_addr = None;
                        }
                        GfxTreeNode::AnimatedPart(node) => self.process_animated_part(node)?,
                        GfxTreeNode::Billboard(node) => self.process_billboard(node)?,
                        GfxTreeNode::DisplayList(node) => self.process_display_list(node)?,
                        GfxTreeNode::Scale(node) => self.process_scale(node)?,
                        GfxTreeNode::Shadow(node) => self.process_shadow(node)?,
                        GfxTreeNode::ObjectParent(node) => self.process_object_parent(node)?,
                        GfxTreeNode::Generated(node) => self.process_generated(node)?,
                        GfxTreeNode::Background(node) => self.process_background(node)?,
                        GfxTreeNode::HeldObject(node) => self.process_held_object(node)?,
                        GfxTreeNode::CullingRadius(node) => self.process_culling_radius(node)?,
                    }
                    self.cur_node_addr = None;
                    self.indent -= 1;

                    if DEBUG_PRINT {
                        let indent_str = "  ".repeat(self.indent);
                        println!("{}}}", indent_str);
                    }
                }
            }

            self.is_active = parent_is_active;

            if !iterate_siblings {
                break;
            }
            let next_addr = cur_node.node().next;
            if next_addr == first_addr {
                break;
            }
            cur_addr = next_addr;
            cur_node = self.reader.read(next_addr)?;
        }

        Ok(())
    }

    fn process_root(&mut self, node: &GraphNodeRoot) -> Result<(), Error> {
        // Skip viewport/scissor override
        self.dl_push_until(|cmd| matches!(cmd, SPViewport(_)));

        self.dl_push_expect(|cmd| matches!(cmd, SPViewport(_)));
        self.dl_push_expect(|cmd| matches!(cmd, SPMatrix { .. }));

        self.cur_root = Some(node.clone());
        self.process_node_and_siblings(node.node.children)?;
        self.cur_root = None;
        Ok(())
    }

    fn process_ortho_projection(&mut self, node: &GraphNodeOrthoProjection) -> Result<(), Error> {
        if !node.node.children.is_null() {
            self.dl_push_expect(|cmd| matches!(cmd, SPPerspNormalize(_)));
            self.dl_push_expect(|cmd| matches!(cmd, SPMatrix { .. }));
            self.process_node_and_siblings(node.node.children)?;
        }
        Ok(())
    }

    fn process_perspective(&mut self, node: &GraphNodePerspective) -> Result<(), Error> {
        if !node.fn_node.node.children.is_null() {
            self.dl_push_expect(|cmd| matches!(cmd, SPPerspNormalize(_)));
            self.dl_push_expect(|cmd| matches!(cmd, SPMatrix { .. }));

            self.cur_perspective = Some(node.clone());
            self.process_node_and_siblings(node.fn_node.node.children)?;
            self.cur_perspective = None;
        }
        Ok(())
    }

    fn read_display_list_node(&mut self, addr: Address) -> Result<DisplayListNode, Error> {
        let ptr_size = self.memory.pointer_int_type().size();

        Ok(DisplayListNode {
            transform: self.memory.read_address(addr)?,
            display_list: self.memory.read_address(addr + ptr_size)?,
            next: self.memory.read_address(addr + 2 * ptr_size)?,
        })
    }

    fn process_master_list_sub(&mut self, node: &GraphNodeMasterList) -> Result<(), Error> {
        let z_buffer = node.node.flags.contains(GraphRenderFlags::Z_BUFFER);
        if z_buffer {
            self.dl_push_expect(|cmd| matches!(cmd, DPPipeSync));
            self.dl_push_expect(|cmd| matches!(cmd, SPSetGeometryMode(_)));
        }

        // TODO: Could detect generated display lists for more accuracy

        let mtx_cmd = |mtx: Pointer| SPMatrix {
            matrix: mtx,
            mode: MatrixMode::ModelView,
            op: MatrixOp::Load,
            push: false,
        };

        let mut original_lists: [Vec<(Pointer, Pointer)>; 8] = Default::default();

        for layer in 0..8 {
            let mut dl_node_addr = node.list_heads[layer as usize];
            if !dl_node_addr.is_null() {
                let render_mode = self.get_render_mode(layer, z_buffer);
                self.dl_expect(|cmd| cmd == DPSetRenderMode(render_mode));

                while !dl_node_addr.is_null() {
                    let dl_node = self.read_display_list_node(dl_node_addr)?;
                    self.dl_expect(|cmd| cmd == mtx_cmd(dl_node.transform.into()));
                    self.dl_expect(|cmd| cmd == SPDisplayList(dl_node.display_list.into()));

                    original_lists[layer as usize]
                        .push((dl_node.transform.into(), dl_node.display_list.into()));

                    dl_node_addr = dl_node.next;
                }
            }
        }

        let pool_start = (self.get_path)("gGfxPool.buffer")?
            .address(self.memory)?
            .unwrap();
        let cmd_size = 2 * self.memory.pointer_int_type().size();
        let pool_size = self.layout.constant("GFX_POOL_SIZE")?.value as usize * cmd_size;
        let pool_end = pool_start + pool_size;
        let is_dynamic = |dl: Pointer| {
            if let Pointer::Address(addr) = dl {
                addr >= pool_start && addr < pool_end
            } else {
                false
            }
        };

        // TODO: Error handling for mismatched display list structure

        let edits = mem::take(&mut self.master_lists);
        let mut new_lists: [Vec<(Pointer, Pointer)>; 8] = Default::default();

        let mut mod_transform = Matrixf::identity();
        if self.config.camera != Camera::InGame {
            if let (Some(camera_mtx), Some(mod_camera_mtx)) =
                (&self.cur_camera_mtx, &self.cur_mod_camera_mtx)
            {
                mod_transform = mod_camera_mtx * &camera_mtx.invert_isometry();
            }
        }

        for (layer, layer_edits) in edits.iter().enumerate() {
            let mut actual_nodes = original_lists[layer].iter().peekable();
            let new_nodes = &mut new_lists[layer];

            for edit in layer_edits {
                match edit {
                    MasterListEdit::Copy {
                        transform,
                        display_list,
                    } => {
                        let (mtx, dl) = actual_nodes
                            .next()
                            .copied()
                            .filter(|&(_, dl)| dl == (*display_list).into())
                            .expect("master list discrepancy");

                        if DEBUG_CALC_TRANSFORMS || self.config.camera != Camera::InGame {
                            let offset = self.u32_buffer.len();
                            self.u32_buffer.extend(cast_slice(transform));
                            new_nodes.push((Pointer::BufferOffset(offset), dl));
                        } else {
                            new_nodes.push((mtx, dl));
                        }
                    }
                    MasterListEdit::Skip(addr) => {
                        actual_nodes
                            .next()
                            .copied()
                            .filter(|&(_, dl)| dl == (*addr).into())
                            .expect("master list discrepancy");
                    }
                    MasterListEdit::Insert {
                        transform,
                        display_list,
                    } => {
                        let offset = self.u32_buffer.len();
                        self.u32_buffer.extend(cast_slice(transform));
                        new_nodes.push((Pointer::BufferOffset(offset), (*display_list).into()));
                    }
                    MasterListEdit::OptDynamic => {
                        if let Some(&&(mtx, dl)) = actual_nodes.peek() {
                            if is_dynamic(dl) {
                                if DEBUG_CALC_TRANSFORMS || self.config.camera != Camera::InGame {
                                    let orig_mtx = Matrixf::from_fixed(&self.read_fixed(mtx)?);
                                    let new_mtx = &mod_transform * &orig_mtx;
                                    let offset = self.u32_buffer.len();
                                    self.u32_buffer.extend(cast_slice(&new_mtx.to_fixed()));
                                    new_nodes.push((Pointer::BufferOffset(offset), dl));
                                } else {
                                    new_nodes.push((mtx, dl));
                                }
                                actual_nodes.next();
                            }
                        }
                    }
                }
            }

            // Most likely from the mario head
            new_nodes.extend(actual_nodes);
        }

        for (layer, list) in new_lists.iter().enumerate() {
            if !list.is_empty() {
                let render_mode = self.get_render_mode(layer as i16, z_buffer);
                self.push_cmd(DPSetRenderMode(render_mode));

                for (mtx, dl) in list.iter().copied() {
                    self.push_cmd(mtx_cmd(mtx));
                    self.push_cmd(SPDisplayList(dl));
                }
            }
        }

        if CHECK_CALC_TRANSFORMS || ASSERT_CALC_TRANSFORMS {
            let mut matches = true;

            for layer in 0..8 {
                let old_list = &original_lists[layer];
                let new_list = &new_lists[layer];

                if old_list.len() != new_list.len() {
                    matches = false;
                    if ASSERT_CALC_TRANSFORMS {
                        eprintln!("{}: len {} -> {}", layer, old_list.len(), new_list.len());
                    }
                }
                for (old_node, new_node) in old_list.iter().zip(new_list.iter()) {
                    if old_node.1 != new_node.1 {
                        matches = false;
                        if ASSERT_CALC_TRANSFORMS {
                            eprintln!("{}: dl {:?} -> {:?}", layer, old_node.1, new_node.1);
                        }
                    }
                    let old_mtx = self.read_fixed(old_node.0)?;
                    let new_mtx = self.read_fixed(new_node.0)?;
                    if old_mtx != new_mtx {
                        matches = false;
                        if ASSERT_CALC_TRANSFORMS {
                            eprintln!(
                                "{}: mtx ({:?})\n{:?}\n{:?}",
                                layer,
                                self.symbol_name(old_node.1.address()),
                                Matrixf::from_fixed(&old_mtx),
                                Matrixf::from_fixed(&new_mtx),
                            );
                        }
                    }
                }
            }

            if !matches {
                eprintln!("display list mismatch");
            }
        }

        if z_buffer {
            self.dl_push_expect(|cmd| matches!(cmd, DPPipeSync));
            self.dl_push_expect(|cmd| matches!(cmd, SPClearGeometryMode(_)));
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn symbol_name(&self, addr: Address) -> Option<String> {
        self.layout
            .globals
            .iter()
            .find(|global| global.1.address == Some(addr.0 as u64))
            .map(|global| global.0.clone())
    }

    #[allow(dead_code)]
    fn read_fixed(&self, ptr: Pointer) -> Result<Vec<i32>, MemoryError> {
        let mut data = vec![0; 16];
        self.read_u32(cast_slice_mut(data.as_mut_slice()), ptr, 0)?;
        Ok(data)
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Pointer, offset: usize) -> Result<(), MemoryError> {
        match ptr {
            Pointer::Address(addr) => {
                let addr = addr + offset;
                for i in 0..dst.len() {
                    dst[i] = self.memory.read_int(addr + 4 * i, IntType::U32)? as u32;
                }
            }
            Pointer::BufferOffset(offset) => {
                dst.copy_from_slice(&self.u32_buffer[offset..offset + dst.len()]);
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    fn process_master_list(&mut self, node: &GraphNodeMasterList) -> Result<(), Error> {
        if !node.node.children.is_null() {
            self.process_node_and_siblings(node.node.children)?;
            self.process_master_list_sub(node)?;
        }
        Ok(())
    }

    fn process_start(&mut self, node: &GraphNodeStart) -> Result<(), Error> {
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_level_of_detail(&mut self, node: &GraphNodeLevelOfDetail) -> Result<(), Error> {
        let mtx = self.mtx_stack.cur.to_fixed();
        let dist_from_cam = -(mtx[7] >> 16) as i16;

        let is_in_lod_range =
            node.min_distance <= dist_from_cam && dist_from_cam < node.max_distance;
        if is_in_lod_range {
            let parent_in_lod_range = self.is_in_lod_range;
            self.is_in_lod_range = Some(is_in_lod_range && parent_in_lod_range != Some(false));
            self.process_node_and_siblings(node.node.children)?;
            self.is_in_lod_range = parent_in_lod_range;
        }
        Ok(())
    }

    fn process_switch_case(&mut self, node: &GraphNodeSwitchCase) -> Result<(), Error> {
        let mut selected_case = node.selected_case;

        if !node.fn_node.func.is_null() {
            // TODO: selected case not set if rendering a culled object
            // TODO: Model shared between different objects (when not using geo_switch_anim_state)

            if !node.fn_node.func.is_null() {
                let geo_switch_anim_state = (self.get_path)("geo_switch_anim_state")?
                    .address(self.memory)?
                    .unwrap();

                // Since different objects can use the same model, we need to calculate the switch
                // case manually in some cases
                if node.fn_node.func == geo_switch_anim_state {
                    let mut obj_addr = self.cur_object_addr;
                    if let Some(held_obj) = &self.cur_held_object {
                        obj_addr = Some(held_obj.obj_node);
                    }

                    if let Some(obj_addr) = obj_addr {
                        let object_struct = self.layout.data_type(&TypeName {
                            namespace: Namespace::Struct,
                            name: "Object".to_string(),
                        })?;
                        if let DataType::Struct { fields } = object_struct.as_ref() {
                            if let Some(field) = fields.get("oAnimState") {
                                let anim_state_offset = field.offset;

                                let anim_state = self
                                    .memory
                                    .read_int(obj_addr + anim_state_offset, IntType::S32)?
                                    as i32;

                                selected_case = anim_state as i16;
                            }
                        }
                    }
                }
            }
        }

        let mut selected_child = node.fn_node.node.children;
        let mut i = 0;

        while !selected_child.is_null() && selected_case > i {
            selected_child = self.reader.read(selected_child)?.node().next;
            i += 1;
        }

        self.process_node_and_siblings(selected_child)?;
        Ok(())
    }

    fn process_camera(&mut self, node: &GraphNodeCamera) -> Result<(), Error> {
        let camera_transform = Matrixf::look_at(node.pos, node.focus, node.roll);
        self.mtx_stack.push_mul(&camera_transform);

        let mod_camera_transform = match self.config.camera {
            Camera::InGame => camera_transform.clone(),
            Camera::LookAt { pos, focus, .. } => Matrixf::look_at(pos, focus, node.roll),
        };
        self.mod_mtx_stack.push_mul(&mod_camera_transform);

        let cmd = self.dl_expect(|cmd| matches!(cmd, SPMatrix { .. }));
        match self.config.camera {
            Camera::InGame => self.push_cmd(cmd),
            Camera::LookAt { roll, .. } => {
                let mtx = Matrixf::rotate_xy(roll);
                let offset = self.u32_buffer.len();
                self.u32_buffer.extend(cast_slice(&mtx.to_fixed()));
                self.push_cmd(SPMatrix {
                    matrix: Pointer::BufferOffset(offset),
                    mode: MatrixMode::Proj,
                    op: MatrixOp::Mul,
                    push: false,
                });
            }
        };

        self.cur_camera = Some(node.clone());
        self.cur_camera_mtx = Some(self.mtx_stack.cur.clone());
        self.cur_mod_camera_mtx = Some(self.mod_mtx_stack.cur.clone());

        self.process_node_and_siblings(node.fn_node.node.children)?;

        // Needed in master_list_sub:
        // self.cur_mod_camera_mtx = None;
        // self.cur_camera_mtx = None;
        self.cur_camera = None;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_translation_rotation(
        &mut self,
        node: &GraphNodeTranslationRotation,
    ) -> Result<(), Error> {
        let translation = [
            node.translation[0] as f32,
            node.translation[1] as f32,
            node.translation[2] as f32,
        ];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, node.rotation);
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_translation(&mut self, node: &GraphNodeTranslation) -> Result<(), Error> {
        let translation = [
            node.translation[0] as f32,
            node.translation[1] as f32,
            node.translation[2] as f32,
        ];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, Default::default());
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_rotation(&mut self, node: &GraphNodeRotation) -> Result<(), Error> {
        let translation = [0.0, 0.0, 0.0];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, node.rotation);
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn obj_is_in_view(&mut self, node: &GraphNodeObject) -> Result<bool, Error> {
        let matrix = self.mtx_stack.cur.clone();

        if node.node.flags.contains(GraphRenderFlags::INVISIBLE) {
            return Ok(false);
        }

        let geo = node.shared_child;

        let fov = self
            .cur_perspective
            .as_ref()
            .expect("no perspective set")
            .fov;
        let half_fov = Wrapping(((fov / 2.0 + 1.0) * 32768.0 / 180.0 + 0.5) as i16);

        let h_screen_edge = -matrix.0[2][3] * sins(half_fov) / coss(half_fov);

        let mut culling_radius = 300;
        if !geo.is_null() {
            if let GfxTreeNode::CullingRadius(node) = self.reader.read(geo)? {
                culling_radius = node.culling_radius;
            }
        }

        if matrix.0[2][3] > -100.0 + culling_radius as f32 {
            return Ok(false);
        }
        if matrix.0[2][3] < -20000.0 - culling_radius as f32 {
            return Ok(false);
        }

        if matrix.0[0][3] > h_screen_edge + culling_radius as f32 {
            return Ok(false);
        }
        if matrix.0[0][3] < -h_screen_edge - culling_radius as f32 {
            return Ok(false);
        }

        Ok(true)
    }

    fn is_mario(&mut self) -> Result<bool, Error> {
        let mario_object = (self.get_path)("gMarioObject")?
            .read(self.memory)?
            .try_as_address()?;
        Ok(self.cur_object_addr == Some(mario_object))
    }

    fn calc_throw_matrix(&mut self) -> Result<Option<Matrixf>, Error> {
        // TODO: Need more accurate condition for checking if align_with_floor was called
        //         if self.is_mario()? {
        //             let align_action_names = [
        //                 "ACT_CRAWLING",
        //                 "ACT_BUTT_SLIDE",
        //                 "ACT_HOLD_BUTT_SLIDE",
        //                 "ACT_CROUCH_SLIDE",
        //                 "ACT_DIVE_SLIDE",
        //                 "ACT_STOMACH_SLIDE",
        //                 "ACT_HOLD_STOMACH_SLIDE",
        //             ];
        //             let mut align_actions = HashSet::new();
        //             for name in &align_action_names {
        //                 align_actions.insert(self.layout.constant(name)?.value);
        //             }
        //
        //             let action = (self.get_path)("gMarioState.action")?
        //                 .read(self.memory)?
        //                 .try_as_int()?;
        //
        //             if align_actions.contains(&action) {
        //                 let mtx_addr = (self.get_path)("sFloorAlignMatrix[0]")?
        //                     .address(self.memory)?
        //                     .unwrap();
        //
        //                 let mut mtx = Matrixf::default();
        //                 self.read_u32(cast_slice_mut(&mut mtx.0), mtx_addr.into(), 0)?;
        //
        //                 return Ok(Some(mtx.transpose()));
        //             }
        //         }

        Ok(None)
    }

    fn mod_camera(&self) -> CameraState {
        match self.config.camera {
            Camera::InGame => {
                let camera = self.cur_camera.as_ref().expect("no current camera");
                CameraState {
                    pos: camera.pos,
                    focus: camera.focus,
                    roll: camera.roll,
                }
            }
            Camera::LookAt { pos, focus, roll } => CameraState { pos, focus, roll },
        }
    }

    fn process_object(&mut self, node: &GraphNodeObject) -> Result<(), Error> {
        let root_area = self.cur_root.as_ref().map(|r| r.area_index);
        if root_area.is_none() || root_area == Some(node.area_index as u8) {
            if let Some(throw_matrix) = self.calc_throw_matrix()? {
                self.mtx_stack.push_mul(&throw_matrix);
                self.mod_mtx_stack.push_mul(&throw_matrix);
            } else if node.node.flags.contains(GraphRenderFlags::BILLBOARD) {
                let mtx = Matrixf::billboard(
                    &self.mtx_stack.cur,
                    node.pos,
                    self.cur_camera.as_ref().expect("no current camera").roll,
                );
                self.mtx_stack.execute(&mtx, MatrixOp::Load, true);

                let mod_mtx =
                    Matrixf::billboard(&self.mod_mtx_stack.cur, node.pos, self.mod_camera().roll);
                self.mod_mtx_stack.execute(&mod_mtx, MatrixOp::Load, true);
            } else {
                let transform = Matrixf::rotate_zxy_and_translate(node.pos, node.angle);
                self.mtx_stack.push_mul(&transform);
                self.mod_mtx_stack.push_mul(&transform);
            }

            let scale_mtx = Matrixf::scale_vec3f(node.scale);
            self.mtx_stack.execute(&scale_mtx, MatrixOp::Mul, false);
            self.mod_mtx_stack.execute(&scale_mtx, MatrixOp::Mul, false);

            self.cur_object_throw_mtx = Some(self.mtx_stack.cur.clone());
            self.cur_object_mod_throw_mtx = Some(self.mod_mtx_stack.cur.clone());

            if !node.anim_info.cur_anim.is_null() {
                self.set_animation_globals(&node.anim_info)?;
            }

            let is_in_view = self.obj_is_in_view(node)?;
            let render_object = match self.config.object_cull {
                ObjectCull::Normal => is_in_view,
                ObjectCull::ShowAll => !node.node.flags.contains(GraphRenderFlags::INVISIBLE),
            };

            if render_object {
                if !node.shared_child.is_null() {
                    self.cur_object = Some(node.clone());
                    self.cur_object_is_in_view = Some(is_in_view);
                    self.process_node_and_siblings(node.shared_child)?;
                    self.cur_object_is_in_view = None;
                    self.cur_object = None;
                }
                self.process_node_and_siblings(node.node.children)?;
            }

            self.mod_mtx_stack.pop();
            self.mtx_stack.pop();
            self.anim = None;
            self.cur_object_mod_throw_mtx = None;
            self.cur_object_throw_mtx = None;
        }

        Ok(())
    }

    fn process_animated_part(&mut self, node: &GraphNodeAnimatedPart) -> Result<(), Error> {
        let mut rotation = [Wrapping(0), Wrapping(0), Wrapping(0)];
        let mut translation = node.translation.map(|x| x as f32);

        if let Some(anim) = &mut self.anim {
            match anim.ty {
                AnimType::Translation => {
                    translation[0] += anim.next(self.memory)? as f32 * anim.translation_multiplier;
                    translation[1] += anim.next(self.memory)? as f32 * anim.translation_multiplier;
                    translation[2] += anim.next(self.memory)? as f32 * anim.translation_multiplier;
                    anim.ty = AnimType::Rotation;
                }
                AnimType::LateralTranslation => {
                    translation[0] += anim.next(self.memory)? as f32 * anim.translation_multiplier;
                    anim.attribute += 4;
                    translation[2] += anim.next(self.memory)? as f32 * anim.translation_multiplier;
                    anim.ty = AnimType::Rotation;
                }
                AnimType::VerticalTranslation => {
                    anim.attribute += 4;
                    translation[1] += anim.next(self.memory)? as f32 * anim.translation_multiplier;
                    anim.attribute += 4;
                    anim.ty = AnimType::Rotation;
                }
                AnimType::NoTranslation => {
                    anim.attribute += 12;
                    anim.ty = AnimType::Rotation;
                }
                _ => {}
            }
            if anim.ty == AnimType::Rotation {
                rotation[0] = Wrapping(anim.next(self.memory)?);
                rotation[1] = Wrapping(anim.next(self.memory)?);
                rotation[2] = Wrapping(anim.next(self.memory)?);
            }
        }

        let transform = Matrixf::rotate_xyz_and_translate(translation, rotation);
        self.mtx_stack.push_mul(&transform);
        self.mod_mtx_stack.push_mul(&transform);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_billboard(&mut self, node: &GraphNodeBillboard) -> Result<(), Error> {
        let translation = [
            node.translation[0] as f32,
            node.translation[1] as f32,
            node.translation[2] as f32,
        ];

        let mtx = Matrixf::billboard(
            &self.mtx_stack.cur,
            translation,
            self.cur_camera.as_ref().expect("no current camera").roll,
        );
        self.mtx_stack.execute(&mtx, MatrixOp::Load, true);

        let mod_mtx =
            Matrixf::billboard(&self.mod_mtx_stack.cur, translation, self.mod_camera().roll);
        self.mod_mtx_stack.execute(&mod_mtx, MatrixOp::Load, true);

        let mut cur_obj = self.cur_object.clone();
        if let Some(node) = self.cur_held_object.as_ref() {
            if let GfxTreeNode::Object(node) = self.reader.read(node.obj_node)? {
                cur_obj = Some(node);
            }
        }
        if let Some(obj) = cur_obj {
            let scale_matrix = Matrixf::scale_vec3f(obj.scale);
            self.mtx_stack.execute(&scale_matrix, MatrixOp::Mul, false);
            self.mod_mtx_stack
                .execute(&scale_matrix, MatrixOp::Mul, false);
        }

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_display_list(&mut self, node: &GraphNodeDisplayList) -> Result<(), Error> {
        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_scale(&mut self, node: &GraphNodeScale) -> Result<(), Error> {
        let mtx = Matrixf::scale_vec3f([node.scale, node.scale, node.scale]);
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8, node.display_list);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_shadow(&mut self, node: &GraphNodeShadow) -> Result<(), Error> {
        if let (Some(camera), Some(object)) = (&self.cur_camera, &self.cur_object) {
            let camera_mtx = self.cur_camera_mtx.as_ref().unwrap();
            let mod_camera_mtx = self.cur_mod_camera_mtx.as_ref().unwrap();

            let mut shadow_pos;
            if self.cur_held_object.is_some() {
                shadow_pos = self.mtx_stack.cur.pos_from_transform_mtx(camera_mtx);
            } else {
                shadow_pos = object.pos;
            }

            if let Some(anim) = &mut self.anim {
                if anim.enabled
                    && matches!(
                        anim.ty,
                        AnimType::Translation | AnimType::LateralTranslation
                    )
                {
                    let mut obj_scale = 1.0;

                    let geo = node.node.children;
                    if !geo.is_null() {
                        if let GfxTreeNode::Scale(scale) = self.reader.read(geo)? {
                            obj_scale = scale.scale;
                        }
                    }

                    let anim_offset_x =
                        anim.next(self.memory)? as f32 * anim.translation_multiplier * obj_scale;
                    anim.attribute += 4;
                    let anim_offset_z =
                        anim.next(self.memory)? as f32 * anim.translation_multiplier * obj_scale;
                    anim.attribute -= 12;

                    let sin_ang = sins(object.angle[1]);
                    let cos_ang = coss(object.angle[1]);

                    shadow_pos[0] += anim_offset_x * cos_ang + anim_offset_z * sin_ang;
                    shadow_pos[2] += -anim_offset_x * sin_ang + anim_offset_z * cos_ang;
                }
            }

            let mod_mtx = mod_camera_mtx * &Matrixf::translate(shadow_pos);

            for layer in [4, 5, 6] {
                self.append_opt_dynamic_list(layer);
            }
        }

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_object_parent(&mut self, node: &GraphNodeObjectParent) -> Result<(), Error> {
        if !node.shared_child.is_null() {
            self.process_node_and_siblings(node.shared_child)?;
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_generated(&mut self, node: &GraphNodeGenerated) -> Result<(), Error> {
        self.append_opt_dynamic_list(node.fn_node.node.flags.bits() >> 8);
        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_background(&mut self, node: &GraphNodeBackground) -> Result<(), Error> {
        let layer = if !node.fn_node.func.is_null() {
            node.fn_node.node.flags.bits() >> 8
        } else {
            0
        };
        self.append_opt_dynamic_list(layer);

        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_held_object(&mut self, node: &GraphNodeHeldObject) -> Result<(), Error> {
        if !node.obj_node.is_null() {
            if let GfxTreeNode::Object(obj_node) = self.reader.read(node.obj_node)? {
                if !obj_node.shared_child.is_null() {
                    let translation = [
                        node.translation[0] as f32 / 4.0,
                        node.translation[1] as f32 / 4.0,
                        node.translation[2] as f32 / 4.0,
                    ];

                    let translate = Matrixf::translate(translation);

                    let mut throw = self
                        .cur_object_throw_mtx
                        .clone()
                        .expect("no current object");
                    throw.0[0][3] = self.mtx_stack.cur.0[0][3];
                    throw.0[1][3] = self.mtx_stack.cur.0[1][3];
                    throw.0[2][3] = self.mtx_stack.cur.0[2][3];

                    let mut mod_throw = self
                        .cur_object_mod_throw_mtx
                        .clone()
                        .expect("no current object");
                    mod_throw.0[0][3] = self.mod_mtx_stack.cur.0[0][3];
                    mod_throw.0[1][3] = self.mod_mtx_stack.cur.0[1][3];
                    mod_throw.0[2][3] = self.mod_mtx_stack.cur.0[2][3];

                    let mtx = &(&throw * &translate) * &Matrixf::scale_vec3f(obj_node.scale);
                    self.mtx_stack.execute(&mtx, MatrixOp::Load, true);

                    let mod_mtx =
                        &(&mod_throw * &translate) * &Matrixf::scale_vec3f(obj_node.scale);
                    self.mod_mtx_stack.execute(&mod_mtx, MatrixOp::Load, true);

                    let temp_anim_state = mem::take(&mut self.anim);
                    self.cur_held_object = Some(node.clone());

                    if !obj_node.anim_info.cur_anim.is_null() {
                        self.set_animation_globals(&obj_node.anim_info)?;
                    }
                    self.process_node_and_siblings(obj_node.shared_child)?;

                    self.cur_held_object = None;
                    self.anim = temp_anim_state;

                    self.mod_mtx_stack.pop();
                    self.mtx_stack.pop();
                }
            }
        }

        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_culling_radius(&mut self, node: &GraphNodeCullingRadius) -> Result<(), Error> {
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }
}
