use std::{mem, num::Wrapping, process};

use bytemuck::{cast_slice, cast_slice_mut};
use fast3d::{
    cmd::{F3DCommand::*, *},
    interpret::{interpret_f3d_display_list, F3DMemory, F3DRenderData},
    util::{coss, cross, normalize3, sins, Angle, MatrixStack, Matrixf},
};
use wafel_data_access::{DataReadable, MemoryLayout, Reader};
use wafel_data_type::{Address, IntType, Namespace, TypeName, Value};
use wafel_memory::MemoryRead;
use wafel_sm64::gfx::*;

use crate::{
    error::VizError,
    f3d_builder::{F3DBuilder, Pointer, Segmented},
    skybox::skybox_main,
    Camera, InGameRenderMode, LookAtCamera, ObjectCull, OrthoCamera, SurfaceMode, VizConfig,
};

const DEBUG_PRINT: bool = false;
const DEBUG_ONE_FRAME: bool = false;
const DEBUG_CALC_TRANSFORMS: bool = false;
const CHECK_CALC_TRANSFORMS: bool = false;
const ASSERT_CALC_TRANSFORMS: bool = false;
const PRINT_DISPLAY_LISTS: bool = false;

pub fn sm64_gfx_render(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<(F3DRenderData, GfxRenderOutput), VizError> {
    if config.in_game_render_mode == InGameRenderMode::Disabled {
        return Ok((
            F3DRenderData::new(config.screen_top_left, config.screen_size),
            approx_render_output(layout, memory, config)?,
        ));
    }

    let input_dl_addr = layout.global_path("gGfxPool?")?.read(memory)?;
    if input_dl_addr.is_none() {
        return Ok((
            F3DRenderData::new(config.screen_top_left, config.screen_size),
            approx_render_output(layout, memory, config)?,
        ));
    }
    let input_dl_addr = input_dl_addr.try_as_address()?;

    // TODO: Determine when seg table should be used
    // let mut seg_table: Vec<u32> = vec![0; 32];
    // let seg_table_addr = layout.symbol_address("sSegmentTable")?;
    // memory.read_u32s(seg_table_addr, seg_table.as_mut_slice())?;
    // let seg_table = Some(seg_table);
    let seg_table = None;

    // if DEBUG_PRINT {
    //     println!("\n\n------- FRAME -------");
    //     let cmds = decode_f3d_display_list(RawDlIter {
    //         memory,
    //         addr: input_dl_addr,
    //     });
    //     for cmd in cmds {
    //         println!("  {:?}", cmd?);
    //     }
    //     println!("\n\n");
    // }

    let (f3d_memory, render_output) = match config.in_game_render_mode {
        InGameRenderMode::Rerender => {
            let pause_rendering = layout
                .global_path("gWarpTransition.pauseRendering")?
                .read(memory)?
                .try_as_int()?
                != 0;
            let root_addr = layout.global_path("gCurrentArea?.unk04")?.read(memory)?;

            let mut renderer = NodeRenderer::new(config, layout, memory, input_dl_addr, seg_table)?;
            renderer.render_game(root_addr, pause_rendering)?;

            let mut output = renderer.output;
            output.proj_mtx = Matrixf::from_fixed(&output.proj_mtx.to_fixed());
            output.view_mtx = Matrixf::from_fixed(&output.view_mtx.to_fixed());

            (renderer.builder, output)
        }
        InGameRenderMode::DisplayList => {
            let mut builder = F3DBuilder::new(memory, input_dl_addr, seg_table);
            builder.push_remaining()?;
            (builder, approx_render_output(layout, memory, config)?)
        }
        InGameRenderMode::Disabled => unreachable!(),
    };

    let render_data = interpret_f3d_display_list(
        &f3d_memory,
        config.screen_top_left,
        config.screen_size,
        true,
    )?;

    if DEBUG_ONE_FRAME {
        process::exit(0);
    }
    Ok((render_data, render_output))
}

#[derive(Debug, Clone, PartialEq)]
pub struct GfxRenderOutput {
    pub proj_mtx: Matrixf,
    pub view_mtx: Matrixf,
    pub used_camera: Option<Camera>,
}

impl Default for GfxRenderOutput {
    fn default() -> Self {
        Self {
            proj_mtx: Matrixf::identity(),
            view_mtx: Matrixf::identity(),
            used_camera: None,
        }
    }
}

#[derive(Debug)]
struct NodeRenderer<'m, L, M: MemoryRead> {
    config: &'m VizConfig,
    layout: &'m L,
    memory: &'m M,
    builder: F3DBuilder<'m, M>,
    output: GfxRenderOutput,
    reader: Reader<GfxTreeNode>,
    object_fields_reader: Reader<ObjectFields>,
    mtx_stack: MatrixStack,
    mod_mtx_stack: MatrixStack,
    master_lists: [Vec<MasterListEdit>; 8],
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
        display_list: Pointer,
    },
    Skip(Pointer),
    Insert {
        transform: Vec<i32>,
        display_list: Pointer,
    },
    OptDynamic,
    SkipDynamic,
}

