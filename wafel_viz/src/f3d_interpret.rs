use core::fmt;

use ordered_float::NotNan;

use crate::{f3d_decode::*, render_api::RenderBackend};

pub trait F3DSource {
    type Ptr: fmt::Debug + Copy;
    type DlIter: Iterator<Item = F3DCommand<Self::Ptr>>;

    fn root_dl(&self) -> Self::DlIter;
    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter;
}

pub fn interpret_f3d_display_list(source: &impl F3DSource, backend: &mut impl RenderBackend) {
    let mut state = State::default();
    state.interpret(source, backend, source.root_dl(), 0);
}

#[derive(Debug, Default)]
struct State {
    alpha_dither: AlphaDither,
    color_dither: ColorDither,
    combine_key: bool,
    texture_convert: TextureConvert,
    texture_filter: TextureFilter,
    texture_lut: TextureLUT,
    texture_lod: bool,
    texture_detail: TextureDetail,
    texture_persp: bool,
    cycle_type: CycleType,
    pipeline_mode: PipelineMode,
    alpha_compare: AlphaCompare,
    depth_source: DepthSource,
    render_mode: RenderMode,
    persp_normalize: u16,
}

impl State {
    fn interpret<S: F3DSource>(
        &mut self,
        source: &S,
        backend: &mut impl RenderBackend,
        dl: S::DlIter,
        indent: usize,
    ) {
        let indent_str = "  ".repeat(indent);
        for cmd in dl {
            // if !matches!(cmd, F3DCommand::Unknown { .. }) {
            //     eprintln!("{}{:?}", indent_str, cmd);
            // }
            match cmd {
                F3DCommand::NoOp => {}
                F3DCommand::Rsp(cmd) => match cmd {
                    // SPCommand::Matrix {
                    //     matrix,
                    //     mode,
                    //     op,
                    //     push,
                    // } => todo!(),
                    // SPCommand::Viewport(_) => todo!(),
                    // SPCommand::Light { light, n } => todo!(),
                    SPCommand::Vertex { v, n, v0 } => {
                        eprintln!("{:?}", cmd);
                    }
                    SPCommand::DisplayList(ptr) => {
                        let child_dl = source.read_dl(ptr);
                        self.interpret(source, backend, child_dl, indent + 1);
                    }
                    SPCommand::BranchList(ptr) => {
                        let child_dl = source.read_dl(ptr);
                        self.interpret(source, backend, child_dl, indent + 1);
                        break;
                    }
                    SPCommand::OneTriangle { v0, v1, v2, flag } => {
                        eprintln!("{:?}", cmd);
                    }
                    // SPCommand::PopMatrix(_) => todo!(),
                    // SPCommand::NumLights(_) => todo!(),
                    // SPCommand::Segment { seg, base } => todo!(),
                    // SPCommand::FogFactor { mul, offset } => todo!(),
                    // SPCommand::Texture {
                    //     sc,
                    //     tc,
                    //     level,
                    //     tile,
                    //     on,
                    // } => todo!(),
                    SPCommand::EndDisplayList => break,
                    // SPCommand::SetGeometryMode(_) => todo!(),
                    // SPCommand::ClearGeometryMode(_) => todo!(),
                    _ => {} //unimplemented!("{:?}", cmd),
                },
                F3DCommand::Rdp(cmd) => match cmd {
                    DPCommand::SetAlphaDither(v) => self.alpha_dither = v,
                    DPCommand::SetColorDither(v) => self.color_dither = v,
                    DPCommand::SetCombineKey(v) => self.combine_key = v,
                    DPCommand::SetTextureConvert(v) => self.texture_convert = v,
                    DPCommand::SetTextureFilter(v) => self.texture_filter = v,
                    DPCommand::SetTextureLUT(v) => self.texture_lut = v,
                    DPCommand::SetTextureLOD(v) => self.texture_lod = v,
                    DPCommand::SetTextureDetail(v) => self.texture_detail = v,
                    DPCommand::SetTexturePersp(v) => self.texture_persp = v,
                    DPCommand::SetCycleType(v) => self.cycle_type = v,
                    DPCommand::PipelineMode(v) => self.pipeline_mode = v,
                    DPCommand::SetAlphaCompare(v) => self.alpha_compare = v,
                    DPCommand::SetDepthSource(v) => self.depth_source = v,
                    DPCommand::SetRenderMode(v) => self.render_mode = v,
                    DPCommand::PerspNormalize(v) => self.persp_normalize = v,
                    // DPCommand::SetColorImage(_) => todo!(),
                    // DPCommand::SetDepthImage(_) => todo!(),
                    // DPCommand::SetTextureImage(_) => todo!(),
                    // DPCommand::SetCombineMode(_) => todo!(),
                    // DPCommand::SetEnvColor(_) => todo!(),
                    // DPCommand::SetPrimColor(_) => todo!(),
                    // DPCommand::SetBlendColor(_) => todo!(),
                    // DPCommand::SetFogColor(_) => todo!(),
                    // DPCommand::SetFillColor(_) => todo!(),
                    // DPCommand::FillRectangle(_) => todo!(),
                    // DPCommand::SetTile(_, _) => todo!(),
                    // DPCommand::LoadTile(_, _) => todo!(),
                    // DPCommand::LoadBlock(_, _) => todo!(),
                    // DPCommand::SetTileSize(_, _) => todo!(),
                    // DPCommand::LoadTLUTCmd(_, _) => todo!(),
                    // DPCommand::SetOtherMode(_) => todo!(),
                    // DPCommand::SetPrimDepth(_) => todo!(),
                    DPCommand::SetScissor(_mode, rect) => {
                        if rect.lrx > rect.ulx && rect.lry > rect.uly {
                            backend.set_scissor(
                                rect.ulx.into_inner() as i32,
                                rect.uly.into_inner() as i32,
                                (rect.lrx - rect.ulx).into_inner() as i32,
                                (rect.lry - rect.uly).into_inner() as i32,
                            );
                        }
                    }
                    // DPCommand::SetConvert(_) => todo!(),
                    // DPCommand::SetKeyR(_) => todo!(),
                    // DPCommand::SetKeyGB(_) => todo!(),
                    DPCommand::FullSync => {}
                    DPCommand::TileSync => {}
                    DPCommand::PipeSync => {}
                    DPCommand::LoadSync => {}
                    // DPCommand::TextureRectangleFlip(_) => todo!(),
                    // DPCommand::TextureRectangle(_) => todo!(),
                    _ => {} //unimplemented!("{:?}", cmd),
                },
                F3DCommand::Unknown(_) => {
                    eprintln!("{}{:?}", indent_str, cmd);
                }
            }
        }
    }
}
