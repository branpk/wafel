use std::{
    collections::HashMap,
    iter::{self, Peekable},
    mem,
    num::Wrapping,
    sync::Arc,
};

use bytemuck::cast_slice;
use fast3d::{
    cmd::{F3DCommand::*, *},
    decode::decode_f3d_display_list,
    interpret::{interpret_f3d_display_list, F3DRenderData},
    util::{MatrixState, Matrixf},
};
use itertools::Itertools;
use wafel_api::{Address, Error, IntType, Value};
use wafel_data_path::GlobalDataPath;
use wafel_data_type::{Namespace, TypeName};
use wafel_layout::DataLayout;
use wafel_memory::MemoryRead;

use crate::{
    sm64_gfx_tree::*,
    sm64_render_mod::{get_dl_addr, F3DMemoryImpl, Pointer, RawDlIter},
    SM64RenderConfig,
};

const DEBUG_PRINT: bool = false;

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

    let mut renderer = NodeRenderer::new(input_dl.into_iter(), memory, layout)?;

    // renderer.u32_buffer.extend(cast_slice(
    //     &Matrixf::perspective(45.0 * PI / 180.0, 320.0 / 240.0, 100.0, 12800.0).to_fixed(),
    // ));
    // renderer.display_list.push(SPMatrix {
    //     matrix: Pointer::BufferOffset(0),
    //     mode: MatrixMode::Proj,
    //     op: MatrixOp::Load,
    //     push: false,
    // });
    // renderer
    //     .display_list
    //     .push(SPSetGeometryMode(GeometryModes::LIGHTING));
    // renderer
    //     .display_list
    //     .push(DPSetCombineMode(CombineMode::one_cycle(
    //         ColorCombineComponent::Shade.into(),
    //         ColorCombineComponent::Shade.into(),
    //     )));

    if let Value::Address(root_addr) = get_path("gCurrentArea?.unk04")?.read(memory)? {
        let pause_rendering = get_path("gWarpTransition.pauseRendering")?
            .read(memory)?
            .try_as_int()?
            != 0;
        renderer.render_game(root_addr, pause_rendering)?;
    }

    let mut f3d_memory = F3DMemoryImpl::new(memory, Pointer::BufferOffset(0));
    // f3d_memory.set_view_transform(Some(Matrixf::look_at(
    //     [0.0, 0.0, 1000.0],
    //     [0.0, 0.0, 0.0],
    //     0.0,
    // )));
    f3d_memory.set_dl_buffer(vec![renderer.display_list]);
    f3d_memory.set_u32_buffer(renderer.u32_buffer);

    let render_data = interpret_f3d_display_list(&f3d_memory, config.screen_size, true)?;

    Ok(render_data)
}

#[derive(Debug)]
struct NodeRenderer<'m, M, I>
where
    I: Iterator<Item = F3DCommand<Pointer>>,
{
    input_display_list: Peekable<I>,
    memory: &'m M,
    layout: &'m DataLayout,
    reader: GfxNodeReader<'m>,
    mtx_stack: MatrixState,
    master_lists: [Vec<DisplayListNode>; 8],
    display_list_mtx_override: HashMap<Address, Vec<i32>>,
    display_list: Vec<F3DCommand<Pointer>>,
    u32_buffer: Vec<u32>,
    anim: Option<AnimState>,
    cur_root: Option<GraphNodeRoot>,
    cur_perspective: Option<GraphNodePerspective>,
    cur_camera: Option<GraphNodeCamera>,
    cur_object: Option<GraphNodeObject>,
    indent: usize,
}

#[derive(Debug)]
struct DisplayListNode {
    transform: Vec<i32>,
    display_list: Address,
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

