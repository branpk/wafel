use std::f32::consts::PI;

use bytemuck::cast_slice;
use fast3d::{
    cmd::*,
    interpret::{interpret_f3d_display_list, F3DRenderData},
    util::{MatrixState, Matrixf},
};
use wafel_api::{Address, Error, IntType};
use wafel_data_type::{Namespace, TypeName};
use wafel_layout::DataLayout;
use wafel_memory::MemoryRead;

use crate::{
    sm64_gfx_tree::*,
    sm64_render_mod::{F3DMemoryImpl, Pointer},
};

pub fn test_render(
    memory: &impl MemoryRead,
    layout: &DataLayout,
    root_addr: Address,
) -> Result<F3DRenderData, Error> {
    let mut renderer = NodeRenderer::new(memory, layout)?;

    renderer.u32_buffer.extend(cast_slice(
        &Matrixf::perspective(45.0 * PI / 180.0, 320.0 / 240.0, 100.0, 12800.0).to_fixed(),
    ));
    renderer.display_list.push(F3DCommand::SPMatrix {
        matrix: Pointer::BufferOffset(0),
        mode: MatrixMode::Proj,
        op: MatrixOp::Load,
        push: false,
    });
    renderer
        .display_list
        .push(F3DCommand::SPSetGeometryMode(GeometryModes::LIGHTING));
    renderer
        .display_list
        .push(F3DCommand::DPSetCombineMode(CombineMode::one_cycle(
            ColorCombineComponent::Shade.into(),
            ColorCombineComponent::Shade.into(),
        )));

    renderer.process_node(root_addr, false)?;
    renderer.submit_master_lists(true);

    let mut f3d_memory = F3DMemoryImpl::new(memory, Pointer::BufferOffset(0));
    f3d_memory.set_view_transform(Some(Matrixf::look_at(
        [0.0, 0.0, 1000.0],
        [0.0, 0.0, 0.0],
        0.0,
    )));
    f3d_memory.set_dl_buffer(vec![renderer.display_list]);
    f3d_memory.set_u32_buffer(renderer.u32_buffer);

    let render_data = interpret_f3d_display_list(&f3d_memory, (320, 240), true)?;

    Ok(render_data)
}

