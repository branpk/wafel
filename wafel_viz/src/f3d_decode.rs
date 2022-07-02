#![allow(missing_docs)]

use std::fmt;

use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawF3DCommand<Ptr> {
    pub w0: u32,
    pub w1: u32,
    pub w1_ptr: Ptr,
}

impl<Ptr> fmt::Debug for RawF3DCommand<Ptr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RawF3DCommand {{ w0: {:#010X}, w1: {:#010X} }}",
            self.w0, self.w1
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum F3DCommand<Ptr> {
    NoOp,
    Rsp(SPCommand<Ptr>),
    Rdp(DPCommand<Ptr>),
    Unknown(RawF3DCommand<Ptr>),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SPCommand<Ptr> {
    Matrix {
        matrix: Ptr,
        mode: MatrixMode,
        op: MatrixOp,
        push: bool,
    },
    Viewport(Ptr),
    Light {
        light: Ptr,
        n: u32,
    },
    Vertex {
        v: Ptr,
        n: u32,
        v0: u32,
    },
    DisplayList(Ptr),
    BranchList(Ptr),
    OneTriangle {
        v0: u32,
        v1: u32,
        v2: u32,
        flag: u32,
    },
    PopMatrix(MatrixMode),
    NumLights(u32),
    Segment {
        seg: u32,
        base: Ptr,
    },
    FogFactor {
        mul: i16,
        offset: i16,
    },
    Texture {
        sc: u32,
        tc: u32,
        level: u32,
        tile: u32,
        on: bool,
    },
    EndDisplayList,
    SetGeometryMode(GeometryModes),
    ClearGeometryMode(GeometryModes),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DPCommand<Ptr> {
    SetAlphaDither(AlphaDither),
    SetColorDither(ColorDither),
    SetCombineKey(bool),
    SetTextureConvert(TextureConvert),
    SetTextureFilter(TextureFilter),
    SetTextureLUT(TextureLUT),
    SetTextureLOD(bool),
    SetTextureDetail(TextureDetail),
    SetTexturePersp(bool),
    SetCycleType(CycleType),
    PipelineMode(PipelineMode),
    SetAlphaCompare(AlphaCompare),
    SetDepthSource(DepthSource),
    SetRenderMode(RenderMode),
    PerspNormalize(u16),
    SetColorImage(Image<Ptr>),
    SetDepthImage(Ptr),
    SetTextureImage(Image<Ptr>),
    SetCombineMode(CombineMode),
    SetEnvColor(Rgba32),
    SetPrimColor(Rgba32),
    SetBlendColor(Rgba32),
    SetFogColor(Rgba32),
    SetFillColor([FillColor; 2]),
    FillRectangle(Rectangle<u32>),
    SetTile(TileIndex, TileParams),
    LoadTile(TileIndex, TileSize),
    LoadBlock(TileIndex, TextureBlock),
    SetTileSize(TileIndex, TileSize),
    LoadTLUTCmd(TileIndex, u32),
    SetOtherMode(Unimplemented),
    SetPrimDepth(PrimDepth),
    SetScissor(ScissorMode, Rectangle<u16>),
    SetConvert(Unimplemented),
    SetKeyR(Unimplemented),
    SetKeyGB(Unimplemented),
    FullSync,
    TileSync,
    PipeSync,
    LoadSync,
    TextureRectangleFlip(Unimplemented),
    TextureRectangle(TextureRectangle),
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
}

impl Default for RenderMode {
    fn default() -> Self {
        Self::NO_OP
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
    pub cmt: WrapMode,
    pub maskt: u32,
    pub shiftt: u32,
    pub cms: WrapMode,
    pub masks: u32,
    pub shifts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WrapMode {
    pub mirror: bool,
    pub clamp: bool,
}

impl From<u8> for WrapMode {
    fn from(v: u8) -> Self {
        Self {
            mirror: v & 0x1 != 0,
            clamp: v & 0x2 != 0,
        }
    }
}

impl From<WrapMode> for u8 {
    fn from(m: WrapMode) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeResult<Ptr> {
    Complete(F3DCommand<Ptr>),
    TextureRectangle1 {
        rect: Rectangle<u32>,
        tile: TileIndex,
    },
    TextureRectangle2 {
        rect: Rectangle<u32>,
        tile: TileIndex,
        s: u16,
        t: u16,
    },
}

impl<Ptr> DecodeResult<Ptr> {
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete(_))
    }

    #[track_caller]
    pub fn unwrap(self) -> F3DCommand<Ptr> {
        match self {
            Self::Complete(command) => command,
            _ => panic!("unwrap() called on partial result"),
        }
    }

    #[track_caller]
    pub fn next(self, cmd_cont: RawF3DCommand<Ptr>) -> Self {
        let w1 = cmd_cont.w1;
        let cmd = cmd_cont.w0 >> 24;

        if ![0xB3, 0xB2, 0xB1].contains(&cmd) {
            return Self::Complete(F3DCommand::Unknown(cmd_cont));
        }

        match self {
            Self::Complete(_) => panic!("next() called on complete result"),
            Self::TextureRectangle1 { rect, tile } => Self::TextureRectangle2 {
                rect,
                tile,
                s: (w1 >> 16) as u16,
                t: w1 as u16,
            },
            Self::TextureRectangle2 { rect, tile, s, t } => Self::Complete(F3DCommand::Rdp(
                DPCommand::TextureRectangle(TextureRectangle {
                    rect,
                    tile,
                    s,
                    t,
                    dsdx: (w1 >> 16) as u16,
                    dtdy: w1 as u16,
                }),
            )),
        }
    }
}

pub fn decode_f3d_command<Ptr: Copy>(raw_command: RawF3DCommand<Ptr>) -> DecodeResult<Ptr> {
    use DPCommand::*;
    use F3DCommand::*;
    use SPCommand::*;

    let w0 = raw_command.w0;
    let w1 = raw_command.w1;
    let w1p = raw_command.w1_ptr;
    let cmd = w0 >> 24;

    DecodeResult::Complete(match cmd {
        0x00 => NoOp,

        // DMA commands
        0x01 => {
            let p = ((w0 >> 16) & 0xFF) as u8;
            Rsp(Matrix {
                matrix: w1p,
                mode: (p & 0x01).try_into().unwrap(),
                op: (p & 0x02).try_into().unwrap(),
                push: p & 0x04 != 0,
            })
        }
        0x03 => {
            let p = (w0 >> 16) & 0xFF;
            match p {
                0x80 => Rsp(Viewport(w1p)),
                0x86..=0x94 => Rsp(Light {
                    light: w1p,
                    n: (p - 0x86) / 2 + 1,
                }),
                _ => Unknown(raw_command),
            }
        }
        0x04 => Rsp(Vertex {
            v: w1p,
            n: ((w0 >> 20) & 0xF) + 1,
            v0: (w0 >> 16) & 0xF,
        }),
        0x06 => {
            let p = (w0 >> 16) & 0xFF;
            match p {
                0 => Rsp(DisplayList(w1p)),
                1 => Rsp(BranchList(w1p)),
                _ => Unknown(raw_command),
            }
        }

        // IMMEDIATE commands
        0xBF => Rsp(OneTriangle {
            v0: ((w1 >> 16) & 0xFF) / 10,
            v1: ((w1 >> 8) & 0xFF) / 10,
            v2: (w1 & 0xFF) / 10,
            flag: w1 >> 24,
        }),
        0xBD => Rsp(PopMatrix(((w1 & 0x01) as u8).try_into().unwrap())),
        0xBC => {
            let index = w0 & 0xFF;
            match index {
                2 => Rsp(NumLights((w1 - 0x80000000) / 0x20 - 1)),
                6 => Rsp(Segment {
                    seg: ((w0 >> 8) & 0xFFFF) / 4,
                    base: w1p,
                }),
                8 => Rsp(FogFactor {
                    mul: ((w1 >> 16) & 0xFFFF) as i16,
                    offset: (w1 & 0xFFFF) as i16,
                }),
                _ => Unknown(raw_command),
            }
        }
        0xBB => Rsp(Texture {
            sc: (w1 >> 16) & 0xFFFF,
            tc: w1 & 0xFFFF,
            level: (w0 >> 11) & 0x7,
            tile: (w0 >> 8) & 0x7,
            on: (w0 & 0xFF) != 0,
        }),
        0xBA => {
            let shift = (w0 >> 8) & 0xFF;
            let data = (w1 >> shift) as u8;
            match shift {
                4 => Rdp(SetAlphaDither(data.try_into().unwrap())),
                6 => Rdp(SetColorDither(data.try_into().unwrap())),
                8 => Rdp(SetCombineKey(data != 0)),
                9 => Rdp(SetTextureConvert(data.try_into().unwrap())),
                12 => Rdp(SetTextureFilter(data.try_into().unwrap())),
                14 => Rdp(SetTextureLUT(data.try_into().unwrap())),
                16 => Rdp(SetTextureLOD(data != 0)),
                17 => Rdp(SetTextureDetail(data.try_into().unwrap())),
                19 => Rdp(SetTexturePersp(data != 0)),
                20 => Rdp(SetCycleType(data.try_into().unwrap())),
                23 => Rdp(PipelineMode(data.try_into().unwrap())),
                _ => Unknown(raw_command),
            }
        }
        0xB9 => {
            let shift = (w0 >> 8) & 0xFF;
            match shift {
                0 => Rdp(SetAlphaCompare(((w1 >> shift) as u8).try_into().unwrap())),
                2 => Rdp(SetDepthSource(((w1 >> shift) as u8).try_into().unwrap())),
                3 => Rdp(SetRenderMode(RenderMode {
                    flags: RenderModeFlags::from_bits_truncate(w1 as u16),
                    cvg_dst: (((w1 >> 8) & 0x3) as u8).try_into().unwrap(),
                    z_mode: (((w1 >> 10) & 0x3) as u8).try_into().unwrap(),
                    blend_cycle1: BlendMode {
                        color1: (((w1 >> 30) & 0x3) as u8).try_into().unwrap(),
                        alpha1: (((w1 >> 26) & 0x3) as u8).try_into().unwrap(),
                        color2: (((w1 >> 22) & 0x3) as u8).try_into().unwrap(),
                        alpha2: (((w1 >> 18) & 0x3) as u8).try_into().unwrap(),
                    },
                    blend_cycle2: BlendMode {
                        color1: (((w1 >> 28) & 0x3) as u8).try_into().unwrap(),
                        alpha1: (((w1 >> 24) & 0x3) as u8).try_into().unwrap(),
                        color2: (((w1 >> 20) & 0x3) as u8).try_into().unwrap(),
                        alpha2: (((w1 >> 16) & 0x3) as u8).try_into().unwrap(),
                    },
                })),
                _ => Unknown(raw_command),
            }
        }
        0xB8 => Rsp(EndDisplayList),
        0xB7 => Rsp(SetGeometryMode(GeometryModes::from_bits_truncate(w1))),
        0xB6 => Rsp(ClearGeometryMode(GeometryModes::from_bits_truncate(w1))),
        0xB4 => Rdp(PerspNormalize(w1 as u16)),
        // RDPHALF_X, not expected here
        0xB3 | 0xB2 | 0xB1 => Unknown(raw_command),

        // RDP commands
        0xFF => Rdp(SetColorImage(Image {
            fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
            size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
            width: (w0 & 0xFFF) + 1,
            img: w1p,
        })),
        0xFE => Rdp(SetDepthImage(w1p)),
        0xFD => Rdp(SetTextureImage(Image {
            fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
            size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
            width: (w0 & 0xFFF) + 1,
            img: w1p,
        })),
        0xFC => {
            let cc1 = [
                ((w0 >> 20) & 0xF) as u8,
                ((w1 >> 28) & 0xF) as u8,
                ((w0 >> 15) & 0x1F) as u8,
                ((w1 >> 15) & 0x7) as u8,
            ];
            let ac1 = [
                ((w0 >> 12) & 0x7) as u8,
                ((w1 >> 12) & 0x7) as u8,
                ((w0 >> 9) & 0x7) as u8,
                ((w1 >> 9) & 0x7) as u8,
            ];
            let cc2 = [
                ((w0 >> 5) & 0xF) as u8,
                ((w1 >> 24) & 0xF) as u8,
                (w0 & 0x1F) as u8,
                ((w1 >> 6) & 0x7) as u8,
            ];
            let ac2 = [
                ((w1 >> 21) & 0x7) as u8,
                ((w1 >> 3) & 0x7) as u8,
                ((w1 >> 18) & 0x7) as u8,
                (w1 & 0x7) as u8,
            ];
            Rdp(SetCombineMode(CombineMode {
                color1: cc1.try_into().unwrap(),
                alpha1: ac1.try_into().unwrap(),
                color2: cc2.try_into().unwrap(),
                alpha2: ac2.try_into().unwrap(),
            }))
        }
        //   0xFC => {
        // 	   let    cc1 = ((w0 >> 20) & 0xF, (w1 >> 28) & 0xF, (w0 >> 15) & 0x1F, (w1 >> 15) & 0x7);
        // 	   let    ac1 = ((w0 >> 12) & 0x7, (w1 >> 12) & 0x7, (w0 >> 9) & 0x7, (w1 >> 9) & 0x7);
        // 	   let    cc2 = ((w0 >> 5) & 0xF, (w1 >> 24) & 0xF, (w0 >> 0) & 0x1F, (w1 >> 6) & 0x7);
        // 	   let    ac2 = ((w1 >> 21) & 0x7, (w1 >> 3) & 0x7, (w1 >> 18) & 0x7, (w1 >> 0) & 0x7);
        //
        //     def get_ccmux(p):
        //       i, m = p;
        // 	   let      ccmux = {
        //         0: "combined",
        //         1: "texel0",
        //         2: "texel1",
        //         3: "primitive",
        //         4: "shade",
        //         5: "environment",
        //       }
        //       if i == 0 {
        //         ccmux[6] = "1";
        //         ccmux[7] = "noise";
        //       } else if i == 1 {
        //         ccmux[6] = "center";
        //         ccmux[7] = "k4";
        //       } else if i == 2 {
        //         ccmux.update({
        //             6: "scale",
        //             7: "combined_alpha",
        //             8: "texel0_alpha",
        //             9: "texel1_alpha",
        //             10: "primitive_alpha",
        //             11: "shade_alpha",
        //             12: "environment_alpha",
        //             13: "lod_fraction",
        //             14: "prim_lod_frac",
        //             15: "k5",
        //           });
        //       } else if i == 3 {
        //         ccmux[6] = "1";
        //       return ccmux.get(m) or "0";
        //
        //     def get_acmux(p):
        //       i, m = p;
        // 	   let      acmux = {
        //         0: "combined_alpha",
        //         1: "texel0_alpha",
        //         2: "texel1_alpha",
        //         3: "primitive_alpha",
        //         4: "shade_alpha",
        //         5: "environment_alpha",
        //         6: "1",
        //         7: "0",
        //       }
        //       if i == 2 {
        //         acmux[0] = "lod_fraction";
        //         acmux[6] = "prim_lod_frac";
        //       return acmux[m];
        //
        //     return ("gDPSetCombineMode",
        //       tuple(map(get_ccmux, enumerate(cc1))),
        //       tuple(map(get_acmux, enumerate(ac1))),
        //       tuple(map(get_ccmux, enumerate(cc2))),
        //       tuple(map(get_acmux, enumerate(ac2))));
        //  }
        0xFB => Rdp(SetEnvColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        })),
        0xFA => Rdp(SetPrimColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        })),
        0xF9 => Rdp(SetBlendColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        })),
        0xF8 => Rdp(SetFogColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        })),
        0xF7 => Rdp(SetFillColor([
            FillColor((w1 >> 16) as u16),
            FillColor((w1 & 0xFFFF) as u16),
        ])),
        0xF6 => Rdp(FillRectangle(Rectangle {
            ulx: (w1 >> 14) & 0x3FF,
            uly: (w1 >> 2) & 0x3FF,
            lrx: (w0 >> 14) & 0x3FF,
            lry: (w0 >> 2) & 0x3FF,
        })),
        0xF5 => Rdp(SetTile(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TileParams {
                fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
                size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
                line: (w0 >> 9) & 0x1FF,
                tmem: w0 & 0x1FF,
                palette: (w1 >> 20) & 0xF,
                cmt: (((w1 >> 18) & 0x3) as u8).into(),
                maskt: (w1 >> 14) & 0xF,
                shiftt: (w1 >> 10) & 0xF,
                cms: (((w1 >> 8) & 0x3) as u8).into(),
                masks: (w1 >> 4) & 0xF,
                shifts: w1 & 0xF,
            },
        )),
        0xF4 => Rdp(LoadTile(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TileSize {
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                lrt: w1 & 0xFFF,
            },
        )),
        0xF3 => Rdp(LoadBlock(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TextureBlock {
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                dxt: w1 & 0xFFF,
            },
        )),
        0xF2 => Rdp(SetTileSize(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TileSize {
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                lrt: w1 & 0xFFF,
            },
        )),
        0xF0 => Rdp(LoadTLUTCmd(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            ((w1 >> 14) & 0x3FF) as u32,
        )),
        0xEF => Rdp(SetOtherMode(Unimplemented { w0, w1 })),
        0xEE => Rdp(SetPrimDepth(PrimDepth {
            z: (w1 >> 16) as u16,
            dz: (w1 & 0xFFFF) as u16,
        })),
        0xED => Rdp(SetScissor(
            (((w1 >> 24) & 0xFF) as u8).try_into().unwrap(),
            Rectangle {
                ulx: ((w0 >> 12) & 0xFFF) as u16,
                uly: (w0 & 0xFFF) as u16,
                lrx: ((w1 >> 12) & 0xFFF) as u16,
                lry: (w1 & 0xFFF) as u16,
            },
        )),
        0xEC => Rdp(SetConvert(Unimplemented { w0, w1 })),
        0xEB => Rdp(SetKeyR(Unimplemented { w0, w1 })),
        0xEA => Rdp(SetKeyGB(Unimplemented { w0, w1 })),
        0xE9 => Rdp(FullSync),
        0xE8 => Rdp(TileSync),
        0xE7 => Rdp(PipeSync),
        0xE6 => Rdp(LoadSync),
        0xE5 => Rdp(TextureRectangleFlip(Unimplemented { w0, w1 })),
        0xE4 => {
            return DecodeResult::TextureRectangle1 {
                rect: Rectangle {
                    ulx: (w1 >> 12) & 0xFFF,
                    uly: w1 & 0xFFF,
                    lrx: (w0 >> 12) & 0xFFF,
                    lry: w0 & 0xFFF,
                },
                tile: TileIndex(((w1 >> 24) & 0x7) as u8),
            };
        }
        _ => Unknown(raw_command),
    })
}

pub fn decode_f3d_display_list<Ptr, I: Iterator<Item = RawF3DCommand<Ptr>>>(
    raw_dl: I,
) -> F3DCommandIter<I> {
    F3DCommandIter { raw_dl }
}

#[derive(Debug)]
pub struct F3DCommandIter<I> {
    raw_dl: I,
}

impl<Ptr, I> Iterator for F3DCommandIter<I>
where
    Ptr: Copy,
    I: Iterator<Item = RawF3DCommand<Ptr>>,
{
    type Item = F3DCommand<Ptr>;

    fn next(&mut self) -> Option<Self::Item> {
        let raw_command = self.raw_dl.next()?;
        let mut result = decode_f3d_command(raw_command);
        while !result.is_complete() {
            match self.raw_dl.next() {
                Some(raw_command_cont) => {
                    result = result.next(raw_command_cont);
                }
                None => {
                    return Some(F3DCommand::Unknown(raw_command));
                }
            }
        }
        Some(result.unwrap())
    }
}