#[derive(Debug, Clone)]
struct DisplayListNode {
    transform: Address,
    display_list: Address,
    next: Address,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AnimState {
    ty: AnimType,
    enabled: bool,
    frame: i16,
    translation_multiplier: f32,
    attribute: Address,
    data: Address,
}

impl AnimState {
    fn index(&mut self, memory: &impl MemoryRead) -> Result<i32, VizError> {
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

    fn next(&mut self, memory: &impl MemoryRead) -> Result<i16, VizError> {
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

#[derive(Debug, Clone, DataReadable)]
#[struct_name("Object")]
struct ObjectFields {
    active_flags: i16,
    o_anim_state: i32,
    collision_data: Address,
    behavior: Address,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("LakituState")]
struct LakituState {
    pos: [f32; 3],
    focus: [f32; 3],
    roll: Angle,
}

impl<'m, L, M> NodeRenderer<'m, L, M>
where
    L: MemoryLayout,
    M: MemoryRead,
{
    fn new(
        config: &'m VizConfig,
        layout: &'m L,
        memory: &'m M,
        input_dl_addr: Address,
        seg_table: Option<Vec<u32>>,
    ) -> Result<Self, VizError> {
        let reader = GfxTreeNode::reader(layout)?;
        Ok(Self {
            config,
            layout,
            memory,
            builder: F3DBuilder::new(memory, input_dl_addr, seg_table),
            output: GfxRenderOutput::default(),
            reader,
            object_fields_reader: ObjectFields::reader(layout)?,
            mtx_stack: MatrixStack::default(),
            mod_mtx_stack: MatrixStack::default(),
            master_lists: Default::default(),
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

    fn edit_master_list(&mut self, layer: i16, edit: MasterListEdit) {
        self.master_lists[layer as usize].push(edit);
    }

    fn is_node_rendered(&self) -> bool {
        self.cur_object_is_in_view != Some(false)
            && self.is_in_lod_range != Some(false)
            && self.is_active != Some(false)
    }

    fn append_display_list(&mut self, layer: i16, display_list: Address) -> Result<(), VizError> {
        let visible = if self.config.surface_mode == SurfaceMode::Visual {
            true
        } else {
            let is_surface = match self.cur_object_addr {
                Some(addr) => {
                    let object = self.object_fields_reader.read(self.memory, addr)?;
                    object.collision_data.is_not_null()
                        || object.behavior == self.layout.symbol_address("bhvStaticObject")?
                }
                None => true,
            };
            !is_surface
        };

        if visible {
            self.append_display_list_unconditional(layer, display_list);
        } else {
            self.skip_display_list(layer, display_list);
        }

        Ok(())
    }

    fn append_display_list_unconditional(&mut self, layer: i16, display_list: Address) {
        let display_list = Pointer::Segmented(self.builder.virt_to_phys(display_list));
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

    fn skip_display_list(&mut self, layer: i16, display_list: Address) {
        let display_list = Pointer::Segmented(self.builder.virt_to_phys(display_list));
        if self.is_node_rendered() {
            self.edit_master_list(layer, MasterListEdit::Skip(display_list));
        }
    }

    fn append_opt_dynamic_list(&mut self, layer: i16) {
        if self.is_node_rendered() {
            self.edit_master_list(layer, MasterListEdit::OptDynamic);
        }
    }

    fn append_dynamic_list(&mut self, layer: i16, display_list: Pointer) {
        if self.is_node_rendered() {
            self.edit_master_list(layer, MasterListEdit::SkipDynamic);
        }
        self.edit_master_list(
            layer,
            MasterListEdit::Insert {
                transform: self.mod_mtx_stack.cur.to_fixed(),
                display_list,
            },
        )
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

    fn set_animation_globals(&mut self, node: &AnimInfo) -> Result<(), VizError> {
        let animation_struct = self.layout.data_layout().data_type(&TypeName {
            namespace: Namespace::Struct,
            name: "Animation".to_string(),
        })?;
        let reader = self.layout.data_type_reader(animation_struct)?;
        let anim = reader.read(self.memory, node.cur_anim)?;

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

        let attribute = Segmented(anim.try_field("index")?.try_as_address()?);
        let data = Segmented(anim.try_field("values")?.try_as_address()?);

        self.anim = Some(AnimState {
            ty,
            enabled: flags & (1 << 5) == 0,
            frame: node.anim_frame,
            translation_multiplier,
            attribute: self.builder.seg_to_virt(attribute),
            data: self.builder.seg_to_virt(data),
        });

        Ok(())
    }

    fn render_game(&mut self, root_addr: Value, pause_rendering: bool) -> Result<(), VizError> {
        if let Value::Address(root_addr) = root_addr {
            // Skip init_rcp and viewport/scissor override
            self.builder
                .push_until(|cmd| matches!(cmd, SPViewport(_)))?;

            if !pause_rendering {
                self.output.used_camera = Some(self.calculated_camera()?);
                self.process_node(root_addr, false)?;

                if !self.config.show_in_game_overlays {
                    return Ok(());
                }
            }
        }

        // Hud, in-game menu etc
        self.builder.push_remaining()?;

        Ok(())
    }

    fn process_node_and_siblings(&mut self, first_addr: Address) -> Result<(), VizError> {
        self.process_node(first_addr, true)
    }

    fn process_node(&mut self, first_addr: Address, siblings: bool) -> Result<(), VizError> {
        if first_addr.is_null() {
            return Ok(());
        }
        let first_node = self.reader.read(self.memory, first_addr)?;

        let mut iterate_siblings = siblings;
        let mut cur_addr = first_addr;
        let mut cur_node = first_node;

        if !cur_node.node().parent.is_null() {
            let parent_type = self.memory.read_int(cur_node.node().parent, IntType::S16)?;
            if parent_type
                == self
                    .layout
                    .data_layout()
                    .constant("GRAPH_NODE_TYPE_SWITCH_CASE")?
                    .value
            {
                iterate_siblings = false;
            }
        }

        loop {
            let flags = cur_node.node().flags();
            let is_active = flags.contains(GraphRenderFlags::ACTIVE);
            let parent_is_active = self.is_active;
            self.is_active = Some(is_active && parent_is_active != Some(false));

            let mut render_node = is_active;

            if self.config.object_cull == ObjectCull::ShowAll && !render_node {
                if let GfxTreeNode::Object(_) = &cur_node {
                    let active_flags = self
                        .object_fields_reader
                        .read(self.memory, cur_addr)?
                        .active_flags;

                    let active_flag_active = self
                        .layout
                        .data_layout()
                        .constant("ACTIVE_FLAG_ACTIVE")?
                        .value as i16;
                    let active_flag_far_away = self
                        .layout
                        .data_layout()
                        .constant("ACTIVE_FLAG_FAR_AWAY")?
                        .value as i16;

                    if (active_flags & active_flag_active) != 0
                        && (active_flags & active_flag_far_away) != 0
                    {
                        render_node = true;
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
            cur_node = self.reader.read(self.memory, next_addr)?;
        }

        Ok(())
    }

    fn process_root(&mut self, node: &GraphNodeRoot) -> Result<(), VizError> {
        // Skip viewport/scissor override
        self.builder
            .push_until(|cmd| matches!(cmd, SPViewport(_)))?;

        self.builder
            .push_expect(|cmd| matches!(cmd, SPViewport(_)))?;
        self.builder
            .push_expect(|cmd| matches!(cmd, SPMatrix { .. }))?;

        self.cur_root = Some(node.clone());
        self.process_node_and_siblings(node.node.children)?;
        self.cur_root = None;
        Ok(())
    }

    fn process_ortho_projection(
        &mut self,
        node: &GraphNodeOrthoProjection,
    ) -> Result<(), VizError> {
        if !node.node.children.is_null() {
            self.builder
                .push_expect(|cmd| matches!(cmd, SPPerspNormalize(_)))?;
            self.builder
                .push_expect(|cmd| matches!(cmd, SPMatrix { .. }))?;
            self.process_node_and_siblings(node.node.children)?;
        }
        Ok(())
    }

    fn process_perspective(&mut self, node: &GraphNodePerspective) -> Result<(), VizError> {
        if !node.fn_node.node.children.is_null() {
            self.builder
                .push_expect(|cmd| matches!(cmd, SPPerspNormalize(_)))?;

            let cmd = self.builder.expect(|cmd| matches!(cmd, SPMatrix { .. }))?;
            if let SPMatrix { matrix, .. } = &cmd {
                let mtx = Matrixf::from_fixed(&self.read_fixed(*matrix)?);
                self.output.proj_mtx = mtx;
            } else {
                unreachable!();
            }

            self.builder.push_cmd(cmd);

            self.cur_perspective = Some(node.clone());
            self.process_node_and_siblings(node.fn_node.node.children)?;
            self.cur_perspective = None;
        }
        Ok(())
    }

    fn read_display_list_node(&mut self, addr: Address) -> Result<DisplayListNode, VizError> {
        let ptr_size = self.memory.pointer_int_type().size();

        Ok(DisplayListNode {
            transform: self.memory.read_addr(addr)?,
            display_list: self.memory.read_addr(addr + ptr_size)?,
            next: self.memory.read_addr(addr + 2 * ptr_size)?,
        })
    }

    fn process_master_list_sub(&mut self, node: &GraphNodeMasterList) -> Result<(), VizError> {
        let z_buffer = node.node.flags().contains(GraphRenderFlags::Z_BUFFER);
        if z_buffer {
            self.builder.push_expect(|cmd| matches!(cmd, DPPipeSync))?;
            self.builder
                .push_expect(|cmd| matches!(cmd, SPSetGeometryMode(_)))?;
        }

        // TODO: Could detect generated display lists for more accuracy

        let mtx_cmd = |mtx: Pointer| SPMatrix {
            matrix: mtx,
            mode: MatrixMode::ModelView,
            op: MatrixOp::Load,
            push: false,
        };

        let mut original_lists: [Vec<(Pointer, Pointer)>; 8] = Default::default();

        if PRINT_DISPLAY_LISTS {
            println!();
            println!("Original:");
        }

        for layer in 0..8 {
            if PRINT_DISPLAY_LISTS {
                println!("  Layer {}:", layer);
            }

            let mut dl_node_addr = node.list_heads[layer as usize];
            if !dl_node_addr.is_null() {
                let render_mode = self.get_render_mode(layer, z_buffer);
                self.builder
                    .expect(|cmd| cmd == DPSetRenderMode(render_mode))?;

                while !dl_node_addr.is_null() {
                    let dl_node = self.read_display_list_node(dl_node_addr)?;

                    let transform_ptr =
                        Pointer::Segmented(self.builder.virt_to_phys(dl_node.transform));
                    self.builder.expect(|cmd| cmd == mtx_cmd(transform_ptr))?;

                    let display_list_ptr =
                        Pointer::Segmented(self.builder.virt_to_phys(dl_node.display_list));
                    self.builder
                        .expect(|cmd| cmd == SPDisplayList(display_list_ptr))?;

                    if PRINT_DISPLAY_LISTS {
                        let dl = dl_node.display_list;
                        println!("    {} ({:?})", dl, self.layout.address_to_symbol(dl).ok());
                    }

                    original_lists[layer as usize].push((transform_ptr, display_list_ptr));

                    dl_node_addr = dl_node.next;
                }
            }
        }

        let pool_start = self
            .layout
            .global_path("gGfxPool.buffer")?
            .address(self.memory)?
            .unwrap();
        let cmd_size = 2 * self.memory.pointer_int_type().size();
        let pool_size =
            self.layout.data_layout().constant("GFX_POOL_SIZE")?.value as usize * cmd_size;
        let pool_end = pool_start + pool_size;
        let is_dynamic = |output: &F3DBuilder<'_, _>, dl: Pointer| {
            if let Pointer::Segmented(segmented) = dl {
                let addr = output.seg_to_virt(segmented);
                addr >= pool_start && addr < pool_end
            } else {
                false
            }
        };

        if PRINT_DISPLAY_LISTS {
            println!();
            println!("Edits:");

            for (layer, edits) in self.master_lists.iter().enumerate() {
                println!("  Layer {}:", layer);
                let name = |ptr: Pointer| match ptr {
                    Pointer::Segmented(seg) => format!(
                        "{} ({:?})",
                        seg.0,
                        self.layout.address_to_symbol(seg.0).ok()
                    ),
                    Pointer::BufferOffset(_) => "buf".to_string(),
                };
                for edit in edits {
                    match edit {
                        MasterListEdit::Copy { display_list, .. } => {
                            println!("    Copy {}", name(*display_list));
                        }
                        MasterListEdit::Skip(dl) => {
                            println!("    Skip {}", name(*dl));
                        }
                        MasterListEdit::Insert { display_list, .. } => {
                            println!("    Skip {}", name(*display_list));
                        }
                        MasterListEdit::OptDynamic => {
                            println!("    OptDynamic");
                        }
                        MasterListEdit::SkipDynamic => {
                            println!("    SkipDynamic");
                        }
                    }
                }
            }
        }

        // TODO: Error handling for mismatched display list structure

        let edits = mem::take(&mut self.master_lists);
        let mut new_lists: [Vec<(Pointer, Pointer)>; 8] = Default::default();

        let mut mod_transform = Matrixf::identity();
        if self.used_camera() != Camera::InGame {
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
                            .filter(|&(_, dl)| dl == *display_list)
                            .ok_or_else(|| VizError::MasterListDiscrepancy {
                                descr: format!("copying {}", display_list),
                            })?;

                        if DEBUG_CALC_TRANSFORMS || self.used_camera() != Camera::InGame {
                            let ptr = self.builder.alloc_u32(cast_slice(transform));
                            new_nodes.push((ptr, dl));
                        } else {
                            new_nodes.push((mtx, dl));
                        }
                    }
                    MasterListEdit::Skip(addr) => {
                        actual_nodes
                            .next()
                            .copied()
                            .filter(|&(_, dl)| dl == *addr)
                            .ok_or_else(|| VizError::MasterListDiscrepancy {
                                descr: format!("skipping {}", addr),
                            })?;
                    }
                    MasterListEdit::Insert {
                        transform,
                        display_list,
                    } => {
                        let ptr = self.builder.alloc_u32(cast_slice(transform));
                        new_nodes.push((ptr, *display_list));
                    }
                    MasterListEdit::OptDynamic => {
                        if let Some(&&(mtx, dl)) = actual_nodes.peek() {
                            if is_dynamic(&self.builder, dl) {
                                if DEBUG_CALC_TRANSFORMS || self.used_camera() != Camera::InGame {
                                    let orig_mtx = Matrixf::from_fixed(&self.read_fixed(mtx)?);
                                    let new_mtx = &mod_transform * &orig_mtx;
                                    let ptr =
                                        self.builder.alloc_u32(cast_slice(&new_mtx.to_fixed()));
                                    new_nodes.push((ptr, dl));
                                } else {
                                    new_nodes.push((mtx, dl));
                                }
                                actual_nodes.next();
                            }
                        }
                    }
                    MasterListEdit::SkipDynamic => {
                        actual_nodes
                            .next()
                            .copied()
                            .filter(|&(_, dl)| is_dynamic(&self.builder, dl))
                            .ok_or_else(|| VizError::MasterListDiscrepancy {
                                descr: "skip dynamic".to_string(),
                            })?;
                    }
                }
            }

            // Most likely from the mario head
            new_nodes.extend(actual_nodes);
        }

        for (layer, list) in new_lists.iter().enumerate() {
            if !list.is_empty() {
                let render_mode = self.get_render_mode(layer as i16, z_buffer);
                self.builder.push_cmd(DPSetRenderMode(render_mode));

                for (mtx, dl) in list.iter().copied() {
                    self.builder.push_cmd(mtx_cmd(mtx));
                    self.builder.push_cmd(SPDisplayList(dl));
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
                            let transform_addr =
                                self.builder.seg_to_virt(old_node.1.segmented().unwrap());
                            eprintln!(
                                "{}: mtx ({:?})\n{:?}\n{:?}",
                                layer,
                                self.symbol_name(transform_addr),
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
            self.builder.push_expect(|cmd| matches!(cmd, DPPipeSync))?;
            self.builder
                .push_expect(|cmd| matches!(cmd, SPClearGeometryMode(_)))?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn symbol_name(&self, addr: Address) -> Option<String> {
        self.layout
            .data_layout()
            .globals
            .iter()
            .find(|global| global.1.address == Some(addr.0 as u64))
            .map(|global| global.0.clone())
    }

    #[allow(dead_code)]
    fn read_fixed(&self, ptr: Pointer) -> Result<Vec<i32>, VizError> {
        let mut data = vec![0; 16];
        self.builder
            .read_u32(cast_slice_mut(data.as_mut_slice()), ptr, 0)?;
        Ok(data)
    }

    fn process_master_list(&mut self, node: &GraphNodeMasterList) -> Result<(), VizError> {
        if !node.node.children.is_null() {
            self.process_node_and_siblings(node.node.children)?;
            self.process_master_list_sub(node)?;
        }
        Ok(())
    }

    fn process_start(&mut self, node: &GraphNodeStart) -> Result<(), VizError> {
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_level_of_detail(&mut self, node: &GraphNodeLevelOfDetail) -> Result<(), VizError> {
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

    fn process_switch_case(&mut self, node: &GraphNodeSwitchCase) -> Result<(), VizError> {
        let mut selected_case = node.selected_case;

        if !node.fn_node.func.is_null() {
            // TODO: selected case not set if rendering a culled object
            // TODO: Model shared between different objects (when not using geo_switch_anim_state)

            if !node.fn_node.func.is_null() {
                let geo_switch_anim_state = self
                    .layout
                    .global_path("geo_switch_anim_state")?
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
                        let fields = self.object_fields_reader.read(self.memory, obj_addr)?;
                        selected_case = fields.o_anim_state as i16;
                    }
                }
            }
        }

        let mut selected_child = node.fn_node.node.children;
        let mut i = 0;

        while !selected_child.is_null() && selected_case > i {
            selected_child = self.reader.read(self.memory, selected_child)?.node().next;
            i += 1;
        }

        self.process_node_and_siblings(selected_child)?;
        Ok(())
    }

    fn process_camera(&mut self, node: &GraphNodeCamera) -> Result<(), VizError> {
        let used_camera = self.used_camera();

        let cmd = self.builder.expect(|cmd| matches!(cmd, SPMatrix { .. }))?;
        match used_camera {
            Camera::InGame => {
                self.builder.push_cmd(cmd);
                if let SPMatrix { matrix, .. } = &cmd {
                    let roll_mtx = Matrixf::from_fixed(&self.read_fixed(*matrix)?);
                    self.output.proj_mtx = &self.output.proj_mtx * &roll_mtx;
                } else {
                    unreachable!();
                }
            }
            Camera::LookAt(LookAtCamera { roll, .. }) => {
                let roll_mtx = Matrixf::rotate_xy(roll);
                let ptr = self.builder.alloc_u32(cast_slice(&roll_mtx.to_fixed()));
                self.builder.push_cmd(SPMatrix {
                    matrix: ptr,
                    mode: MatrixMode::Proj,
                    op: MatrixOp::Mul,
                    push: false,
                });
                self.output.proj_mtx = &self.output.proj_mtx * &roll_mtx;
            }
            Camera::Ortho(OrthoCamera { span_v, .. }) => {
                let aspect = 320.0 / 240.0;
                let span_h = aspect * span_v;
                let span_z = 40_000.0;
                let scale = Matrixf::scale_vec3f([2.0 / span_h, 2.0 / span_v, -2.0 / span_z]);
                let translate = Matrixf::translate([0.0, 0.0, -1.0]);
                let proj_mtx = &translate * &scale;
                let ptr = self.builder.alloc_u32(cast_slice(&proj_mtx.to_fixed()));
                self.builder.push_cmd(SPMatrix {
                    matrix: ptr,
                    mode: MatrixMode::Proj,
                    op: MatrixOp::Load,
                    push: false,
                });
                self.output.proj_mtx = proj_mtx;
            }
        };

        let camera_transform = Matrixf::look_at(node.pos, node.focus, node.roll);
        self.mtx_stack.push_mul(&camera_transform);

        let mod_camera_transform = match used_camera {
            Camera::InGame => camera_transform.clone(),
            Camera::LookAt(LookAtCamera { pos, focus, .. }) => {
                Matrixf::look_at(pos, focus, node.roll)
            }
            Camera::Ortho(OrthoCamera {
                pos,
                forward,
                upward,
                ..
            }) => {
                let forward = normalize3(forward);
                let backward = [-forward[0], -forward[1], -forward[2]];
                let upward = normalize3(upward);
                let rightward = cross(forward, upward);
                let rotate = Matrixf::from_rows_vec3([rightward, upward, backward]);
                let translate = Matrixf::translate([-pos[0], -pos[1], -pos[2]]);
                &rotate * &translate
            }
        };
        self.mod_mtx_stack.push_mul(&mod_camera_transform);

        self.output.view_mtx = mod_camera_transform;

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
    ) -> Result<(), VizError> {
        let translation = [
            node.translation[0] as f32,
            node.translation[1] as f32,
            node.translation[2] as f32,
        ];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, node.rotation);
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_translation(&mut self, node: &GraphNodeTranslation) -> Result<(), VizError> {
        let translation = [
            node.translation[0] as f32,
            node.translation[1] as f32,
            node.translation[2] as f32,
        ];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, Default::default());
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_rotation(&mut self, node: &GraphNodeRotation) -> Result<(), VizError> {
        let translation = [0.0, 0.0, 0.0];
        let mtx = Matrixf::rotate_zxy_and_translate(translation, node.rotation);
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn obj_is_in_view(&mut self, node: &GraphNodeObject) -> Result<bool, VizError> {
        let matrix = self.mtx_stack.cur.clone();

        if node.node.flags().contains(GraphRenderFlags::INVISIBLE) {
            return Ok(false);
        }

        let geo = node.shared_child;

        let fov = self
            .cur_perspective
            .as_ref()
            .ok_or(VizError::InvalidGfxTree {
                descr: "no perspective set",
            })?
            .fov;
        let half_fov = Wrapping(((fov / 2.0 + 1.0) * 32768.0 / 180.0 + 0.5) as i16);

        let h_screen_edge = -matrix.cols[3][2] * sins(half_fov) / coss(half_fov);

        let mut culling_radius = 300;
        if !geo.is_null() {
            if let GfxTreeNode::CullingRadius(node) = self.reader.read(self.memory, geo)? {
                culling_radius = node.culling_radius;
            }
        }

        if matrix.cols[3][2] > -100.0 + culling_radius as f32 {
            return Ok(false);
        }
        if matrix.cols[3][2] < -20000.0 - culling_radius as f32 {
            return Ok(false);
        }

        if matrix.cols[3][0] > h_screen_edge + culling_radius as f32 {
            return Ok(false);
        }
        if matrix.cols[3][0] < -h_screen_edge - culling_radius as f32 {
            return Ok(false);
        }

        Ok(true)
    }

    #[allow(dead_code)]
    fn is_mario(&mut self) -> Result<bool, VizError> {
        let mario_object = self
            .layout
            .global_path("gMarioObject")?
            .read(self.memory)?
            .try_as_address()?;
        Ok(self.cur_object_addr == Some(mario_object))
    }

    fn calc_throw_matrix(&mut self) -> Result<Option<Matrixf>, VizError> {
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

    fn calculated_camera(&self) -> Result<Camera, VizError> {
        match self.used_camera() {
            Camera::InGame => {
                let lakitu_state_addr = self.layout.symbol_address("gLakituState")?;
                let LakituState { pos, focus, roll } =
                    LakituState::reader(self.layout)?.read(self.memory, lakitu_state_addr)?;
                Ok(Camera::LookAt(LookAtCamera { pos, focus, roll }))
            }
            c => Ok(c),
        }
    }

    fn used_camera(&self) -> Camera {
        if let Camera::LookAt(LookAtCamera { pos, focus, .. }) = self.config.camera {
            if pos == focus {
                return Camera::InGame;
            }
        }
        self.config.camera
    }

    fn mod_camera_roll(&self) -> Result<Angle, VizError> {
        Ok(match self.used_camera() {
            Camera::InGame => {
                let camera = self.cur_camera.as_ref().ok_or(VizError::InvalidGfxTree {
                    descr: "no current camra",
                })?;
                camera.roll
            }
            Camera::LookAt(LookAtCamera { roll, .. }) => roll,
            Camera::Ortho(_) => Wrapping(0),
        })
    }

    fn process_object(&mut self, node: &GraphNodeObject) -> Result<(), VizError> {
        let root_area = self.cur_root.as_ref().map(|r| r.area_index);
        if root_area.is_none() || root_area == Some(node.area_index as u8) {
            if let Some(throw_matrix) = self.calc_throw_matrix()? {
                self.mtx_stack.push_mul(&throw_matrix);
                self.mod_mtx_stack.push_mul(&throw_matrix);
            } else if node.node.flags().contains(GraphRenderFlags::BILLBOARD) {
                let mtx = Matrixf::billboard(
                    &self.mtx_stack.cur,
                    node.pos,
                    self.cur_camera
                        .as_ref()
                        .ok_or(VizError::InvalidGfxTree {
                            descr: "no camera set",
                        })?
                        .roll,
                );
                self.mtx_stack.execute(&mtx, MatrixOp::Load, true);

                let mod_mtx =
                    Matrixf::billboard(&self.mod_mtx_stack.cur, node.pos, self.mod_camera_roll()?);
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
                ObjectCull::ShowAll => !node.node.flags().contains(GraphRenderFlags::INVISIBLE),
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

    fn process_animated_part(&mut self, node: &GraphNodeAnimatedPart) -> Result<(), VizError> {
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
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_billboard(&mut self, node: &GraphNodeBillboard) -> Result<(), VizError> {
        let translation = [
            node.translation[0] as f32,
            node.translation[1] as f32,
            node.translation[2] as f32,
        ];

        let mtx = Matrixf::billboard(
            &self.mtx_stack.cur,
            translation,
            self.cur_camera
                .as_ref()
                .ok_or(VizError::InvalidGfxTree {
                    descr: "no current camera",
                })?
                .roll,
        );
        self.mtx_stack.execute(&mtx, MatrixOp::Load, true);

        let mod_mtx = Matrixf::billboard(
            &self.mod_mtx_stack.cur,
            translation,
            self.mod_camera_roll()?,
        );
        self.mod_mtx_stack.execute(&mod_mtx, MatrixOp::Load, true);

        let mut cur_obj = self.cur_object.clone();
        if let Some(node) = self.cur_held_object.as_ref() {
            if let GfxTreeNode::Object(node) = self.reader.read(self.memory, node.obj_node)? {
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
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_display_list(&mut self, node: &GraphNodeDisplayList) -> Result<(), VizError> {
        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_scale(&mut self, node: &GraphNodeScale) -> Result<(), VizError> {
        let mtx = Matrixf::scale_vec3f([node.scale, node.scale, node.scale]);
        self.mtx_stack.push_mul(&mtx);
        self.mod_mtx_stack.push_mul(&mtx);

        if !node.display_list.is_null() {
            self.append_display_list(node.node.flags >> 8, node.display_list)?;
        }
        self.process_node_and_siblings(node.node.children)?;

        self.mod_mtx_stack.pop();
        self.mtx_stack.pop();
        Ok(())
    }

    fn process_shadow(&mut self, node: &GraphNodeShadow) -> Result<(), VizError> {
        if self.cur_camera.is_some() && self.cur_object.is_some() {
            for layer in [4, 5, 6] {
                self.append_opt_dynamic_list(layer);
            }
        }

        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_object_parent(&mut self, node: &GraphNodeObjectParent) -> Result<(), VizError> {
        if !node.shared_child.is_null() {
            self.process_node_and_siblings(node.shared_child)?;
        }
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }

    fn process_generated(&mut self, node: &GraphNodeGenerated) -> Result<(), VizError> {
        self.append_opt_dynamic_list(node.fn_node.node.flags >> 8);
        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn lakitu_state_for_background(&self) -> Result<LookAtCamera, VizError> {
        match self.used_camera() {
            Camera::InGame => {
                let lakitu_state_addr = self.layout.symbol_address("gLakituState")?;
                let LakituState { pos, focus, roll } =
                    LakituState::reader(self.layout)?.read(self.memory, lakitu_state_addr)?;
                Ok(LookAtCamera { pos, focus, roll })
            }
            Camera::LookAt(camera) => Ok(camera),
            Camera::Ortho(camera) => Ok(LookAtCamera {
                pos: camera.pos,
                focus: [
                    camera.pos[0] + camera.forward[0],
                    camera.pos[1] + camera.forward[1],
                    camera.pos[2] + camera.forward[2],
                ],
                roll: Wrapping(0),
            }),
        }
    }

    fn process_background(&mut self, node: &GraphNodeBackground) -> Result<(), VizError> {
        if !node.fn_node.func.is_null() {
            let lakitu_state = self.lakitu_state_for_background()?;
            let display_list = skybox_main(
                &mut self.builder,
                self.layout,
                self.memory,
                node,
                self.config.screen_size,
                &lakitu_state,
            )?;
            self.append_dynamic_list(node.fn_node.node.flags >> 8, display_list);
        } else {
            self.append_opt_dynamic_list(0);
        };

        self.process_node_and_siblings(node.fn_node.node.children)?;
        Ok(())
    }

    fn process_held_object(&mut self, node: &GraphNodeHeldObject) -> Result<(), VizError> {
        if !node.obj_node.is_null() {
            if let GfxTreeNode::Object(obj_node) = self.reader.read(self.memory, node.obj_node)? {
                if !obj_node.shared_child.is_null() {
                    let translation = [
                        node.translation[0] as f32 / 4.0,
                        node.translation[1] as f32 / 4.0,
                        node.translation[2] as f32 / 4.0,
                    ];

                    let translate = Matrixf::translate(translation);

                    let mut throw =
                        self.cur_object_throw_mtx
                            .clone()
                            .ok_or(VizError::InvalidGfxTree {
                                descr: "no current object",
                            })?;
                    throw.cols[3][0] = self.mtx_stack.cur.cols[3][0];
                    throw.cols[3][1] = self.mtx_stack.cur.cols[3][1];
                    throw.cols[3][2] = self.mtx_stack.cur.cols[3][2];

                    let mut mod_throw =
                        self.cur_object_mod_throw_mtx
                            .clone()
                            .ok_or(VizError::InvalidGfxTree {
                                descr: "no current object",
                            })?;
                    mod_throw.cols[3][0] = self.mod_mtx_stack.cur.cols[3][0];
                    mod_throw.cols[3][1] = self.mod_mtx_stack.cur.cols[3][1];
                    mod_throw.cols[3][2] = self.mod_mtx_stack.cur.cols[3][2];

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

    fn process_culling_radius(&mut self, node: &GraphNodeCullingRadius) -> Result<(), VizError> {
        self.process_node_and_siblings(node.node.children)?;
        Ok(())
    }
}

fn approx_render_output(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<GfxRenderOutput, VizError> {
    let camera = if config.in_game_render_mode == InGameRenderMode::DisplayList {
        Camera::InGame
    } else {
        config.camera
    };
    let mut output = GfxRenderOutput {
        proj_mtx: approx_proj_mtx(layout, memory, &camera)?,
        view_mtx: approx_view_mtx(layout, memory, &camera)?,
        used_camera: Some(approx_used_camera(layout, memory, &camera)?),
    };
    output.proj_mtx = Matrixf::from_fixed(&output.proj_mtx.to_fixed());
    output.view_mtx = Matrixf::from_fixed(&output.view_mtx.to_fixed());
    Ok(output)
}

fn approx_proj_mtx(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    camera: &Camera,
) -> Result<Matrixf, VizError> {
    if let Camera::Ortho(OrthoCamera { span_v, .. }) = camera {
        let aspect = 320.0 / 240.0;
        let span_h = aspect * span_v;
        let span_z = 40_000.0;
        let scale = Matrixf::scale_vec3f([2.0 / span_h, 2.0 / span_v, -2.0 / span_z]);
        let translate = Matrixf::translate([0.0, 0.0, -1.0]);
        let proj_mtx = &translate * &scale;
        Ok(proj_mtx)
    } else {
        let fov = layout
            .global_path("sFOVState.fov")?
            .read(memory)?
            .try_as_f32()?;
        let aspect = 320.0 / 240.0;
        let proj_mtx = Matrixf::perspective(fov, aspect, 100.0, 20_000.0);
        Ok(proj_mtx)
    }
}

fn approx_view_mtx(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    camera: &Camera,
) -> Result<Matrixf, VizError> {
    let view_mtx = match *camera {
        Camera::InGame => {
            let pos = layout
                .global_path("gLakituState.pos")?
                .read(memory)?
                .try_as_f32_3()?;
            let focus = layout
                .global_path("gLakituState.focus")?
                .read(memory)?
                .try_as_f32_3()?;
            Matrixf::look_at(pos, focus, Wrapping(0))
        }
        Camera::LookAt(LookAtCamera { pos, focus, .. }) => {
            Matrixf::look_at(pos, focus, Wrapping(0))
        }
        Camera::Ortho(OrthoCamera {
            pos,
            forward,
            upward,
            ..
        }) => {
            let forward = normalize3(forward);
            let backward = [-forward[0], -forward[1], -forward[2]];
            let upward = normalize3(upward);
            let rightward = cross(forward, upward);
            let rotate = Matrixf::from_rows_vec3([rightward, upward, backward]);
            let translate = Matrixf::translate([-pos[0], -pos[1], -pos[2]]);
            &rotate * &translate
        }
    };
    Ok(view_mtx)
}

fn approx_used_camera(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    camera: &Camera,
) -> Result<Camera, VizError> {
    match *camera {
        Camera::InGame => {
            let lakitu_state_addr = layout.symbol_address("gLakituState")?;
            let LakituState { pos, focus, roll } =
                LakituState::reader(layout)?.read(memory, lakitu_state_addr)?;
            Ok(Camera::LookAt(LookAtCamera { pos, focus, roll }))
        }
        c => Ok(c),
    }
}
