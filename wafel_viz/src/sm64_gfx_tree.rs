use bitflags::bitflags;
use fast3d::util::Angle;
use wafel_api::{Address, IntType};
use wafel_data_access::{DataError, DataReadable, DataReader, MemoryLayout, Reader};
use wafel_memory::MemoryRead;

#[derive(Debug, Clone)]
pub enum GfxTreeNode {
    Root(GraphNodeRoot),
    OrthoProjection(GraphNodeOrthoProjection),
    Perspective(GraphNodePerspective),
    MasterList(GraphNodeMasterList),
    Start(GraphNodeStart),
    LevelOfDetail(GraphNodeLevelOfDetail),
    SwitchCase(GraphNodeSwitchCase),
    Camera(GraphNodeCamera),
    TranslationRotation(GraphNodeTranslationRotation),
    Translation(GraphNodeTranslation),
    Rotation(GraphNodeRotation),
    Object(GraphNodeObject),
    AnimatedPart(GraphNodeAnimatedPart),
    Billboard(GraphNodeBillboard),
    DisplayList(GraphNodeDisplayList),
    Scale(GraphNodeScale),
    Shadow(GraphNodeShadow),
    ObjectParent(GraphNodeObjectParent),
    Generated(GraphNodeGenerated),
    Background(GraphNodeBackground),
    HeldObject(GraphNodeHeldObject),
    CullingRadius(GraphNodeCullingRadius),
}

impl GfxTreeNode {
    pub fn node(&self) -> &GraphNode {
        match self {
            GfxTreeNode::Root(n) => &n.node,
            GfxTreeNode::OrthoProjection(n) => &n.node,
            GfxTreeNode::Perspective(n) => &n.fn_node.node,
            GfxTreeNode::MasterList(n) => &n.node,
            GfxTreeNode::Start(n) => &n.node,
            GfxTreeNode::LevelOfDetail(n) => &n.node,
            GfxTreeNode::SwitchCase(n) => &n.fn_node.node,
            GfxTreeNode::Camera(n) => &n.fn_node.node,
            GfxTreeNode::TranslationRotation(n) => &n.node,
            GfxTreeNode::Translation(n) => &n.node,
            GfxTreeNode::Rotation(n) => &n.node,
            GfxTreeNode::Object(n) => &n.node,
            GfxTreeNode::AnimatedPart(n) => &n.node,
            GfxTreeNode::Billboard(n) => &n.node,
            GfxTreeNode::DisplayList(n) => &n.node,
            GfxTreeNode::Scale(n) => &n.node,
            GfxTreeNode::Shadow(n) => &n.node,
            GfxTreeNode::ObjectParent(n) => &n.node,
            GfxTreeNode::Generated(n) => &n.fn_node.node,
            GfxTreeNode::Background(n) => &n.fn_node.node,
            GfxTreeNode::HeldObject(n) => &n.fn_node.node,
            GfxTreeNode::CullingRadius(n) => &n.node,
        }
    }

