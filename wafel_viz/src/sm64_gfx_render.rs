use bytemuck::cast_slice;
use fast3d::{
    decode::{F3DCommand, MatrixMode, MatrixOp, SPCommand},
    interpret::{interpret_f3d_display_list, F3DRenderData},
    util::Matrixf,
};
use wafel_api::{Address, Error, IntType};
use wafel_layout::DataLayout;
use wafel_memory::MemoryRead;

use crate::{
    sm64_gfx_tree::{
        get_gfx_node_reader, GfxNodeReader, GfxTreeNode, GraphNodeAnimatedPart, GraphNodeObject,
        GraphNodeScale, GraphNodeShadow, GraphNodeSwitchCase, GraphRenderFlags,
    },
    sm64_render_mod::{F3DMemoryImpl, Pointer},
};

pub fn test_render(
    memory: &impl MemoryRead,
    layout: &DataLayout,
    addr: Address,
) -> Result<F3DRenderData, Error> {
    let mut renderer = NodeRenderer::new(memory, layout)?;
    renderer.process_node(addr, false)?;
    renderer.submit_master_lists();

    let mut f3d_memory = F3DMemoryImpl::new(memory, Pointer::BufferOffset(0));
    f3d_memory.set_view_transform(Some(Matrixf::look_at(
        [1000.0, 0.0, 0.0],
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
    master_lists: [Vec<DisplayListNode>; 8],
    display_list: Vec<F3DCommand<Pointer>>,
    u32_buffer: Vec<u32>,
}

#[derive(Debug)]
struct DisplayListNode {
    transform: Vec<i32>,
    display_list: Address,
}

impl<'m, M: MemoryRead> NodeRenderer<'m, M> {
    fn new(memory: &'m M, layout: &'m DataLayout) -> Result<Self, Error> {
        let reader = get_gfx_node_reader(memory, layout)?;
        Ok(Self {
            memory,
            layout,
            reader,
            master_lists: Default::default(),
            display_list: Vec::new(),
            u32_buffer: Vec::new(),
        })
    }

    fn append_display_list(&mut self, display_list: Address, layer: i16) {
        if !display_list.is_null() {
            // TODO: Set transform
            self.master_lists[layer as usize].push(DisplayListNode {
                transform: Matrixf::identity().to_fixed(),
                display_list,
            });
        }
    }

    fn submit_master_lists(&mut self) {
        // TODO: z buffer, render modes

        for (i, lists) in self.master_lists.iter().enumerate() {
            for list in lists {
                let offset = self.u32_buffer.len();
                self.u32_buffer.extend(cast_slice(&list.transform));
                self.display_list.push(F3DCommand::Rsp(SPCommand::Matrix {
                    matrix: Pointer::BufferOffset(offset),
                    mode: MatrixMode::ModelView,
                    op: MatrixOp::Load,
                    push: false,
                }));

                self.display_list
                    .push(F3DCommand::Rsp(SPCommand::DisplayList(
                        list.display_list.into(),
                    )));
            }
        }
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
                    eprintln!("{:?}: {:?}", cur_addr, cur_node);
                    match &cur_node {
                        GfxTreeNode::Root(_) => todo!(),
                        GfxTreeNode::OrthoProjection(_) => todo!(),
                        GfxTreeNode::Perspective(_) => todo!(),
                        GfxTreeNode::MasterList(_) => todo!(),
                        GfxTreeNode::Start(_) => todo!(),
                        GfxTreeNode::LevelOfDetail(_) => todo!(),
                        GfxTreeNode::SwitchCase(node) => self.process_switch(node)?,
                        GfxTreeNode::Camera(_) => todo!(),
                        GfxTreeNode::TranslationRotation(_) => todo!(),
                        GfxTreeNode::Translation(_) => todo!(),
                        GfxTreeNode::Rotation(_) => todo!(),
                        GfxTreeNode::Object(node) => self.process_object(node)?,
                        GfxTreeNode::AnimatedPart(node) => self.process_animated_part(node)?,
                        GfxTreeNode::Billboard(_) => todo!(),
                        GfxTreeNode::DisplayList(_) => todo!(),
                        GfxTreeNode::Scale(node) => self.process_scale(node)?,
                        GfxTreeNode::Shadow(node) => self.process_shadow(node)?,
                        GfxTreeNode::ObjectParent(_) => todo!(),
                        GfxTreeNode::Generated(_) => todo!(),
                        GfxTreeNode::Background(_) => todo!(),
                        GfxTreeNode::HeldObject(_) => todo!(),
                        GfxTreeNode::CullingRadius(_) => todo!(),
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

    fn process_switch(&mut self, node: &GraphNodeSwitchCase) -> Result<(), Error> {
        // TODO

        let selected_child = node.fn_node.node.children;
        self.process_node_and_siblings(selected_child);
        Ok(())
    }

    fn process_object(&mut self, node: &GraphNodeObject) -> Result<(), Error> {
        let has_animation = node.node.flags.contains(GraphRenderFlags::HAS_ANIMATION);

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
                // TODO: Set animation globals
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
            // TODO: Reset gCurrAnimType
            // TODO: Reset object throw matrix
        }

        Ok(())
    }

    fn process_animated_part(&mut self, node: &GraphNodeAnimatedPart) -> Result<(), Error> {
        // TODO

        self.append_display_list(node.display_list, node.node.flags.bits() >> 8);
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_scale(&mut self, node: &GraphNodeScale) -> Result<(), Error> {
        // TODO

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_shadow(&mut self, node: &GraphNodeShadow) -> Result<(), Error> {
        // TODO

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }
}