        // println!("    {:04X} {:04X}", attr0, attr1);

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

impl<'m, M, I> NodeRenderer<'m, M, I>
where
    M: MemoryRead,
    I: Iterator<Item = F3DCommand<Pointer>>,
{
    fn new(input_display_list: I, memory: &'m M, layout: &'m DataLayout) -> Result<Self, Error> {
        let reader = get_gfx_node_reader(memory, layout)?;
        Ok(Self {
            input_display_list: input_display_list.peekable(),
            memory,
            layout,
            reader,
            mtx_stack: MatrixState::default(),
            master_lists: Default::default(),
            display_list_mtx_override: HashMap::new(),
            display_list: Vec::new(),
            u32_buffer: Vec::new(),
            anim: None,
            cur_root: None,
            cur_perspective: None,
            cur_camera: None,
            cur_object: None,
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

    fn append_display_list(&mut self, layer: i16) {
        self.master_lists[layer as usize].push(DisplayListNode {
            transform: self.mtx_stack.cur.to_fixed(),
            display_list: Address::NULL,
        });
    }

    fn append_display_list_with(&mut self, display_list: Address, layer: i16) {
        self.master_lists[layer as usize].push(DisplayListNode {
            transform: self.mtx_stack.cur.to_fixed(),
            display_list,
        });
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

    fn submit_master_lists(&mut self, z_buffer: bool) {
        // TODO: z buffer, render modes

        if z_buffer {
            self.push_cmd(SPSetGeometryMode(GeometryModes::ZBUFFER));
        }

        for (layer, lists) in mem::take(&mut self.master_lists).iter().enumerate() {
            let render_mode = self.get_render_mode(layer as i16, z_buffer);
            self.push_cmd(DPSetRenderMode(render_mode));

            for list in lists {
                let offset = self.u32_buffer.len();
                self.u32_buffer.extend(cast_slice(&list.transform));
                self.push_cmd(SPMatrix {
                    matrix: Pointer::BufferOffset(offset),
                    mode: MatrixMode::ModelView,
                    op: MatrixOp::Load,
                    push: false,
                });

                self.push_cmd(SPDisplayList(list.display_list.into()));
            }
        }

        if z_buffer {
            self.push_cmd(SPClearGeometryMode(GeometryModes::ZBUFFER));
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
        // println!("{:02X} {:?}", node.anim_id, self.anim);

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
            if flags.contains(GraphRenderFlags::ACTIVE) {
                if flags.contains(GraphRenderFlags::CHILDREN_FIRST) {
                    self.process_node_and_siblings(cur_node.node().children)?;
                } else {
                    if DEBUG_PRINT {
                        let indent_str = "  ".repeat(self.indent);
                        println!("{}{:?} {:?} {{", indent_str, cur_addr, cur_node);
                    }

                    self.indent += 1;
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
                        GfxTreeNode::Object(node) => self.process_object(node)?,
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
                    self.indent -= 1;

                    if DEBUG_PRINT {
                        let indent_str = "  ".repeat(self.indent);
                        println!("{}}}", indent_str);
                    }
                }
            }

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

    fn process_master_list_sub(&mut self, node: &GraphNodeMasterList) -> Result<(), Error> {
        if node.node.flags.contains(GraphRenderFlags::Z_BUFFER) {
            self.dl_push_expect(|cmd| matches!(cmd, DPPipeSync));
            self.dl_push_expect(|cmd| matches!(cmd, SPSetGeometryMode(_)));
        }

        // TODO: Need to set render mode and splice custom display lists in
        // TODO: Probably can remove calls to append_display_list

        while matches!(self.input_display_list.peek(), Some(DPSetRenderMode(_))) {
            self.dl_push_expect(|cmd| matches!(cmd, DPSetRenderMode(_)));
            while matches!(self.input_display_list.peek(), Some(SPMatrix { .. })) {
                let mut mtx_cmd = self.dl_expect(|cmd| matches!(cmd, SPMatrix { .. }));
                let dl_cmd = self.dl_expect(|cmd| matches!(cmd, SPDisplayList(_)));

                if let SPMatrix { matrix, .. } = &mut mtx_cmd {
                    if let SPDisplayList(Pointer::Address(addr)) = dl_cmd {
                        if let Some(new_mtx) = self.display_list_mtx_override.get(&addr) {
                            let offset = self.u32_buffer.len();
                            self.u32_buffer.extend(cast_slice(new_mtx));
                            *matrix = Pointer::BufferOffset(offset);
                        }
                    }
                }

                self.push_cmd(mtx_cmd);
                self.push_cmd(dl_cmd);
            }
        }

        //         for (layer, lists) in mem::take(&mut self.master_lists).into_iter().enumerate() {
        //             if !lists.is_empty() {
        //                 self.dl_push_expect(|cmd| matches!(cmd, DPSetRenderMode(_)));
        //
        //                 for list in lists {
        //                     self.dl_push_expect(|cmd| matches!(cmd, SPMatrix { .. }));
        //                     self.dl_push_expect(|cmd| matches!(cmd, SPDisplayList(_)));
        //
        //                     //                     let offset = self.u32_buffer.len();
        //                     //                     self.u32_buffer.extend(cast_slice(&list.transform));
        //                     //                     self.push_cmd(SPMatrix {
        //                     //                         matrix: Pointer::BufferOffset(offset),
        //                     //                         mode: MatrixMode::ModelView,
        //                     //                         op: MatrixOp::Load,
        //                     //                         push: false,
        //                     //                     });
        //                     //
        //                     //                     self.push_cmd(SPDisplayList(list.display_list.into()));
        //                 }
        //             }
        //         }

        if node.node.flags.contains(GraphRenderFlags::Z_BUFFER) {
            self.dl_push_expect(|cmd| matches!(cmd, DPPipeSync));
            self.dl_push_expect(|cmd| matches!(cmd, SPClearGeometryMode(_)));
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
        let dist_from_cam = (mtx[7] >> 16) as i16;

        if node.min_distance <= dist_from_cam && dist_from_cam < node.max_distance {
            self.process_node_and_siblings(node.node.children)?;
        }
        Ok(())
    }

    fn process_switch_case(&mut self, node: &GraphNodeSwitchCase) -> Result<(), Error> {
        // TODO: selected case not set if rendering a culled object

        let mut selected_child = node.fn_node.node.children;
        let mut i = 0;

        while !selected_child.is_null() && node.selected_case > i {
            selected_child = self.reader.read(selected_child)?.node().next;
            i += 1;
        }

        self.process_node_and_siblings(selected_child)?;
        Ok(())
    }

    fn process_camera(&mut self, node: &GraphNodeCamera) -> Result<(), Error> {
        let camera_transform = Matrixf::look_at(node.pos, node.focus, node.roll);
        self.mtx_stack.push_mul(camera_transform);

        self.dl_push_expect(|cmd| matches!(cmd, SPMatrix { .. }));

        self.cur_camera = Some(node.clone());
        self.process_node_and_siblings(node.fn_node.node.children)?;
        self.cur_camera = None;

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
        self.mtx_stack.push_mul(mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8);
        }
        self.process_node_and_siblings(node.node.children)?;

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
        self.mtx_stack.push_mul(mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mtx_stack.pop();
        Ok(())
    }

    fn process_rotation(&mut self, node: &GraphNodeRotation) -> Result<(), Error> {
        let translation = [0.0, 0.0, 0.0];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, node.rotation);
        self.mtx_stack.push_mul(mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mtx_stack.pop();
        Ok(())
    }

    fn process_object(&mut self, node: &GraphNodeObject) -> Result<(), Error> {
        // TODO: if (node->header.gfx.areaIndex == gCurGraphNodeRoot->areaIndex) {
        {
            // TODO: Matrix transform
            if !node.throw_matrix.is_null() {
                // TODO
                self.mtx_stack.push_mul(Matrixf::identity());
            } else if node.node.flags.contains(GraphRenderFlags::BILLBOARD) {
                // TODO
                self.mtx_stack.push_mul(Matrixf::identity());
            } else {
                let transform = Matrixf::rotate_zxy_and_translate(node.pos, node.angle);
                self.mtx_stack.push_mul(transform);
            }

            // TODO: Calculate throwMatrix and cameraToObject

            if !node.anim_info.cur_anim.is_null() {
                self.set_animation_globals(&node.anim_info)?;
            }
            // TODO: if (obj_is_in_view(&node->header.gfx, gMatStack[gMatStackIndex])) {
            {
                // TODO: Calculate matrix
                if !node.shared_child.is_null() {
                    // TODO: Set & unset shared_child parent
                    self.cur_object = Some(node.clone());
                    self.process_node_and_siblings(node.shared_child)?;
                    self.cur_object = None;
                }
                self.process_node_and_siblings(node.node.children)?;
            }

            self.mtx_stack.pop();
            self.anim = None;
            // TODO: Reset object throw matrix
        }

        Ok(())
    }

    fn process_animated_part(&mut self, node: &GraphNodeAnimatedPart) -> Result<(), Error> {
        let mut rotation = [Wrapping(0), Wrapping(0), Wrapping(0)];
        let mut translation = node.translation.map(|x| x as f32);

        // println!("{}{:?}", "  ".repeat(self.indent), translation);

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

        // eprintln!("    -> {:?} {:?}", translation, rotation);

        let transform = Matrixf::rotate_xyz_and_translate(translation, rotation);
        self.mtx_stack.push_mul(transform);

        if !node.display_list.is_null() {
            // self.append_display_list_with(node.display_list, node.node.flags.bits() >> 8);
            self.append_display_list(node.node.flags.bits() >> 8);
            self.display_list_mtx_override
                .insert(node.display_list, self.mtx_stack.cur.to_fixed());
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mtx_stack.pop();
        Ok(())
    }

    fn process_billboard(&mut self, node: &GraphNodeBillboard) -> Result<(), Error> {
        // TODO: Matrix transform

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8);
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_display_list(&mut self, node: &GraphNodeDisplayList) -> Result<(), Error> {
        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8);
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_scale(&mut self, node: &GraphNodeScale) -> Result<(), Error> {
        let mtx = Matrixf::scale_vec3f([node.scale, node.scale, node.scale]);
        self.mtx_stack.push_mul(mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags.bits() >> 8);
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mtx_stack.pop();
        Ok(())
    }

    fn process_shadow(&mut self, node: &GraphNodeShadow) -> Result<(), Error> {
        // TODO: extra objects + maybe append_display_list
        // TODO: matrix transform

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_object_parent(&mut self, node: &GraphNodeObjectParent) -> Result<(), Error> {
        // TODO: Do we need to set parent?
        if !node.shared_child.is_null() {
            self.process_node_and_siblings(node.shared_child)?;
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_generated(&mut self, node: &GraphNodeGenerated) -> Result<(), Error> {
        // TODO: append_display_list?
        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_background(&mut self, node: &GraphNodeBackground) -> Result<(), Error> {
        if !node.fn_node.func.is_null() {
            self.append_display_list(node.fn_node.node.flags.bits() >> 8);
        } else {
            self.append_display_list(0);
        }
        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_held_object(&mut self, node: &GraphNodeHeldObject) -> Result<(), Error> {
        // TODO: Animation globals?
        // TODO: Matrix transform
        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_culling_radius(&mut self, node: &GraphNodeCullingRadius) -> Result<(), Error> {
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }
}
