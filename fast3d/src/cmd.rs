//! Rust types representing Fast3D commands.
//!
//! Note: this module is not complete and may have errors.

#![allow(missing_docs)]

use core::fmt;

use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::decode::RawF3DCommand;

/// A decoded Fast3D command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F3DCommand<Ptr> {
    NoOp,
    Unknown(RawF3DCommand<Ptr>),

    // SP commands
    SPMatrix {
        matrix: Ptr,
        mode: MatrixMode,
        op: MatrixOp,
        push: bool,
    },
    SPViewport(Ptr),
    SPLight {
        light: Ptr,
        n: u32,
    },
    SPVertex {
        v: Ptr,
        n: u32,
        v0: u32,
    },
    SPDisplayList(Ptr),
    SPBranchList(Ptr),
    SPOneTriangle {
        v0: u32,
        v1: u32,
        v2: u32,
        flag: u32,
    },
    SPPopMatrix(MatrixMode),
    SPNumLights(u32),
    SPSegment {
        seg: u32,
        base: Ptr,
    },
    SPFogFactor {
        mul: i16,
        offset: i16,
    },
    SPTexture {
        sc: u32,
        tc: u32,
        level: u32,
        tile: u32,
        on: bool,
    },
    SPEndDisplayList,
    SPSetGeometryMode(GeometryModes),
    SPClearGeometryMode(GeometryModes),
    SPPerspNormalize(u16),

    // DP commands
    DPSetAlphaDither(AlphaDither),
    DPSetColorDither(ColorDither),
    DPSetCombineKey(bool),
    DPSetTextureConvert(TextureConvert),
    DPSetTextureFilter(TextureFilter),
    DPSetTextureLUT(TextureLUT),
    DPSetTextureLOD(bool),
    DPSetTextureDetail(TextureDetail),
    DPSetTexturePersp(bool),
    DPSetCycleType(CycleType),
    DPPipelineMode(PipelineMode),
    DPSetAlphaCompare(AlphaCompare),
    DPSetDepthSource(DepthSource),
    DPSetRenderMode(RenderMode),
    DPSetColorImage(Image<Ptr>),
    DPSetDepthImage(Ptr),
    DPSetTextureImage(Image<Ptr>),
    DPSetCombineMode(CombineMode),
    DPSetEnvColor(Rgba32),
    DPSetPrimColor(Rgba32),
    DPSetBlendColor(Rgba32),
    DPSetFogColor(Rgba32),
    DPSetFillColor([FillColor; 2]),
    DPFillRectangle(Rectangle<u32>),
    DPSetTile(TileIndex, TileParams),
    DPLoadTile(TileIndex, TileSize),
    DPLoadBlock(TileIndex, TextureBlock),
    DPSetTileSize(TileIndex, TileSize),
    DPLoadTLUTCmd(TileIndex, u32),
    DPSetOtherMode(Unimplemented),
    DPSetPrimDepth(PrimDepth),
    DPSetScissor(ScissorMode, Rectangle<u16>),
    DPSetConvert(Unimplemented),
    DPSetKeyR(Unimplemented),
    DPSetKeyGB(Unimplemented),
    DPFullSync,
    DPTileSync,
    DPPipeSync,
    DPLoadSync,
    DPTextureRectangleFlip(TextureRectangle),
    DPTextureRectangle(TextureRectangle),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Unimplemented {
    pub w0: u32,
    pub w1: u32,
}