    pub fn fn_node(&self) -> Option<&FnGraphNode> {
        match self {
            GfxTreeNode::Perspective(n) => Some(&n.fn_node),
            GfxTreeNode::SwitchCase(n) => Some(&n.fn_node),
            GfxTreeNode::Camera(n) => Some(&n.fn_node),
            GfxTreeNode::Generated(n) => Some(&n.fn_node),
            GfxTreeNode::Background(n) => Some(&n.fn_node),
            GfxTreeNode::HeldObject(n) => Some(&n.fn_node),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNode")]
pub struct GraphNode {
    #[field_name("type")]
    pub ty: i16,
    pub flags: i16,
    pub prev: Address,
    pub next: Address,
    pub parent: Address,
    pub children: Address,
}

impl GraphNode {
    pub fn flags(&self) -> GraphRenderFlags {
        unsafe { GraphRenderFlags::from_bits_unchecked(self.flags) }
    }
}

bitflags! {
    #[derive(DataReadable)]
    #[struct_name("GraphRenderFlags")]
    pub struct GraphRenderFlags: i16 {
        const ACTIVE         = 1 << 0;
        const CHILDREN_FIRST = 1 << 1;
        const BILLBOARD      = 1 << 2;
        const Z_BUFFER       = 1 << 3;
        const INVISIBLE      = 1 << 4;
        const HAS_ANIMATION  = 1 << 5;
    }
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("FnGraphNode")]
pub struct FnGraphNode {
    pub node: GraphNode,
    pub func: Address,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeRoot")]
pub struct GraphNodeRoot {
    pub node: GraphNode,
    pub area_index: u8,
    pub unk_15: i8,
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    pub num_views: i16,
    pub views: Address,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeOrthoProjection")]
pub struct GraphNodeOrthoProjection {
    pub node: GraphNode,
    pub scale: f32,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodePerspective")]
pub struct GraphNodePerspective {
    pub fn_node: FnGraphNode,
    pub fov: f32,
    pub near: i16,
    pub far: i16,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeMasterList")]
pub struct GraphNodeMasterList {
    pub node: GraphNode,
    pub list_heads: [Address; 8],
    pub list_tails: [Address; 8],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeStart")]
pub struct GraphNodeStart {
    pub node: GraphNode,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeLevelOfDetail")]
pub struct GraphNodeLevelOfDetail {
    pub node: GraphNode,
    pub min_distance: i16,
    pub max_distance: i16,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeSwitchCase")]
pub struct GraphNodeSwitchCase {
    pub fn_node: FnGraphNode,
    pub num_cases: i16,
    pub selected_case: i16,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeCamera")]
pub struct GraphNodeCamera {
    pub fn_node: FnGraphNode,
    #[field_name("config")]
    pub camera: Address,
    pub pos: [f32; 3],
    pub focus: [f32; 3],
    pub matrix_ptr: Address,
    pub roll: Angle,
    pub roll_screen: Angle,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeTranslationRotation")]
pub struct GraphNodeTranslationRotation {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
    pub rotation: [Angle; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeTranslation")]
pub struct GraphNodeTranslation {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeRotation")]
pub struct GraphNodeRotation {
    pub node: GraphNode,
    pub display_list: Address,
    pub rotation: [Angle; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("AnimInfo")]
pub struct AnimInfo {
    #[field_name("animID")]
    pub anim_id: i16,
    pub anim_y_trans: i16,
    pub cur_anim: Address,
    pub anim_frame: i16,
    pub anim_timer: u16,
    pub anim_frame_accel_assist: i32,
    pub anim_accel: i32,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeObject")]
pub struct GraphNodeObject {
    pub node: GraphNode,
    pub shared_child: Address,
    pub area_index: i8,
    pub active_area_index: i8,
    pub angle: [Angle; 3],
    pub pos: [f32; 3],
    pub scale: [f32; 3],
    pub anim_info: AnimInfo,
    #[field_name("unk4C")]
    pub unk_4c: Address,
    pub throw_matrix: Address,
    pub camera_to_object: [f32; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeAnimatedPart")]
pub struct GraphNodeAnimatedPart {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeBillboard")]
pub struct GraphNodeBillboard {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeDisplayList")]
pub struct GraphNodeDisplayList {
    pub node: GraphNode,
    pub display_list: Address,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeScale")]
pub struct GraphNodeScale {
    pub node: GraphNode,
    pub display_list: Address,
    pub scale: f32,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeShadow")]
pub struct GraphNodeShadow {
    pub node: GraphNode,
    pub shadow_scale: i16,
    pub shadow_solidity: u8,
    pub shadow_type: u8,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeObjectParent")]
pub struct GraphNodeObjectParent {
    pub node: GraphNode,
    pub shared_child: Address,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeGenerated")]
pub struct GraphNodeGenerated {
    pub fn_node: FnGraphNode,
    pub parameter: u32,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeBackground")]
pub struct GraphNodeBackground {
    pub fn_node: FnGraphNode,
    pub background: i32,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeHeldObject")]
pub struct GraphNodeHeldObject {
    pub fn_node: FnGraphNode,
    pub player_index: i32,
    pub obj_node: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("GraphNodeCullingRadius")]
pub struct GraphNodeCullingRadius {
    pub node: GraphNode,
    pub culling_radius: i16,
}

#[derive(Debug, Clone)]
pub struct GfxTreeNodeReader {
    root_reader: Reader<GraphNodeRoot>,
    ortho_projection_reader: Reader<GraphNodeOrthoProjection>,
    perspective_reader: Reader<GraphNodePerspective>,
    master_list_reader: Reader<GraphNodeMasterList>,
    start_reader: Reader<GraphNodeStart>,
    level_of_detail_reader: Reader<GraphNodeLevelOfDetail>,
    switch_case_reader: Reader<GraphNodeSwitchCase>,
    camera_reader: Reader<GraphNodeCamera>,
    translation_rotation_reader: Reader<GraphNodeTranslationRotation>,
    translation_reader: Reader<GraphNodeTranslation>,
    rotation_reader: Reader<GraphNodeRotation>,
    object_reader: Reader<GraphNodeObject>,
    animated_part_reader: Reader<GraphNodeAnimatedPart>,
    billboard_reader: Reader<GraphNodeBillboard>,
    display_list_reader: Reader<GraphNodeDisplayList>,
    scale_reader: Reader<GraphNodeScale>,
    shadow_reader: Reader<GraphNodeShadow>,
    object_parent_reader: Reader<GraphNodeObjectParent>,
    generated_reader: Reader<GraphNodeGenerated>,
    background_reader: Reader<GraphNodeBackground>,
    held_object_reader: Reader<GraphNodeHeldObject>,
    culling_radius_reader: Reader<GraphNodeCullingRadius>,
}

impl GfxTreeNodeReader {
    pub fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<GfxTreeNode, DataError> {
        let type_id = memory.read_int(addr, IntType::S16)? as i16;
        #[rustfmt::skip]
        match type_id {
            0x001 => self.root_reader.read(memory, addr).map(GfxTreeNode::Root),
            0x002 => self.ortho_projection_reader.read(memory, addr).map(GfxTreeNode::OrthoProjection),
            0x103 => self.perspective_reader.read(memory, addr).map(GfxTreeNode::Perspective),
            0x004 => self.master_list_reader.read(memory, addr).map(GfxTreeNode::MasterList),
            0x00A => self.start_reader.read(memory, addr).map(GfxTreeNode::Start),
            0x00B => self.level_of_detail_reader.read(memory, addr).map(GfxTreeNode::LevelOfDetail),
            0x10C => self.switch_case_reader.read(memory, addr).map(GfxTreeNode::SwitchCase),
            0x114 => self.camera_reader.read(memory, addr).map(GfxTreeNode::Camera),
            0x015 => self.translation_rotation_reader.read(memory, addr).map(GfxTreeNode::TranslationRotation),
            0x016 => self.translation_reader.read(memory, addr).map(GfxTreeNode::Translation),
            0x017 => self.rotation_reader.read(memory, addr).map(GfxTreeNode::Rotation),
            0x018 => self.object_reader.read(memory, addr).map(GfxTreeNode::Object),
            0x019 => self.animated_part_reader.read(memory, addr).map(GfxTreeNode::AnimatedPart),
            0x01A => self.billboard_reader.read(memory, addr).map(GfxTreeNode::Billboard),
            0x01B => self.display_list_reader.read(memory, addr).map(GfxTreeNode::DisplayList),
            0x01C => self.scale_reader.read(memory, addr).map(GfxTreeNode::Scale),
            0x028 => self.shadow_reader.read(memory, addr).map(GfxTreeNode::Shadow),
            0x029 => self.object_parent_reader.read(memory, addr).map(GfxTreeNode::ObjectParent),
            0x12A => self.generated_reader.read(memory, addr).map(GfxTreeNode::Generated),
            0x12C => self.background_reader.read(memory, addr).map(GfxTreeNode::Background),
            0x12E => self.held_object_reader.read(memory, addr).map(GfxTreeNode::HeldObject),
            0x02F => self.culling_radius_reader.read(memory, addr).map(GfxTreeNode::CullingRadius),
            _ => unimplemented!("gfx node id: {:#X}", type_id) // TODO: Error handling
        }
    }
}

impl DataReader for GfxTreeNodeReader {
    type Value = GfxTreeNode;

    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Self::Value, DataError> {
        self.read(memory, addr)
    }
}

impl DataReadable for GfxTreeNode {
    type Reader = GfxTreeNodeReader;

    fn reader(layout: &impl MemoryLayout) -> Result<Self::Reader, DataError> {
        Ok(GfxTreeNodeReader {
            root_reader: GraphNodeRoot::reader(layout)?,
            ortho_projection_reader: GraphNodeOrthoProjection::reader(layout)?,
            perspective_reader: GraphNodePerspective::reader(layout)?,
            master_list_reader: GraphNodeMasterList::reader(layout)?,
            start_reader: GraphNodeStart::reader(layout)?,
            level_of_detail_reader: GraphNodeLevelOfDetail::reader(layout)?,
            switch_case_reader: GraphNodeSwitchCase::reader(layout)?,
            camera_reader: GraphNodeCamera::reader(layout)?,
            translation_rotation_reader: GraphNodeTranslationRotation::reader(layout)?,
            translation_reader: GraphNodeTranslation::reader(layout)?,
            rotation_reader: GraphNodeRotation::reader(layout)?,
            object_reader: GraphNodeObject::reader(layout)?,
            animated_part_reader: GraphNodeAnimatedPart::reader(layout)?,
            billboard_reader: GraphNodeBillboard::reader(layout)?,
            display_list_reader: GraphNodeDisplayList::reader(layout)?,
            scale_reader: GraphNodeScale::reader(layout)?,
            shadow_reader: GraphNodeShadow::reader(layout)?,
            object_parent_reader: GraphNodeObjectParent::reader(layout)?,
            generated_reader: GraphNodeGenerated::reader(layout)?,
            background_reader: GraphNodeBackground::reader(layout)?,
            held_object_reader: GraphNodeHeldObject::reader(layout)?,
            culling_radius_reader: GraphNodeCullingRadius::reader(layout)?,
        })
    }
}
