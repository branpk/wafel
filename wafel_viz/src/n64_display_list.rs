use bitflags::bitflags;

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
    ClearGeometryMode(GeometryModes),
    SetGeometryMode(GeometryModes),
    EndDisplayList,
    Texture {
        sc: u32,
        tc: u32,
        level: u32,
        tile: u32,
        on: bool,
    },
    NumLights(u32),
    Segment {
        seg: u32,
        base: Ptr,
    },
    FogFactor {
        fm: u32,
        fo: u32,
    },
    PopMatrix(MatrixMode),
    OneTriangle {
        v0: u32,
        v1: u32,
        v2: u32,
        flag: u32,
    },
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatrixMode {
    Proj,
    ModelView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatrixOp {
    Load,
    Mul,
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
    LoadSync,
    PipeSync,
    TileSync,
    FullSync,
    SetScissor {
        mode: ScissorMode,
        ulx: f32,
        uly: f32,
        lrx: f32,
        lry: f32,
    },
    SetTileSize {
        tile: u32,
        uls: u32,
        ult: u32,
        lrs: u32,
        lrt: u32,
    },
    LoadBlock {
        tile: u32,
        uls: u32,
        ult: u32,
        lrs: u32,
        dxt: u32,
    },
    SetTile {
        fmt: ImageFormat,
        size: u32,
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
    FillRectangle {
        ulx: u32,
        uly: u32,
        lrx: u32,
        lry: u32,
    },
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
    SetFogColor(Rgba8),
    SetBlendColor(Rgba8),
    SetEnvColor(Rgba8),
    SetCombineMode {
        // TODO
    },
    SetTextureImage {
        fmt: ImageFormat,
        size: u32,
        width: u32,
        img: Ptr,
    },
    SetDepthImage(Ptr),
    SetColorImage {
        fmt: ImageFormat,
        size: u32,
        width: u32,
        img: Ptr,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorDither {
    MagicSq,
    Bayer,
    Noise,
    Disable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureConvert {
    Conv,
    FiltConv,
    Filt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFilter {
    Point,
    Average,
    Bilerp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureLUT {
    None,
    Rgba16,
    Ia16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDetail {
    Clamp,
    Sharpen,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CycleType {
    OneCycle,
    TwoCycle,
    Copy,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineMode {
    OnePrimitive,
    NPrimitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScissorMode {
    NonInterlace,
    OddInterlace,
    EvenInterlace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Rgba,
    Yuv,
    Ci,
    Ia,
    I,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapMode {
    Mirror,
    NoMirror,
    Wrap,
    Clamp,
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

        let check = |cond: bool| {
            if !cond {
                panic!("Invalid F3D command: {:#010X}_{:080X}", w0, w1);
            }
        };

        Some(match cmd {
            0x01 => {
                let p = (w0 >> 16) & 0xFF;
                check((w0 & 0xFFFF) == 0x40);
                check((p & 0x7) == p);
                Rsp(Matrix {
                    matrix: w1p,
                    mode: if p & 0x01 != 0 {
                        MatrixMode::Proj
                    } else {
                        MatrixMode::ModelView
                    },
                    op: if p & 0x02 != 0 {
                        MatrixOp::Load
                    } else {
                        MatrixOp::Mul
                    },
                    push: p & 0x04 != 0,
                })
            }

            //   elif cmd == 0x03:
            //     param = (w0 >> 16) & 0xFF
            //     length = w0 & 0xFFFF
            //
            //     if param == 0x80:
            //       gbi_assert(length == 0x10, dl_cmd)
            //       return ('gSPViewport', w1)
            //
            //     if param in (0x86, 0x88, 0x8A, 0x8C, 0x8E, 0x90, 0x92, 0x94):
            //       gbi_assert(length == 0x10, dl_cmd)
            //       return ('gSPLight', w1, (param - 0x86) // 2 + 1)
            //
            //   elif cmd == 0x04:
            //     n = ((w0 >> 20) & 0xF) + 1
            //     v0 = (w0 >> 16) & 0xF
            //     gbi_assert((w0 & 0xFFFF) == 0x10 * n, dl_cmd)
            //     return ('gSPVertex', w1, n, v0)
            //
            //   elif cmd == 0x06:
            //     gbi_assert(w0 in (0x06000000, 0x06010000), dl_cmd)
            //     if w0 == 0x06000000:
            //       return ('gSPDisplayList', w1)
            //     else:
            //       return ('gSPBranchList', w1)
            //
            //   elif cmd == 0xB6 or cmd == 0xB7:
            //     gbi_assert(w0 in (0xB6000000, 0xB7000000), dl_cmd)
            //     mode_bits = {
            //       'zbuffer':            0x00000001,
            //       'texture_enable':     0x00000002,
            //       'shade':              0x00000004,
            //       'shading_smooth':     0x00000200,
            //       'cull_front':         0x00001000,
            //       'cull_back':          0x00002000,
            //       'fog':                0x00010000,
            //       'lighting':           0x00020000,
            //       'texture_gen':        0x00040000,
            //       'texture_gen_linear': 0x00080000,
            //       'lod':                0x00100000,
            //       'clipping':           0x00800000,
            //     }
            //     modes = tuple(m for m in mode_bits if w1 & mode_bits[m])
            //     mode0 = 0
            //     for m in modes:
            //       mode0 |= mode_bits[m]
            //     gbi_assert(w1 == mode0, dl_cmd)
            //     if cmd == 0xB6:
            //       return ('gSPClearGeometryMode', modes)
            //     else:
            //       return ('gSPSetGeometryMode', modes)
            //
            //   elif cmd == 0xB8:
            //     gbi_assert(dl_cmd == 0xB8000000_00000000, dl_cmd)
            //     return ('gSPEndDisplayList',)
            //
            //   elif cmd == 0xB9:
            //     shift = (w0 >> 8) & 0xFF
            //     data = w1 >> shift
            //
            //     if w0 == 0xB9000002:
            //       return ('gDPSetAlphaCompare', {0: 'none', 1: 'threshold', 3: 'dither'}[data])
            //
            //     elif w0 == 0xB9000201:
            //       return ('gDPSetDepthSource', {0: 'pixel', 1: 'prim'}[data])
            //
            //     elif w0 == 0xB900031D:
            //       # TODO: All render mode flags
            //
            //       shared = {}
            //       shared['aa'] = (w1 & 0x8) != 0
            //       shared['z_cmp'] = (w1 & 0x10) != 0
            //       shared['z_upd'] = (w1 & 0x20) != 0
            //       shared['im_rd'] = (w1 & 0x40) != 0
            //       shared['clr_on_cvg'] = (w1 & 0x80) != 0
            //       if w1 & 0x100:
            //         shared['cvg_dst'] = 'wrap'
            //       elif w1 & 0x200:
            //         shared['cvg_dst'] = 'full'
            //       elif w1 & 0x300:
            //         shared['cvg_dst'] = 'save'
            //       else:
            //         shared['cvg_dst'] = 'clamp'
            //       if w1 & 0x400:
            //         shared['zmode'] = 'inter'
            //       elif w1 & 0x800:
            //         shared['zmode'] = 'xlu'
            //       elif w1 & 0xC00:
            //         shared['zmode'] = 'dec'
            //       else:
            //         shared['zmode'] = 'opa'
            //       shared['cvg_x_alpha'] = (w1 & 0x1000) != 0
            //       shared['alpha_cvg_sel'] = (w1 & 0x2000) != 0
            //       shared['force_bl'] = (w1 & 0x4000) != 0
            //       shared['_tex_edge'] = (w1 & 0x8000) != 0
            //
            //       # 0x005041C8, 0x00552048, 0x0F0A4000, ...
            //       return ('gDPSetRenderMode', shared, '<unimplemented>', '<unimplemented>')
            //
            //   elif cmd == 0xBA:
            //     shift = (w0 >> 8) & 0xFF
            //     data = w1 >> shift
            //     if w0 == 0xBA000602:
            //       return ('gDPSetColorDither', {0: 'magicsq', 1: 'bayer', 2: 'noise', 3: 'disable'}[data])
            //     elif w0 == 0xBA000801:
            //       return ('gDPSetCombineKey', {0: 'off', 1: 'on'}[data])
            //     elif w0 == 0xBA000903:
            //       return ('gDPSetTextureConvert', {0: 'conv', 5: 'filtconv', 6: 'conv'}[data])
            //     elif w0 == 0xBA000C02:
            //       return ('gDPSetTextureFilter', {0: 'point', 3: 'average', 2: 'bilerp'}[data])
            //     elif w0 == 0xBA000E02:
            //       return ('gDPSetTextureLUT', {0: 'none', 2: 'rgba16', 3: 'ia16'}[data])
            //     elif w0 == 0xBA001001:
            //       return ('gDPSetTextureLOD', {0: 'off', 1: 'on'}[data])
            //     elif w0 == 0xBA001102:
            //       return ('gDPSetTextureDetail', {0: 'clamp', 1: 'sharpen', 2: 'detail'}[data])
            //     elif w0 == 0xBA001301:
            //       return ('gDPSetTexturePersp', {0: 'off', 1: 'on'}[data])
            //     elif w0 == 0xBA001402:
            //       return ('gDPSetCycleType', {0: '1cycle', 1: '2cycle', 2: 'copy', 3: 'fill'}[data])
            //     elif w0 == 0xBA001701:
            //       return ('gDPPipelineMode', {1: '1primitive', 0: 'nprimitive'}[data])
            //
            //   elif cmd == 0xBB:
            //     gbi_assert(((w0 >> 16) & 0xFF) == 0, dl_cmd)
            //     level = (w0 >> 11) & 0x7
            //     tile = (w0 >> 8) & 0x7
            //     on = w0 & 0xFF
            //     sc = (w1 >> 16) & 0xFFFF
            //     tc = w1 & 0xFFFF
            //     return ('gSPTexture', sc, tc, level, tile, 'on' if on else 'off')
            //
            //   elif cmd == 0xBC:
            //     offset = (w0 >> 8) & 0xFFFF
            //     index = (w0 >> 0) & 0xFF
            //
            //     if index == 2 and offset == 0:
            //       return ('gSPNumLights', (w1 - 0x80000000) // 0x20 - 1)
            //
            //     elif index == 6:
            //       return ('gSPSegment', offset // 4, w1)
            //
            //     elif index == 8 and offset == 0:
            //       return ('gSPFogFactor', <s16><s64>((w1 >> 16) & 0xFFFF), <s16><s64>((w1 >> 0) & 0xFFFF))
            //
            //   elif cmd == 0xBD:
            //     gbi_assert(w0 == 0xBD000000, dl_cmd)
            //     gbi_assert((w1 & 0x1) == w1, dl_cmd)
            //     return ('gSPPopMatrix', 'proj' if w1 & 0x01 else 'modelview')
            //
            //   elif cmd == 0xBF:
            //     gbi_assert(w0 == 0xBF000000, dl_cmd)
            //     flag = w1 >> 24
            //     v0 = ((w1 >> 16) & 0xFF) // 10
            //     v1 = ((w1 >> 8) & 0xFF) // 10
            //     v2 = ((w1 >> 0) & 0xFF) // 10
            //     return ('gSP1Triangle', v0, v1, v2, flag)
            //
            //   elif cmd == 0xE4:
            //     ulx = ((w1 >> 12) & 0xFFF) / (1 << 2)
            //     uly = ((w1 >> 0) & 0xFFF) / (1 << 2)
            //     lrx = ((w0 >> 12) & 0xFFF) / (1 << 2)
            //     lry = ((w0 >> 0) & 0xFFF) / (1 << 2)
            //     tile = (w1 >> 24) & 0x7
            //
            //     dl_cmd = get_next_cmd()
            //     w0 = dl_cmd >> 32
            //     w1 = dl_cmd & <u32>0xFFFFFFFF
            //     # This is supposed to be 0xB4 (??)
            //     gbi_assert(w0 == 0xB3000000, dl_cmd)
            //     s = ((w1 >> 16) & 0xFFFF) / (1 << 5)
            //     t = ((w1 >> 0) & 0xFFFF) / (1 << 5)
            //
            //     dl_cmd = get_next_cmd()
            //     w0 = dl_cmd >> 32
            //     w1 = dl_cmd & <u32>0xFFFFFFFF
            //     # This is supposed to be 0xB3 (??)
            //     gbi_assert(w0 == 0xB2000000, dl_cmd)
            //     dsdx = ((w1 >> 16) & 0xFFFF) / (1 << 10)
            //     dtdy = ((w1 >> 0) & 0xFFFF) / (1 << 10)
            //
            //     return ('gSPTextureRectangle', ulx, uly, lrx, lry, tile, s, t, dsdx, dtdy)
            //
            //   elif cmd == 0xE6:
            //     gbi_assert(dl_cmd == 0xE6000000_00000000, dl_cmd)
            //     return ('gDPLoadSync',)
            //
            //   elif cmd == 0xE7:
            //     gbi_assert(dl_cmd == 0xE7000000_00000000, dl_cmd)
            //     return ('gDPPipeSync',)
            //
            //   elif cmd == 0xE8:
            //     gbi_assert(dl_cmd == 0xE8000000_00000000, dl_cmd)
            //     return ('gDPTileSync',)
            //
            //   elif cmd == 0xE9:
            //     gbi_assert(dl_cmd == 0xE9000000_00000000, dl_cmd)
            //     return ('gDPFullSync',)
            //
            //   elif cmd == 0xED:
            //     mode = {0: 'non_interlace', 3: 'odd_interlace', 2: 'even_interlace'}[(w1 >> 24) & 0xFF]
            //     ulx = <f32>((w0 >> 12) & 0xFFF) / 4
            //     uly = <f32>((w0 >> 0) & 0xFFF) / 4
            //     lrx = <f32>((w1 >> 12) & 0xFFF) / 4
            //     lry = <f32>((w1 >> 0) & 0xFFF) / 4
            //     return ('gDPSetScissor', mode, ulx, uly, lrx, lry)
            //
            //   elif cmd == 0xF2:
            //     uls = (w0 >> 12) & 0xFFF
            //     ult = w0 & 0xFFF
            //     tile = (w1 >> 24) & 0x7
            //     lrs = (w1 >> 12) & 0xFFF
            //     lrt = w1 & 0xFFF
            //     return ('gDPSetTileSize', tile, uls, ult, lrs, lrt)
            //
            //   elif cmd == 0xF3:
            //     uls = (w0 >> 12) & 0xFFF
            //     ult = w0 & 0xFFF
            //     tile = (w1 >> 24) & 0x7
            //     lrs = (w1 >> 12) & 0xFFF
            //     dxt = w1 & 0xFFF
            //     return ('gDPLoadBlock', tile, uls, ult, lrs, dxt)
            //
            //   elif cmd == 0xF5:
            //     fmt = {
            //       0: 'rgba',
            //       1: 'yuv',
            //       2: 'ci',
            //       3: 'ia',
            //       4: 'i',
            //     }.get((w0 >> 21) & 0x7) or gbi_assert(False, dl_cmd)
            //     size = {
            //       0: '4b',
            //       1: '8b',
            //       2: '16b',
            //       3: '32b',
            //       5: 'dd',
            //     }.get(((w0 >> 19) & 0x3) or gbi_assert(False, dl_cmd))
            //     line = (w0 >> 9) & 0x1FF
            //     tmem = w0 & 0x1FF
            //     tile = (w1 >> 24) & 0x7
            //     palette = (w1 >> 20) & 0xF
            //     cmt = (w1 >> 18) & 0x3
            //     cmt = ('mirror' if cmt & 0x1 else 'nomirror', 'clamp' if cmt & 0x2 else 'wrap')
            //     maskt = (w1 >> 14) & 0xF
            //     shiftt = (w1 >> 10) & 0xF
            //     cms = (w1 >> 8) & 0x3
            //     cms = ('mirror' if cms & 0x1 else 'nomirror', 'clamp' if cms & 0x2 else 'wrap')
            //     masks = (w1 >> 4) & 0xF
            //     shifts = w1 & 0xF
            //     return ('gDPSetTile', fmt, size, line, tmem, tile, palette, cmt, maskt, shiftt, cms, masks, shifts)
            //
            //   elif cmd == 0xF6:
            //     gbi_assert((w1 >> 24) == 0, dl_cmd)
            //     lrx = (w0 >> 14) & 0x3FF
            //     lry = (w0 >> 2) & 0x3FF
            //     ulx = (w1 >> 14) & 0x3FF
            //     uly = (w1 >> 2) & 0x3FF
            //     return ('gDPFillRectangle', ulx, uly, lrx, lry)
            //
            //   elif cmd == 0xF7:
            //     gbi_assert(w0 == 0xF7000000, dl_cmd)
            //     gbi_assert((w1 >> 16) == (w1 & 0xFFFF), dl_cmd)
            //     c = w1 & 0xFFFF
            //     rgba = ((c >> 8) & 0xF8, (c >> 3) & 0xF8, (c << 2) & 0xF8, (c >> 0) & 0x1)
            //     zdz = (c >> 2, c & 0x3)
            //     return ('gDPSetFillColor', rgba, zdz)
            //
            //   elif cmd == 0xF8:
            //     gbi_assert(w0 == 0xF8000000, dl_cmd)
            //     return ('gDPSetFogColor', (w1 >> 24) & 0xFF, (w1 >> 16) & 0xFF, (w1 >> 8) & 0xFF, w1 & 0xFF)
            //
            //   elif cmd == 0xF9:
            //     gbi_assert(w0 == 0xF9000000, dl_cmd)
            //     return ('gDPSetBlendColor', (w1 >> 24) & 0xFF, (w1 >> 16) & 0xFF, (w1 >> 8) & 0xFF, w1 & 0xFF)
            //
            //   elif cmd == 0xFB:
            //     gbi_assert(w0 == 0xFB000000, dl_cmd)
            //     return ('gDPSetEnvColor', (w1 >> 24) & 0xFF, (w1 >> 16) & 0xFF, (w1 >> 8) & 0xFF, w1 & 0xFF)
            //
            //   elif cmd == 0xFC:
            //     cc1 = ((w0 >> 20) & 0xF, (w1 >> 28) & 0xF, (w0 >> 15) & 0x1F, (w1 >> 15) & 0x7)
            //     ac1 = ((w0 >> 12) & 0x7, (w1 >> 12) & 0x7, (w0 >> 9) & 0x7, (w1 >> 9) & 0x7)
            //     cc2 = ((w0 >> 5) & 0xF, (w1 >> 24) & 0xF, (w0 >> 0) & 0x1F, (w1 >> 6) & 0x7)
            //     ac2 = ((w1 >> 21) & 0x7, (w1 >> 3) & 0x7, (w1 >> 18) & 0x7, (w1 >> 0) & 0x7)
            //
            //     def get_ccmux(p):
            //       i, m = p
            //       ccmux = {
            //         0: 'combined',
            //         1: 'texel0',
            //         2: 'texel1',
            //         3: 'primitive',
            //         4: 'shade',
            //         5: 'environment',
            //       }
            //       if i == 0:
            //         ccmux[6] = '1'
            //         ccmux[7] = 'noise'
            //       elif i == 1:
            //         ccmux[6] = 'center'
            //         ccmux[7] = 'k4'
            //       elif i == 2:
            //         ccmux.update({
            //             6: 'scale',
            //             7: 'combined_alpha',
            //             8: 'texel0_alpha',
            //             9: 'texel1_alpha',
            //             10: 'primitive_alpha',
            //             11: 'shade_alpha',
            //             12: 'environment_alpha',
            //             13: 'lod_fraction',
            //             14: 'prim_lod_frac',
            //             15: 'k5',
            //           })
            //       elif i == 3:
            //         ccmux[6] = '1'
            //       return ccmux.get(m) or '0'
            //
            //     def get_acmux(p):
            //       i, m = p
            //       acmux = {
            //         0: 'combined_alpha',
            //         1: 'texel0_alpha',
            //         2: 'texel1_alpha',
            //         3: 'primitive_alpha',
            //         4: 'shade_alpha',
            //         5: 'environment_alpha',
            //         6: '1',
            //         7: '0',
            //       }
            //       if i == 2:
            //         acmux[0] = 'lod_fraction'
            //         acmux[6] = 'prim_lod_frac'
            //       return acmux[m]
            //
            //     return ('gDPSetCombineMode',
            //       tuple(map(get_ccmux, enumerate(cc1))),
            //       tuple(map(get_acmux, enumerate(ac1))),
            //       tuple(map(get_ccmux, enumerate(cc2))),
            //       tuple(map(get_acmux, enumerate(ac2))))
            //
            //   elif cmd == 0xFD:
            //     fmt = {
            //       0: 'rgba',
            //       1: 'yuv',
            //       2: 'ci',
            //       3: 'ia',
            //       4: 'i',
            //     }.get((w0 >> 21) & 0x7) or gbi_assert(False, dl_cmd)
            //     size = {
            //       0: '4b',
            //       1: '8b',
            //       2: '16b',
            //       3: '32b',
            //       5: 'dd',
            //     }.get(((w0 >> 19) & 0x3) or gbi_assert(False, dl_cmd))
            //     width = (w0 & 0xFFF) + 1
            //     return ('gDPSetTextureImage', fmt, size, width, w1)
            //
            //   elif cmd == 0xFE:
            //     gbi_assert(w0 == 0xFE000000, dl_cmd)
            //     return ('gDPSetDepthImage', w1)
            //
            //   elif cmd == 0xFF:
            //     fmt = {
            //       0: 'rgba',
            //       1: 'yuv',
            //       2: 'ci',
            //       3: 'ia',
            //       4: 'i',
            //     }.get((w0 >> 21) & 0x7) or gbi_assert(False, dl_cmd)
            //     size = {
            //       0: '4b',
            //       1: '8b',
            //       2: '16b',
            //       3: '32b',
            //       5: 'dd',
            //     }.get(((w0 >> 19) & 0x3) or gbi_assert(False, dl_cmd))
            //     width = ((w0 >> 0) & 0xFFF) + 1
            //     return ('gDPSetColorImage', fmt, size, width, w1)
            _ => Unknown { w0, w1 },
        })
    }
}
