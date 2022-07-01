use core::fmt;

use ordered_float::NotNan;

use crate::{f3d_decode::*, render_api::RenderBackend};

pub trait F3DSource {
    type Ptr: fmt::Debug + Copy;
    type DlIter: Iterator<Item = F3DCommand<Self::Ptr>>;

    fn root_dl(&self) -> Self::DlIter;
    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter;

    fn read_viewport(&self, ptr: Self::Ptr) -> Viewport;
    fn read_vertices(&self, ptr: Self::Ptr, offset: usize, count: usize) -> Vec<Vertex>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Viewport {
    pub scale: [i16; 4],
    pub trans: [i16; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Vertex {
    pub pos: [i16; 3],
    pub padding: u16,
    pub uv: [i16; 2],
    pub cn: [u8; 4],
}

pub fn interpret_f3d_display_list(source: &impl F3DSource, backend: &mut impl RenderBackend) {
    let mut state = State::default();
    state.interpret(source, backend, source.root_dl(), 0);
    state.flush(backend);
}

#[derive(Debug, Default)]
struct State {
    viewport: Viewport,

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

    scissor: (ScissorMode, Rectangle<NotNan<f32>>),

    vertices: Vec<Vertex>,
    vertex_buffer: Vec<f32>,
    vertex_buffer_num_tris: usize,
}

impl State {
    fn vertex(&self, index: u32) -> Vertex {
        *self
            .vertices
            .get(index as usize)
            .expect("invalid vertex index")
    }

    fn flush(&mut self, backend: &mut impl RenderBackend) {
        if self.vertex_buffer_num_tris > 0 {
            backend.draw_triangles(&self.vertex_buffer, self.vertex_buffer_num_tris);
            self.vertex_buffer.clear();
            self.vertex_buffer_num_tris = 0;
            // std::process::exit(0);
        }
    }

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
                    SPCommand::Viewport(ptr) => {
                        let viewport = source.read_viewport(ptr);
                        if self.viewport != viewport {
                            self.flush(backend);
                            self.viewport = viewport;

                            let width = 2.0 * viewport.scale[0] as f32 / 4.0;
                            let height = 2.0 * viewport.scale[1] as f32 / 4.0;
                            let x = (viewport.trans[0] as f32 / 4.0) - width / 2.0;
                            let y = 240.0 - ((viewport.trans[1] as f32 / 4.0) + height / 2.0);

                            backend.set_viewport(x as i32, y as i32, width as i32, height as i32);
                        }
                    }
                    // SPCommand::Light { light, n } => todo!(),
                    SPCommand::Vertex { v, n, v0 } => {
                        self.vertices = source.read_vertices(v, v0 as usize, n as usize);
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
                        // TODO: Use flag for flat shading
                        let vertices = [self.vertex(v0), self.vertex(v1), self.vertex(v2)];
                        for v in vertices {
                            // TODO: transform vertices + fixed point
                            self.vertex_buffer.push(v.pos[0] as f32);
                            self.vertex_buffer.push(v.pos[1] as f32);
                            self.vertex_buffer.push(v.pos[2] as f32);
                        }
                        self.vertex_buffer_num_tris += 1;
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
                    DPCommand::SetAlphaDither(v) => {
                        if self.alpha_dither != v {
                            self.flush(backend);
                            self.alpha_dither = v;
                        }
                    }
                    DPCommand::SetColorDither(v) => {
                        if self.color_dither != v {
                            self.flush(backend);
                            self.color_dither = v;
                        }
                    }
                    DPCommand::SetCombineKey(v) => {
                        if self.combine_key != v {
                            self.flush(backend);
                            self.combine_key = v;
                        }
                    }
                    DPCommand::SetTextureConvert(v) => {
                        if self.texture_convert != v {
                            self.flush(backend);
                            self.texture_convert = v;
                        }
                    }
                    DPCommand::SetTextureFilter(v) => {
                        if self.texture_filter != v {
                            self.flush(backend);
                            self.texture_filter = v;
                        }
                    }
                    DPCommand::SetTextureLUT(v) => {
                        if self.texture_lut != v {
                            self.flush(backend);
                            self.texture_lut = v;
                        }
                    }
                    DPCommand::SetTextureLOD(v) => {
                        if self.texture_lod != v {
                            self.flush(backend);
                            self.texture_lod = v;
                        }
                    }
                    DPCommand::SetTextureDetail(v) => {
                        if self.texture_detail != v {
                            self.flush(backend);
                            self.texture_detail = v;
                        }
                    }
                    DPCommand::SetTexturePersp(v) => {
                        if self.texture_persp != v {
                            self.flush(backend);
                            self.texture_persp = v;
                        }
                    }
                    DPCommand::SetCycleType(v) => {
                        if self.cycle_type != v {
                            self.flush(backend);
                            self.cycle_type = v;
                        }
                    }
                    DPCommand::PipelineMode(v) => {
                        if self.pipeline_mode != v {
                            self.flush(backend);
                            self.pipeline_mode = v;
                        }
                    }
                    DPCommand::SetAlphaCompare(v) => {
                        if self.alpha_compare != v {
                            self.flush(backend);
                            self.alpha_compare = v;
                        }
                    }
                    DPCommand::SetDepthSource(v) => {
                        if self.depth_source != v {
                            self.flush(backend);
                            self.depth_source = v;
                        }
                    }
                    DPCommand::SetRenderMode(v) => {
                        if self.render_mode != v {
                            self.flush(backend);
                            self.render_mode = v;
                        }
                    }
                    DPCommand::PerspNormalize(v) => {
                        if self.persp_normalize != v {
                            self.flush(backend);
                            self.persp_normalize = v;
                        }
                    }
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
                    DPCommand::SetScissor(mode, rect) => {
                        assert!(rect.lrx > rect.ulx && rect.lry > rect.uly);
                        let new_scissor = (mode, rect);
                        if self.scissor != new_scissor {
                            self.flush(backend);
                            self.scissor = new_scissor;
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