#[derive(Debug)]
struct NodeRenderer<'m, M> {
    memory: &'m M,
    layout: &'m DataLayout,
    reader: GfxNodeReader<'m>,
    mtx_stack: MatrixState,
    master_lists: [Vec<DisplayListNode>; 8],
    display_list: Vec<F3DCommand<Pointer>>,
    u32_buffer: Vec<u32>,
    anim: Option<AnimState>,
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

        let result = if frame < attr0 as i32 {
            attr1 as i32 + frame
        } else {
            (attr1 + attr0 - 1) as i32
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

impl<'m, M: MemoryRead> NodeRenderer<'m, M> {
    fn new(memory: &'m M, layout: &'m DataLayout) -> Result<Self, Error> {
        let reader = get_gfx_node_reader(memory, layout)?;
        Ok(Self {
            memory,
            layout,
            reader,
            mtx_stack: MatrixState::default(),
            master_lists: Default::default(),
            display_list: Vec::new(),
            u32_buffer: Vec::new(),
            anim: None,
        })
    }

    fn append_display_list(&mut self, display_list: Address, layer: i16) {
        if !display_list.is_null() {
            // TODO: Set transform
            self.master_lists[layer as usize].push(DisplayListNode {
                transform: self.mtx_stack.cur.to_fixed(),
                display_list,
            });
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

    fn submit_master_lists(&mut self, z_buffer: bool) {
        // TODO: z buffer, render modes

        if z_buffer {
            self.display_list
                .push(F3DCommand::SPSetGeometryMode(GeometryModes::ZBUFFER));
        }

        for (layer, lists) in self.master_lists.iter().enumerate() {
            let render_mode = self.get_render_mode(layer as i16, z_buffer);
            self.display_list
                .push(F3DCommand::DPSetRenderMode(render_mode));

            for list in lists {
                let offset = self.u32_buffer.len();
                self.u32_buffer.extend(cast_slice(&list.transform));
                self.display_list.push(F3DCommand::SPMatrix {
                    matrix: Pointer::BufferOffset(offset),
                    mode: MatrixMode::ModelView,
                    op: MatrixOp::Load,
                    push: false,
                });

                self.display_list
                    .push(F3DCommand::SPDisplayList(list.display_list.into()));
            }
        }

        if z_buffer {
            self.display_list
                .push(F3DCommand::SPClearGeometryMode(GeometryModes::ZBUFFER));
        }
    }

    // #define ANIM_FLAG_NOLOOP     (1 << 0) // 0x01
    // #define ANIM_FLAG_FORWARD    (1 << 1) // 0x02
    // #define ANIM_FLAG_2          (1 << 2) // 0x04
    // #define ANIM_FLAG_HOR_TRANS  (1 << 3) // 0x08
    // #define ANIM_FLAG_VERT_TRANS (1 << 4) // 0x10
    // #define ANIM_FLAG_5          (1 << 5) // 0x20
    // #define ANIM_FLAG_6          (1 << 6) // 0x40
    // #define ANIM_FLAG_7          (1 << 7) // 0x80

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
            enabled: flags & (1 << 5) != 0,
            frame: node.anim_frame,
            translation_multiplier,
            attribute: anim.try_field("index")?.try_as_address()?, // TODO: Seg to virt
            data: anim.try_field("values")?.try_as_address()?,     // TODO: Seg to virt
        });

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
                }
            }

            if !iterate_siblings {
                break;
            }
            let next_addr = cur_node.node().next;
            if next_addr == first_addr {
                break;
            }
            cur_node = self.reader.read(next_addr)?;
        }

        Ok(())
    }

    fn process_root(&mut self, node: &GraphNodeRoot) -> Result<(), Error> {
        todo!()
    }

    fn process_ortho_projection(&mut self, node: &GraphNodeOrthoProjection) -> Result<(), Error> {
        todo!()
    }

    fn process_perspective(&mut self, node: &GraphNodePerspective) -> Result<(), Error> {
        todo!()
    }

    fn process_master_list(&mut self, node: &GraphNodeMasterList) -> Result<(), Error> {
        todo!()
    }

    fn process_start(&mut self, node: &GraphNodeStart) -> Result<(), Error> {
        todo!()
    }

    fn process_level_of_detail(&mut self, node: &GraphNodeLevelOfDetail) -> Result<(), Error> {
        todo!()
    }

    fn process_switch_case(&mut self, node: &GraphNodeSwitchCase) -> Result<(), Error> {
        // TODO

        let selected_child = node.fn_node.node.children;
        self.process_node_and_siblings(selected_child)?;
        Ok(())
    }

    fn process_camera(&mut self, node: &GraphNodeCamera) -> Result<(), Error> {
        todo!()
    }

    fn process_translation_rotation(
        &mut self,
        node: &GraphNodeTranslationRotation,
    ) -> Result<(), Error> {
        todo!()
    }

    fn process_translation(&mut self, node: &GraphNodeTranslation) -> Result<(), Error> {
        todo!()
    }

    fn process_rotation(&mut self, node: &GraphNodeRotation) -> Result<(), Error> {
        todo!()
    }

    fn process_object(&mut self, node: &GraphNodeObject) -> Result<(), Error> {
        // TODO: if (node->header.gfx.areaIndex == gCurGraphNodeRoot->areaIndex) {
        {
            if !node.throw_matrix.is_null() {
                todo!()
            } else if node.node.flags.contains(GraphRenderFlags::BILLBOARD) {
                todo!()
            } else {
                // TODO: Calculate matrix
            }

            // TODO: Calculate throwMatrix and cameraToObject

            if !node.anim_info.cur_anim.is_null() {
                self.set_animation_globals(&node.anim_info)?;
            }
            // TODO: if (obj_is_in_view(&node->header.gfx, gMatStack[gMatStackIndex])) {
            {
                // TODO: Calculate matrix
                if !node.shared_child.is_null() {
                    // TODO: Set & unset gCurGraphNodeObject
                    // TODO: Set & unset shared_child parent
                    self.process_node_and_siblings(node.shared_child)?;
                }
                self.process_node_and_siblings(node.node.children)?;
            }

            // TODO: Pop matrix
            self.anim = None;
            // TODO: Reset object throw matrix
        }

        Ok(())
    }

    fn process_animated_part(&mut self, node: &GraphNodeAnimatedPart) -> Result<(), Error> {
        let mut rotation = [0.0, 0.0, 0.0];
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
                rotation[0] = anim.next(self.memory)? as f32 / 0x8000 as f32 * PI;
                rotation[1] = anim.next(self.memory)? as f32 / 0x8000 as f32 * PI;
                rotation[2] = anim.next(self.memory)? as f32 / 0x8000 as f32 * PI;
            }
        }

        eprintln!("{:?} {:?}", translation, rotation);
        let transform = Matrixf::rotate_xyz_and_translate(translation, rotation);

        self.mtx_stack.push_mul(transform);
        self.append_display_list(node.display_list, node.node.flags.bits() >> 8);
        self.process_node_and_siblings(node.node.children)?;
        self.mtx_stack.pop();

        Ok(())
    }

    fn process_billboard(&mut self, node: &GraphNodeBillboard) -> Result<(), Error> {
        todo!()
    }

    fn process_display_list(&mut self, node: &GraphNodeDisplayList) -> Result<(), Error> {
        todo!()
    }

    fn process_scale(&mut self, node: &GraphNodeScale) -> Result<(), Error> {
        // TODO

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_shadow(&mut self, node: &GraphNodeShadow) -> Result<(), Error> {
        // TODO

        if let Some(anim) = &mut self.anim {
            if anim.enabled
                && matches!(
                    anim.ty,
                    AnimType::Translation | AnimType::LateralTranslation
                )
            {
                // TODO
                anim.next(self.memory)?;
                anim.attribute += 4;
                anim.next(self.memory)?;
                anim.attribute -= 12;
            }
        }

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_object_parent(&mut self, node: &GraphNodeObjectParent) -> Result<(), Error> {
        todo!()
    }

    fn process_generated(&mut self, node: &GraphNodeGenerated) -> Result<(), Error> {
        todo!()
    }

    fn process_background(&mut self, node: &GraphNodeBackground) -> Result<(), Error> {
        todo!()
    }

    fn process_held_object(&mut self, node: &GraphNodeHeldObject) -> Result<(), Error> {
        todo!()
    }

    fn process_culling_radius(&mut self, node: &GraphNodeCullingRadius) -> Result<(), Error> {
        todo!()
    }
}