impl fmt::Debug for Unimplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unimplemented {{ w0: {:#010X}, w1: {:#010X} }}",
            self.w0, self.w1
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum MatrixMode {
    Proj = 1,
    ModelView = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum MatrixOp {
    Load = 2,
    Mul = 0,
}

bitflags! {
    pub struct GeometryModes: u32 {
        const ZBUFFER             = 0x00000001;
        const TEXTURE_ENABLE      = 0x00000002;
        const SHADE               = 0x00000004;
        const SHADING_SMOOTH      = 0x00000200;
        const CULL_FRONT          = 0x00001000;
        const CULL_BACK           = 0x00002000;
        const FOG                 = 0x00010000;
        const LIGHTING            = 0x00020000;
        const TEXTURE_GEN         = 0x00040000;
        const TEXTURE_GEN_LINEAR  = 0x00080000;
        const LOD                 = 0x00100000;
        const CLIPPING            = 0x00800000;
    }
}

impl Default for GeometryModes {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum AlphaDither {
    Pattern = 0,
    NotPattern = 1,
    Noise = 2,
    Disable = 3,
}

impl Default for AlphaDither {
    fn default() -> Self {
        Self::Disable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ColorDither {
    MagicSq = 0,
    Bayer = 1,
    Noise = 2,
    Disable = 3,
}

impl Default for ColorDither {
    fn default() -> Self {
        Self::Disable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureConvert {
    Conv = 0,
    FiltConv = 5,
    Filt = 6,
}

impl Default for TextureConvert {
    fn default() -> Self {
        Self::Conv
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureFilter {
    Point = 0,
    Average = 3,
    Bilerp = 2,
}

impl Default for TextureFilter {
    fn default() -> Self {
        Self::Point
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureLUT {
    None = 0,
    Rgba16 = 2,
    Ia16 = 3,
}

impl Default for TextureLUT {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureDetail {
    Clamp = 0,
    Sharpen = 1,
    Detail = 2,
}

impl Default for TextureDetail {
    fn default() -> Self {
        Self::Clamp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum CycleType {
    OneCycle = 0,
    TwoCycle = 1,
    Copy = 2,
    Fill = 3,
}

impl Default for CycleType {
    fn default() -> Self {
        Self::OneCycle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum PipelineMode {
    OnePrimitive = 1,
    NPrimitive = 0,
}

impl Default for PipelineMode {
    fn default() -> Self {
        Self::OnePrimitive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum AlphaCompare {
    None = 0,
    Threshold = 1,
    Dither = 3,
}

impl Default for AlphaCompare {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum DepthSource {
    Pixel = 0,
    Prim = 1,
}

impl Default for DepthSource {
    fn default() -> Self {
        Self::Pixel
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderMode {
    pub flags: RenderModeFlags,
    pub cvg_dst: CvgDst,
    pub z_mode: ZMode,
    pub blend_cycle1: BlendMode,
    pub blend_cycle2: BlendMode,
}

macro_rules! defn_render_mode {
    ($name:ident, $value:literal) => {
        #[allow(non_snake_case)]
        pub fn $name() -> Self {
            let mut mode = Self::try_from($value).unwrap();
            mode.blend_cycle2 = mode.blend_cycle1;
            mode
        }
    };
}

impl RenderMode {
    pub const NO_OP: Self = Self {
        flags: RenderModeFlags::empty(),
        cvg_dst: CvgDst::Clamp,
        z_mode: ZMode::Opaque,
        blend_cycle1: BlendMode {
            color1: BlendColor::Input,
            alpha1: BlendAlpha1::Input,
            color2: BlendColor::Input,
            alpha2: BlendAlpha2::OneMinusAlpha,
        },
        blend_cycle2: BlendMode {
            color1: BlendColor::Input,
            alpha1: BlendAlpha1::Input,
            color2: BlendColor::Input,
            alpha2: BlendAlpha2::OneMinusAlpha,
        },
    };

    defn_render_mode!(RM_AA_ZB_OPA_SURF, 4464760);
    defn_render_mode!(RM_RA_ZB_OPA_SURF, 4464696);
    defn_render_mode!(RM_AA_ZB_XLU_SURF, 4213208);
    defn_render_mode!(RM_AA_ZB_OPA_DECAL, 4468056);
    defn_render_mode!(RM_RA_ZB_OPA_DECAL, 4467992);
    defn_render_mode!(RM_AA_ZB_XLU_DECAL, 4214232);
    defn_render_mode!(RM_AA_ZB_OPA_INTER, 4465784);
    defn_render_mode!(RM_AA_ZB_XLU_INTER, 4212184);
    defn_render_mode!(RM_AA_ZB_XLU_LINE, 4225112);
    defn_render_mode!(RM_AA_ZB_DEC_LINE, 4226904);
    defn_render_mode!(RM_AA_ZB_TEX_EDGE, 4468856);
    defn_render_mode!(RM_AA_ZB_TEX_INTER, 4469880);
    defn_render_mode!(RM_AA_ZB_SUB_SURF, 4465272);
    defn_render_mode!(RM_AA_ZB_PCL_SURF, 4194427);
    defn_render_mode!(RM_AA_ZB_OPA_TERR, 4202616);
    defn_render_mode!(RM_AA_ZB_TEX_TERR, 4206712);
    defn_render_mode!(RM_AA_ZB_SUB_TERR, 4203128);
    defn_render_mode!(RM_AA_OPA_SURF, 4464712);
    defn_render_mode!(RM_RA_OPA_SURF, 4464648);
    defn_render_mode!(RM_AA_XLU_SURF, 4211144);
    defn_render_mode!(RM_AA_XLU_LINE, 4223048);
    defn_render_mode!(RM_AA_DEC_LINE, 4223560);
    defn_render_mode!(RM_AA_TEX_EDGE, 4468808);
    defn_render_mode!(RM_AA_SUB_SURF, 4465224);
    defn_render_mode!(RM_AA_PCL_SURF, 4194379);
    defn_render_mode!(RM_AA_OPA_TERR, 4202568);
    defn_render_mode!(RM_AA_TEX_TERR, 4206664);
    defn_render_mode!(RM_AA_SUB_TERR, 4203080);
    defn_render_mode!(RM_ZB_OPA_SURF, 4465200);
    defn_render_mode!(RM_ZB_XLU_SURF, 4213328);
    defn_render_mode!(RM_ZB_OPA_DECAL, 4468240);
    defn_render_mode!(RM_ZB_XLU_DECAL, 4214352);
    defn_render_mode!(RM_ZB_CLD_SURF, 4213584);
    defn_render_mode!(RM_ZB_OVL_SURF, 4214608);
    defn_render_mode!(RM_ZB_PCL_SURF, 201851443);
    defn_render_mode!(RM_OPA_SURF, 201867264);
    defn_render_mode!(RM_XLU_SURF, 4211264);
    defn_render_mode!(RM_TEX_EDGE, 201879560);
    defn_render_mode!(RM_CLD_SURF, 4211520);
    defn_render_mode!(RM_PCL_SURF, 201867779);
    defn_render_mode!(RM_ADD, 71844672);
    defn_render_mode!(RM_NOOP, 0);
    defn_render_mode!(RM_VISCVG, 209993792);
    defn_render_mode!(RM_OPA_CI, 201850880);
    defn_render_mode!(RM_CUSTOM_AA_ZB_XLU_SURF, 4213240);
}

impl Default for RenderMode {
    fn default() -> Self {
        Self::NO_OP
    }
}

impl TryFrom<u32> for RenderMode {
    type Error = ();

    fn try_from(w1: u32) -> Result<Self, Self::Error> {
        Ok(Self {
            flags: RenderModeFlags::from_bits_truncate(w1 as u16),
            cvg_dst: (((w1 >> 8) & 0x3) as u8).try_into().map_err(|_| {})?,
            z_mode: (((w1 >> 10) & 0x3) as u8).try_into().map_err(|_| {})?,
            blend_cycle1: BlendMode {
                color1: (((w1 >> 30) & 0x3) as u8).try_into().map_err(|_| {})?,
                alpha1: (((w1 >> 26) & 0x3) as u8).try_into().map_err(|_| {})?,
                color2: (((w1 >> 22) & 0x3) as u8).try_into().map_err(|_| {})?,
                alpha2: (((w1 >> 18) & 0x3) as u8).try_into().map_err(|_| {})?,
            },
            blend_cycle2: BlendMode {
                color1: (((w1 >> 28) & 0x3) as u8).try_into().map_err(|_| {})?,
                alpha1: (((w1 >> 24) & 0x3) as u8).try_into().map_err(|_| {})?,
                color2: (((w1 >> 20) & 0x3) as u8).try_into().map_err(|_| {})?,
                alpha2: (((w1 >> 16) & 0x3) as u8).try_into().map_err(|_| {})?,
            },
        })
    }
}

bitflags! {
    pub struct RenderModeFlags: u16 {
        const ANTI_ALIASING = 0x0008;
        const Z_COMPARE     = 0x0010;
        const Z_UPDATE      = 0x0020;
        const IMAGE_READ    = 0x0040;
        const CLEAR_ON_CVG  = 0x0080;
        const CVG_X_ALPHA   = 0x1000;
        const ALPHA_CVG_SEL = 0x2000;
        const FORCE_BLEND   = 0x4000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum CvgDst {
    Clamp = 0,
    Wrap = 1,
    Full = 2,
    Save = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ZMode {
    Opaque = 0,
    Interpenetrating = 1,
    Translucent = 2,
    Decal = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlendMode {
    pub color1: BlendColor,
    pub alpha1: BlendAlpha1,
    pub color2: BlendColor,
    pub alpha2: BlendAlpha2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum BlendColor {
    Input = 0,
    Memory = 1,
    Blend = 2,
    Fog = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum BlendAlpha1 {
    Input = 0,
    Fog = 1,
    Shade = 2,
    Zero = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum BlendAlpha2 {
    OneMinusAlpha = 0,
    Memory = 1,
    One = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Image<Ptr> {
    pub fmt: ImageFormat,
    pub size: ComponentSize,
    pub width: u32,
    pub img: Ptr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ImageFormat {
    Rgba = 0,
    Yuv = 1,
    Ci = 2,
    Ia = 3,
    I = 4,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Rgba
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ComponentSize {
    Bits4 = 0,
    Bits8 = 1,
    Bits16 = 2,
    Bits32 = 3,
    DD = 5,
}

impl Default for ComponentSize {
    fn default() -> Self {
        Self::Bits4
    }
}

impl ComponentSize {
    pub fn num_bits(self) -> u32 {
        match self {
            ComponentSize::Bits4 => 4,
            ComponentSize::Bits8 => 8,
            ComponentSize::Bits16 => 16,
            ComponentSize::Bits32 => 32,
            ComponentSize::DD => unimplemented!("DD"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CombineMode {
    pub color1: ColorCombineMode,
    pub alpha1: ColorCombineMode,
    pub color2: ColorCombineMode,
    pub alpha2: ColorCombineMode,
}

impl CombineMode {
    pub fn one_cycle(color: ColorCombineMode, alpha: ColorCombineMode) -> Self {
        Self {
            color1: color,
            alpha1: alpha,
            color2: color,
            alpha2: alpha,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorCombineMode {
    /// [A, B, C, D]  ->  (A - B) * C + D
    pub args: [ColorCombineComponent; 4],
}

impl From<[u8; 4]> for ColorCombineMode {
    fn from(v: [u8; 4]) -> Self {
        Self {
            args: [
                ColorCombineComponent::from_u8(v[0]),
                ColorCombineComponent::from_u8(v[1]),
                ColorCombineComponent::from_u8(v[2]),
                ColorCombineComponent::from_u8(v[3]),
            ],
        }
    }
}

impl From<ColorCombineComponent> for ColorCombineMode {
    fn from(v: ColorCombineComponent) -> Self {
        Self {
            args: [
                ColorCombineComponent::Zero,
                ColorCombineComponent::Zero,
                ColorCombineComponent::Zero,
                v,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ColorCombineComponent {
    CombinedOrPrimLodFraction = 0,
    Texel0 = 1,
    Texel1 = 2,
    Prim = 3,
    Shade = 4,
    Env = 5,
    CenterOrScaleOrOne = 6,
    CombinedAlphaOrNoiseOrK4OrZero = 7,
    Texel0Alpha = 8,
    Texel1Alpha = 9,
    PrimAlpha = 10,
    ShadeAlpha = 11,
    EnvAlpha = 12,
    LodFraction = 13,
    PrimLodFraction = 14,
    K5 = 15,
    Zero = 31,
}

impl Default for ColorCombineComponent {
    fn default() -> Self {
        Self::Zero
    }
}

impl ColorCombineComponent {
    fn from_u8(v: u8) -> Self {
        v.try_into().unwrap_or_else(|_| {
            eprintln!("  color comp: {}", v);
            Self::Zero
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgba32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba32 {
    pub fn rgb(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }

    pub fn from_rgb_a([r, g, b]: [u8; 3], a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Either rgba5551 or zdz (z = 14 bits, dz = 2 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FillColor(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rectangle<T> {
    pub ulx: T,
    pub uly: T,
    pub lrx: T,
    pub lry: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileParams {
    pub fmt: ImageFormat,
    pub size: ComponentSize,
    pub line: u32,
    pub tmem: u32,
    pub palette: u32,
    pub cmt: F3DWrapMode,
    pub maskt: u32,
    pub shiftt: u32,
    pub cms: F3DWrapMode,
    pub masks: u32,
    pub shifts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct F3DWrapMode {
    pub mirror: bool,
    pub clamp: bool,
}

impl From<u8> for F3DWrapMode {
    fn from(v: u8) -> Self {
        Self {
            mirror: v & 0x1 != 0,
            clamp: v & 0x2 != 0,
        }
    }
}

impl From<F3DWrapMode> for u8 {
    fn from(m: F3DWrapMode) -> Self {
        let mut v = 0;
        if m.mirror {
            v |= 0x1;
        }
        if m.clamp {
            v |= 0x2;
        }
        v
    }
}

impl F3DWrapMode {
    pub const WRAP: Self = Self {
        mirror: false,
        clamp: false,
    };
    pub const MIRROR: Self = Self {
        mirror: true,
        clamp: false,
    };
    pub const CLAMP: Self = Self {
        mirror: false,
        clamp: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureBlock {
    pub uls: u32,
    pub ult: u32,
    pub lrs: u32,
    pub dxt: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileSize {
    pub uls: u32,
    pub ult: u32,
    pub lrs: u32,
    pub lrt: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileIndex(pub u8);

impl TileIndex {
    pub const LOAD: TileIndex = TileIndex(7);
    pub const RENDER: TileIndex = TileIndex(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrimDepth {
    pub z: u16,
    pub dz: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
#[allow(clippy::enum_variant_names)]
pub enum ScissorMode {
    NonInterlace = 0,
    OddInterlace = 3,
    EvenInterlace = 2,
}

impl Default for ScissorMode {
    fn default() -> Self {
        Self::NonInterlace
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureRectangle {
    pub rect: Rectangle<u32>,
    pub tile: TileIndex,
    pub s: u16,
    pub t: u16,
    pub dsdx: u16,
    pub dtdy: u16,
}
