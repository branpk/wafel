use bitflags::bitflags;
use num_enum::{FromPrimitive, TryFromPrimitive};

pub trait RawDLCommand: Copy {
    type Ptr: Copy;

    fn w0(self) -> u32;
    fn w1(self) -> u32;
    fn w1p(self) -> Self::Ptr;
}

impl RawDLCommand for [u32; 2] {
    type Ptr = u32;

    fn w0(self) -> u32 {
        self[0]
    }

    fn w1(self) -> u32 {
        self[1]
    }

    fn w1p(self) -> Self::Ptr {
        self[1]
    }
}

impl RawDLCommand for [u64; 2] {
    type Ptr = u64;

    fn w0(self) -> u32 {
        self[0] as u32
    }

    fn w1(self) -> u32 {
        self[1] as u32
    }

    fn w1p(self) -> Self::Ptr {
        self[1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DLCommand<Ptr> {
    NoOp,
    Rsp(SPCommand<Ptr>),
    Rdp(DPCommand<Ptr>),
    Unknown { w0: u32, w1: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
        mul: u32,
        offset: u32,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DPCommand<Ptr> {
    SetColorImage {
        fmt: ImageFormat,
        size: ComponentSize,
        width: u32,
        img: Ptr,
    },
    SetDepthImage(Ptr),
    SetTextureImage {
        fmt: ImageFormat,
        size: ComponentSize,
        width: u32,
        img: Ptr,
    },
    SetEnvColor(Rgba8),
    SetBlendColor(Rgba8),
    SetFogColor(Rgba8),
    SetFillColor {
        /// rgba is 5551 bits
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        // zdz is 14, 2 bits
        z: u8,
        dz: u8,
    },
    FillRectangle {
        ulx: u32,
        uly: u32,
        lrx: u32,
        lry: u32,
    },
    SetTile {
        fmt: ImageFormat,
        size: ComponentSize,
        line: u32,
        tmem: u32,
        tile: u32,
        palette: u32,
        cmt: WrapMode,
        maskt: u32,
        shiftt: u32,
        cms: WrapMode,
        masks: u32,
        shifts: u32,
    },
    LoadBlock {
        tile: u32,
        uls: u32,
        ult: u32,
        lrs: u32,
        dxt: u32,
    },
    SetTileSize {
        tile: u32,
        uls: u32,
        ult: u32,
        lrs: u32,
        lrt: u32,
    },
    SetScissor {
        mode: ScissorMode,
        ulx: f32,
        uly: f32,
        lrx: f32,
        lry: f32,
    },
    FullSync,
    TileSync,
    PipeSync,
    LoadSync,
    TextureRectangle {
        ulx: u32,
        uly: u32,
        lrx: u32,
        lry: u32,
        tile: u32,
        s: u32,
        t: u32,
        dsdx: u32,
        dtdy: u32,
    },

    // TODO:
    SetAlphaCompare(AlphaCompare),
    SetDepthSource(DepthSource),
    SetRenderMode {
        // TODO
    },
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
    SetCombineMode {
        // TODO
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlphaCompare {
    None,
    Threshold,
    Dither,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthSource {
    Pixel,
    Prim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ColorDither {
    MagicSq = 0,
    Bayer = 1,
    Noise = 2,
    Disable = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureConvert {
    Conv = 0,
    FiltConv = 5,
    Filt = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureFilter {
    Point = 0,
    Average = 3,
    Bilerp = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureLUT {
    None = 0,
    Rgba16 = 2,
    Ia16 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum TextureDetail {
    Clamp = 0,
    Sharpen = 1,
    Detail = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum CycleType {
    OneCycle = 0,
    TwoCycle = 1,
    Copy = 2,
    Fill = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum PipelineMode {
    OnePrimitive = 1,
    NPrimitive = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
#[allow(clippy::enum_variant_names)]
pub enum ScissorMode {
    NonInterlace = 0,
    OddInterlace = 3,
    EvenInterlace = 2,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum ComponentSize {
    Bits4 = 0,
    Bits8 = 1,
    Bits16 = 2,
    Bits32 = 3,
    DD = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WrapMode {
    pub mirror: bool,
    pub clamp: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub fn parse_display_list<C: RawDLCommand>(
    raw_dl: impl Iterator<Item = C>,
) -> impl Iterator<Item = DLCommand<C::Ptr>> {
    DlIter { raw_dl }
}

#[derive(Debug)]
struct DlIter<I> {
    raw_dl: I,
}

impl<C, I> Iterator for DlIter<I>
where
    C: RawDLCommand,
    I: Iterator<Item = C>,
{
    type Item = DLCommand<C::Ptr>;

    fn next(&mut self) -> Option<Self::Item> {
        use DLCommand::*;
        use DPCommand::*;
        use SPCommand::*;

        let full_cmd = self.raw_dl.next()?;
        let w0 = full_cmd.w0();
        let w1 = full_cmd.w1();
        let w1p = full_cmd.w1p();
        let cmd = w0 >> 24;

        Some(match cmd {
            // DMA commands
            0x00 => NoOp,
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
                    _ => Unknown { w0, w1 },
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
                    _ => Unknown { w0, w1 },
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
                        mul: ((w1 >> 16) & 0xFFFF),
                        offset: (w1 & 0xFFFF),
                    }),
                    _ => Unknown { w0, w1 },
                }
            }
            0xBB => Rsp(Texture {
                sc: (w1 >> 16) & 0xFFFF,
                tc: w1 & 0xFFFF,
                level: (w0 >> 11) & 0x7,
                tile: (w0 >> 8) & 0x7,
                on: (w0 & 0xFF) != 0,
            }),
            // TODO: SetOtherMode_H
            0xBA => {
                let shift = (w0 >> 8) & 0xFF;
                let data = (w1 >> shift) as u8;
                if w0 == 0xBA000602 {
                    Rdp(SetColorDither(data.try_into().unwrap()))
                } else if w0 == 0xBA000801 {
                    Rdp(SetCombineKey(data != 0))
                } else if w0 == 0xBA000903 {
                    Rdp(SetTextureConvert(data.try_into().unwrap()))
                } else if w0 == 0xBA000C02 {
                    Rdp(SetTextureFilter(data.try_into().unwrap()))
                } else if w0 == 0xBA000E02 {
                    Rdp(SetTextureLUT(data.try_into().unwrap()))
                } else if w0 == 0xBA001001 {
                    Rdp(SetTextureLOD(data != 0))
                } else if w0 == 0xBA001102 {
                    Rdp(SetTextureDetail(data.try_into().unwrap()))
                } else if w0 == 0xBA001301 {
                    Rdp(SetTexturePersp(data != 0))
                } else if w0 == 0xBA001402 {
                    Rdp(SetCycleType(data.try_into().unwrap()))
                } else if w0 == 0xBA001701 {
                    Rdp(PipelineMode(data.try_into().unwrap()))
                } else {
                    Unknown { w0, w1 }
                }
            }
            // TODO: SetOtherMode_L
            //   0xB9 => {
            // 	   let    shift = (w0 >> 8) & 0xFF;
            // 	   let    data = w1 >> shift;
            //
            //     if w0 == 0xB9000002 {
            //       Rdp(SetAlphaCompare {  {0: "none", 1: "threshold", 3: "dither"}[data] })
            //
            //     } else if w0 == 0xB9000201 {
            //       Rdp(SetDepthSource {  {0: "pixel", 1: "prim"}[data] })
            //
            //     } else if w0 == 0xB900031D {
            //       # TODO: All render mode flags;
            //
            // 	   let      shared = {}
            //       shared["aa"] = (w1 & 0x8) != 0;
            //       shared["z_cmp"] = (w1 & 0x10) != 0;
            //       shared["z_upd"] = (w1 & 0x20) != 0;
            //       shared["im_rd"] = (w1 & 0x40) != 0;
            //       shared["clr_on_cvg"] = (w1 & 0x80) != 0;
            //       if w1 & 0x100 {
            //         shared["cvg_dst"] = "wrap";
            //       } else if w1 & 0x200 {
            //         shared["cvg_dst"] = "full";
            //       } else if w1 & 0x300 {
            //         shared["cvg_dst"] = "save";
            //       } else {
            //         shared["cvg_dst"] = "clamp";
            //       if w1 & 0x400 {
            //         shared["zmode"] = "inter";
            //       } else if w1 & 0x800 {
            //         shared["zmode"] = "xlu";
            //       } else if w1 & 0xC00 {
            //         shared["zmode"] = "dec";
            //       } else {
            //         shared["zmode"] = "opa";
            //       shared["cvg_x_alpha"] = (w1 & 0x1000) != 0;
            //       shared["alpha_cvg_sel"] = (w1 & 0x2000) != 0;
            //       shared["force_bl"] = (w1 & 0x4000) != 0;
            //       shared["_tex_edge"] = (w1 & 0x8000) != 0;
            //
            //       # 0x005041C8, 0x00552048, 0x0F0A4000, ...;
            //       Rdp(SetRenderMode {  shared, "<unimplemented>", "<unimplemented>" })
            //  }
            0xB8 => Rsp(EndDisplayList),
            0xB7 => Rsp(SetGeometryMode(GeometryModes::from_bits_truncate(w1))),
            0xB6 => Rsp(ClearGeometryMode(GeometryModes::from_bits_truncate(w1))),
            // TODO: RDP_HALF_N

            // RDP commands
            0xFF => Rdp(SetColorImage {
                fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
                size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
                width: (w0 & 0xFFF) + 1,
                img: w1p,
            }),
            0xFE => Rdp(SetDepthImage(w1p)),
            0xFD => Rdp(SetTextureImage {
                fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
                size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
                width: (w0 & 0xFFF) + 1,
                img: w1p,
            }),
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
            0xFB => Rdp(SetEnvColor(Rgba8 {
                r: (w1 >> 24) as u8,
                g: (w1 >> 16) as u8,
                b: (w1 >> 8) as u8,
                a: w1 as u8,
            })),
            // TODO: G_SETPRIMCOLOR
            0xF9 => Rdp(SetBlendColor(Rgba8 {
                r: (w1 >> 24) as u8,
                g: (w1 >> 16) as u8,
                b: (w1 >> 8) as u8,
                a: w1 as u8,
            })),
            0xF8 => Rdp(SetFogColor(Rgba8 {
                r: (w1 >> 24) as u8,
                g: (w1 >> 16) as u8,
                b: (w1 >> 8) as u8,
                a: w1 as u8,
            })),
            //   0xF7 => {
            // 	   let    c = w1 & 0xFFFF;
            // 	   let    rgba = ((c >> 8) & 0xF8, (c >> 3) & 0xF8, (c << 2) & 0xF8, (c >> 0) & 0x1);
            // 	   let    zdz = (c >> 2, c & 0x3);
            //     Rdp(SetFillColor {  rgba, zdz })
            //  }
            //   0xF6 => {
            // 	   let    lrx = (w0 >> 14) & 0x3FF;
            // 	   let    lry = (w0 >> 2) & 0x3FF;
            // 	   let    ulx = (w1 >> 14) & 0x3FF;
            // 	   let    uly = (w1 >> 2) & 0x3FF;
            //     Rdp(FillRectangle {  ulx, uly, lrx, lry })
            //  }
            //   0xF5 => {
            // 	   let    fmt = {
            //       0: "rgba",
            //       1: "yuv",
            //       2: "ci",
            //       3: "ia",
            //       4: "i",
            // 	   let    size = {
            //       0: "4b",
            //       1: "8b",
            //       2: "16b",
            //       3: "32b",
            //       5: "dd",
            // 	   let    line = (w0 >> 9) & 0x1FF;
            // 	   let    tmem = w0 & 0x1FF;
            // 	   let    tile = (w1 >> 24) & 0x7;
            // 	   let    palette = (w1 >> 20) & 0xF;
            // 	   let    cmt = (w1 >> 18) & 0x3;
            // 	   let    cmt = ("mirror" if cmt & 0x1 else "nomirror", "clamp" if cmt & 0x2 else "wrap");
            // 	   let    maskt = (w1 >> 14) & 0xF;
            // 	   let    shiftt = (w1 >> 10) & 0xF;
            // 	   let    cms = (w1 >> 8) & 0x3;
            // 	   let    cms = ("mirror" if cms & 0x1 else "nomirror", "clamp" if cms & 0x2 else "wrap");
            // 	   let    masks = (w1 >> 4) & 0xF;
            // 	   let    shifts = w1 & 0xF;
            //     Rdp(SetTile {  fmt, size, line, tmem, tile, palette, cmt, maskt, shiftt, cms, masks, shifts })
            //  }
            // TODO: G_LOADTILE
            0xF3 => Rdp(LoadBlock {
                tile: (w1 >> 24) & 0x7,
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                dxt: w1 & 0xFFF,
            }),
            0xF2 => Rdp(SetTileSize {
                tile: (w1 >> 24) & 0x7,
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                lrt: w1 & 0xFFF,
            }),
            // TODO: G_LOADTLUT
            // TODO: G_RDPSETOTHERMODE
            // TODO: G_SETPRIMDEPTH
            0xED => Rdp(SetScissor {
                mode: (((w1 >> 24) & 0xFF) as u8).try_into().unwrap(),
                ulx: (((w0 >> 12) & 0xFFF) / 4) as f32,
                uly: ((w0 & 0xFFF) / 4) as f32,
                lrx: (((w1 >> 12) & 0xFFF) / 4) as f32,
                lry: ((w1 & 0xFFF) / 4) as f32,
            }),
            // TODO: G_SETCONVERT
            // TODO: G_SETKEYR
            // TODO: G_SETKEYGB
            0xE9 => Rdp(FullSync),
            0xE8 => Rdp(TileSync),
            0xE7 => Rdp(PipeSync),
            0xE6 => Rdp(LoadSync),
            // TODO: G_TEXRECTFLIP
            //   0xE4 => {
            // 	   let    ulx = ((w1 >> 12) & 0xFFF) / (1 << 2);
            // 	   let    uly = ((w1 >> 0) & 0xFFF) / (1 << 2);
            // 	   let    lrx = ((w0 >> 12) & 0xFFF) / (1 << 2);
            // 	   let    lry = ((w0 >> 0) & 0xFFF) / (1 << 2);
            // 	   let    tile = (w1 >> 24) & 0x7;
            //
            // 	   let    dl_cmd = get_next_cmd();
            // 	   let    w0 = dl_cmd >> 32;
            // 	   let    w1 = dl_cmd & <u32>0xFFFFFFFF;
            //     # This is supposed to be 0xB4 (??);
            // 	   let    s = ((w1 >> 16) & 0xFFFF) / (1 << 5);
            // 	   let    t = ((w1 >> 0) & 0xFFFF) / (1 << 5);
            //
            // 	   let    dl_cmd = get_next_cmd();
            // 	   let    w0 = dl_cmd >> 32;
            // 	   let    w1 = dl_cmd & <u32>0xFFFFFFFF;
            //     # This is supposed to be 0xB3 (??);
            // 	   let    dsdx = ((w1 >> 16) & 0xFFFF) / (1 << 10);
            // 	   let    dtdy = ((w1 >> 0) & 0xFFFF) / (1 << 10);
            //
            //     Rsp(TextureRectangle {  ulx, uly, lrx, lry, tile, s, t, dsdx, dtdy })
            //  }
            _ => Unknown { w0, w1 },
        })
    }
}
