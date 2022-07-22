use core::fmt;
use std::{collections::HashMap, sync::Arc};

use bitflags::bitflags;
use fast3d::util::Angle;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wafel_api::{Address, Error, IntType, Value};
use wafel_data_access::MemoryLayout;
use wafel_data_type::{DataType, DataTypeRef, Field, Namespace, TypeName, ValueSerializeWrapper};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    #[serde(rename = "type")]
    pub ty: i16,
    pub flags: GraphRenderFlags,
    pub prev: Address,
    pub next: Address,
    pub parent: Address,
    pub children: Address,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct GraphRenderFlags: i16 {
        const ACTIVE         = 1 << 0;
        const CHILDREN_FIRST = 1 << 1;
        const BILLBOARD      = 1 << 2;
        const Z_BUFFER       = 1 << 3;
        const INVISIBLE      = 1 << 4;
        const HAS_ANIMATION  = 1 << 5;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FnGraphNode {
    pub node: GraphNode,
    pub func: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeOrthoProjection {
    pub node: GraphNode,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodePerspective {
    pub fn_node: FnGraphNode,
    pub fov: f32,
    pub near: i16,
    pub far: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeMasterList {
    pub node: GraphNode,
    pub list_heads: [Address; 8],
    pub list_tails: [Address; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeStart {
    pub node: GraphNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeLevelOfDetail {
    pub node: GraphNode,
    pub min_distance: i16,
    pub max_distance: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeSwitchCase {
    pub fn_node: FnGraphNode,
    pub num_cases: i16,
    pub selected_case: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeCamera {
    pub fn_node: FnGraphNode,
    pub camera: Address,
    pub pos: [f32; 3],
    pub focus: [f32; 3],
    pub matrix_ptr: Address,
    pub roll: Angle,
    pub roll_screen: Angle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeTranslationRotation {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
    pub rotation: [Angle; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeTranslation {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeRotation {
    pub node: GraphNode,
    pub display_list: Address,
    pub rotation: [Angle; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimInfo {
    #[serde(rename = "animID")]
    pub anim_id: i16,
    pub anim_y_trans: i16,
    pub cur_anim: Address,
    pub anim_frame: i16,
    pub anim_timer: u16,
    pub anim_frame_accel_assist: i32,
    pub anim_accel: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeObject {
    pub node: GraphNode,
    pub shared_child: Address,
    pub area_index: i8,
    pub active_area_index: i8,
    pub angle: [Angle; 3],
    pub pos: [f32; 3],
    pub scale: [f32; 3],
    pub anim_info: AnimInfo,
    #[serde(rename = "unk4C")]
    pub unk_4c: Address,
    pub throw_matrix: Address,
    pub camera_to_object: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeAnimatedPart {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeBillboard {
    pub node: GraphNode,
    pub display_list: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDisplayList {
    pub node: GraphNode,
    pub display_list: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeScale {
    pub node: GraphNode,
    pub display_list: Address,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeShadow {
    pub node: GraphNode,
    pub shadow_scale: i16,
    pub shadow_solidity: u8,
    pub shadow_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeObjectParent {
    pub node: GraphNode,
    pub shared_child: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeGenerated {
    pub fn_node: FnGraphNode,
    pub parameter: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeBackground {
    pub fn_node: FnGraphNode,
    pub background: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeHeldObject {
    pub fn_node: FnGraphNode,
    pub player_index: i32,
    pub obj_node: Address,
    pub translation: [i16; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeCullingRadius {
    pub node: GraphNode,
    pub culling_radius: i16,
}

pub struct GfxNodeReader<'m>(Box<dyn Fn(Address) -> Result<GfxTreeNode, Error> + 'm>);

impl<'m> fmt::Debug for GfxNodeReader<'m> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GfxNodeReader").finish_non_exhaustive()
    }
}

impl<'m> GfxNodeReader<'m> {
    pub fn read(&self, addr: Address) -> Result<GfxTreeNode, Error> {
        self.0(addr)
    }
}

pub fn get_gfx_node_reader<'m>(
    layout: &'m impl MemoryLayout,
    memory: &'m impl MemoryRead,
) -> Result<GfxNodeReader<'m>, Error> {
    let readers = Arc::new(get_node_readers(layout, memory)?);
    let func = move |addr| {
        let readers = Arc::clone(&readers);
        let type_id = memory.read_int(addr, IntType::S16)? as i16;
        let reader = readers
            .get(&type_id)
            .unwrap_or_else(|| unimplemented!("gfx node type: {:#06X}", type_id));
        reader.read(addr)
    };
    Ok(GfxNodeReader(Box::new(func)))
}

fn get_node_readers<'m>(
    layout: &'m impl MemoryLayout,
    memory: &'m impl MemoryRead,
) -> Result<HashMap<i16, GfxNodeReader<'m>>, Error> {
    Ok([
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_ROOT",
            "GraphNodeRoot",
            GfxTreeNode::Root,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_ORTHO_PROJECTION",
            "GraphNodeOrthoProjection",
            GfxTreeNode::OrthoProjection,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_PERSPECTIVE",
            "GraphNodePerspective",
            GfxTreeNode::Perspective,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_MASTER_LIST",
            "GraphNodeMasterList",
            GfxTreeNode::MasterList,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_START",
            "GraphNodeStart",
            GfxTreeNode::Start,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_LEVEL_OF_DETAIL",
            "GraphNodeLevelOfDetail",
            GfxTreeNode::LevelOfDetail,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_SWITCH_CASE",
            "GraphNodeSwitchCase",
            GfxTreeNode::SwitchCase,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_CAMERA",
            "GraphNodeCamera",
            GfxTreeNode::Camera,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_TRANSLATION_ROTATION",
            "GraphNodeTranslationRotation",
            GfxTreeNode::TranslationRotation,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_TRANSLATION",
            "GraphNodeTranslation",
            GfxTreeNode::Translation,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_ROTATION",
            "GraphNodeRotation",
            GfxTreeNode::Rotation,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_OBJECT",
            "GraphNodeObject",
            GfxTreeNode::Object,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_ANIMATED_PART",
            "GraphNodeAnimatedPart",
            GfxTreeNode::AnimatedPart,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_BILLBOARD",
            "GraphNodeBillboard",
            GfxTreeNode::Billboard,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_DISPLAY_LIST",
            "GraphNodeDisplayList",
            GfxTreeNode::DisplayList,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_SCALE",
            "GraphNodeScale",
            GfxTreeNode::Scale,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_SHADOW",
            "GraphNodeShadow",
            GfxTreeNode::Shadow,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_OBJECT_PARENT",
            "GraphNodeObjectParent",
            GfxTreeNode::ObjectParent,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_GENERATED_LIST",
            "GraphNodeGenerated",
            GfxTreeNode::Generated,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_BACKGROUND",
            "GraphNodeBackground",
            GfxTreeNode::Background,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_HELD_OBJ",
            "GraphNodeHeldObject",
            GfxTreeNode::HeldObject,
        )?,
        get_node_reader_entry(
            layout,
            memory,
            "GRAPH_NODE_TYPE_CULLING_RADIUS",
            "GraphNodeCullingRadius",
            GfxTreeNode::CullingRadius,
        )?,
    ]
    .into_iter()
    .collect())
}

fn get_node_reader_entry<'m, T: DeserializeOwned + 'static>(
    layout: &'m impl MemoryLayout,
    memory: &'m impl MemoryRead,
    id_name: &'static str,
    struct_name: &'static str,
    variant: fn(T) -> GfxTreeNode,
) -> Result<(i16, GfxNodeReader<'m>), Error> {
    let resolve_type =
        |type_name: &TypeName| layout.data_layout().data_type(type_name).ok().cloned();

    let mut data_type = Arc::clone(layout.data_layout().data_type(&TypeName {
        namespace: Namespace::Struct,
        name: struct_name.into(),
    })?);

    // GraphNodeCamera contains a union, so replace it with void *camera
    if struct_name == "GraphNodeCamera" {
        if let DataType::Struct { fields } = data_type.as_ref() {
            let mut new_fields = fields.clone();
            if let Some(config) = new_fields.remove("config") {
                new_fields.insert(
                    "camera".to_string(),
                    Field {
                        offset: config.offset,
                        data_type: DataTypeRef::new(DataType::Pointer {
                            base: DataTypeRef::new(DataType::Void),
                            stride: None,
                        }),
                    },
                );
            }
            data_type = DataTypeRef::new(DataType::Struct { fields: new_fields });
        }
    }

    let func = move |addr| {
        let data_type = Arc::clone(&data_type);
        let reader = layout.data_type_reader(&data_type)?;
        let data = reader.read(memory, addr)?;
        let node = value_to_struct(data);
        Ok(variant(node))
    };

    Ok((
        layout.data_layout().constant(id_name)?.value as i16,
        GfxNodeReader(Box::new(func)),
    ))
}

fn value_to_struct<T: DeserializeOwned + 'static>(data: Value) -> T {
    let json =
        serde_json::to_string(&ValueSerializeWrapper(&data)).expect("failed to serialize value");
    let node: T = serde_json::from_str(&json).expect("failed to deserialize gfx node");
    node
}
