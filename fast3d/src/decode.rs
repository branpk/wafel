//! Fast3D command decoding.
//!
//! This module provides two methods for decoding commands:
//! - [decode_f3d_command]
//! - [decode_f3d_display_list]
//!
//! Both of these transform [RawF3DCommand]s into [F3DCommand]s.
//! Since some commands have multiple parts, the former returns a [DecodeResult],
//! which may require additional raw commands to complete.
//!
//! Currently these functions panic on invalid commands.
//!
//! Note: this module is not complete and may have errors.

#![allow(missing_docs)]

use std::fmt;

use crate::cmd::*;

/// A raw Fast3D command for decoding.
///
/// Normally this consists of two 32 bit words, with the latter sometimes representing a
/// segmented memory address. To support 64 bit machines, a third `w1_ptr` field is
/// included.
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

/// The result of decoding a raw command.
///
/// In most cases, one raw command maps to one output command, so you can call
/// [DecodeResult::unwrap] to get the final result.
/// Longer commands can be completed using [DecodeResult::is_complete] and
/// [DecodeResult::next].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeResult<Ptr> {
    Complete(F3DCommand<Ptr>),
    TextureRectangle1 {
        flip: bool,
        rect: Rectangle<u32>,
        tile: TileIndex,
    },
    TextureRectangle2 {
        flip: bool,
        rect: Rectangle<u32>,
        tile: TileIndex,
        s: u16,
        t: u16,
    },
}

impl<Ptr> DecodeResult<Ptr> {
    /// Returns true if the command is fully decoded and [DecodeResult::unwrap] can be called.
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete(_))
    }

    /// Returns the decoded command.
    ///
    /// # Panics
    /// Panics if [DecodeResult::is_complete] is false.
    #[track_caller]
    pub fn unwrap(self) -> F3DCommand<Ptr> {
        match self {
            Self::Complete(command) => command,
            _ => panic!("unwrap() called on partial result"),
        }
    }

    /// If the command is not yet complete, continue decoding using the given data.
    ///
    /// # Panics
    /// Panics if [DecodeResult::is_complete] is true.
    #[track_caller]
    pub fn next(self, cmd_cont: RawF3DCommand<Ptr>) -> Self {
        let w1 = cmd_cont.w1;
        let cmd = cmd_cont.w0 >> 24;

        if ![0xB4, 0xB3, 0xB2, 0xB1].contains(&cmd) {
            return Self::Complete(F3DCommand::Unknown(cmd_cont));
        }

        match self {
            Self::Complete(_) => panic!("next() called on complete result"),
            Self::TextureRectangle1 { flip, rect, tile } => Self::TextureRectangle2 {
                flip,
                rect,
                tile,
                s: (w1 >> 16) as u16,
                t: w1 as u16,
            },
            Self::TextureRectangle2 {
                flip,
                rect,
                tile,
                s,
                t,
            } => {
                let tex_rect = TextureRectangle {
                    rect,
                    tile,
                    s,
                    t,
                    dsdx: (w1 >> 16) as u16,
                    dtdy: w1 as u16,
                };
                let cmd = if flip {
                    F3DCommand::DPTextureRectangleFlip(tex_rect)
                } else {
                    F3DCommand::DPTextureRectangle(tex_rect)
                };
                Self::Complete(cmd)
            }
        }
    }
}

/// Decodes a raw Fast3D command.
///
/// This returns a [DecodeResult] since the command may be incomplete.
pub fn decode_f3d_command<Ptr: Copy>(raw_command: RawF3DCommand<Ptr>) -> DecodeResult<Ptr> {
    use F3DCommand::*;

    let w0 = raw_command.w0;
    let w1 = raw_command.w1;
    let w1p = raw_command.w1_ptr;
    let cmd = w0 >> 24;

    DecodeResult::Complete(match cmd {
        0x00 => NoOp,

        // DMA commands
        0x01 => {
            let p = ((w0 >> 16) & 0xFF) as u8;
            SPMatrix {
                matrix: w1p,
                mode: (p & 0x01).try_into().unwrap(),
                op: (p & 0x02).try_into().unwrap(),
                push: p & 0x04 != 0,
            }
        }
        0x03 => {
            let p = (w0 >> 16) & 0xFF;
            match p {
                0x80 => SPViewport(w1p),
                0x86..=0x94 => SPLight {
                    light: w1p,
                    n: (p - 0x86) / 2 + 1,
                },
                _ => Unknown(raw_command),
            }
        }
        0x04 => SPVertex {
            v: w1p,
            n: ((w0 >> 20) & 0xF) + 1,
            v0: (w0 >> 16) & 0xF,
        },
        0x06 => {
            let p = (w0 >> 16) & 0xFF;
            match p {
                0 => SPDisplayList(w1p),
                1 => SPBranchList(w1p),
                _ => Unknown(raw_command),
            }
        }

        // IMMEDIATE commands
        0xBF => SPOneTriangle {
            v0: ((w1 >> 16) & 0xFF) / 10,
            v1: ((w1 >> 8) & 0xFF) / 10,
            v2: (w1 & 0xFF) / 10,
            flag: w1 >> 24,
        },
        0xBD => SPPopMatrix(((w1 & 0x01) as u8).try_into().unwrap()),
        0xBC => {
            let index = w0 & 0xFF;
            match index {
                2 => SPNumLights((w1 - 0x80000000) / 0x20 - 1),
                6 => SPSegment {
                    seg: ((w0 >> 8) & 0xFFFF) / 4,
                    base: w1p,
                },
                8 => SPFogFactor {
                    mul: ((w1 >> 16) & 0xFFFF) as i16,
                    offset: (w1 & 0xFFFF) as i16,
                },
                _ => Unknown(raw_command),
            }
        }
        0xBB => SPTexture {
            sc: (w1 >> 16) & 0xFFFF,
            tc: w1 & 0xFFFF,
            level: (w0 >> 11) & 0x7,
            tile: (w0 >> 8) & 0x7,
            on: (w0 & 0xFF) != 0,
        },
        0xBA => {
            let shift = (w0 >> 8) & 0xFF;
            let data = (w1 >> shift) as u8;
            match shift {
                4 => DPSetAlphaDither(data.try_into().unwrap()),
                6 => DPSetColorDither(data.try_into().unwrap()),
                8 => DPSetCombineKey(data != 0),
                9 => DPSetTextureConvert(data.try_into().unwrap()),
                12 => DPSetTextureFilter(data.try_into().unwrap()),
                14 => DPSetTextureLUT(data.try_into().unwrap()),
                16 => DPSetTextureLOD(data != 0),
                17 => DPSetTextureDetail(data.try_into().unwrap()),
                19 => DPSetTexturePersp(data != 0),
                20 => DPSetCycleType(data.try_into().unwrap()),
                23 => DPPipelineMode(data.try_into().unwrap()),
                _ => Unknown(raw_command),
            }
        }
        0xB9 => {
            let shift = (w0 >> 8) & 0xFF;
            match shift {
                0 => DPSetAlphaCompare(((w1 >> shift) as u8).try_into().unwrap()),
                2 => DPSetDepthSource(((w1 >> shift) as u8).try_into().unwrap()),
                3 => DPSetRenderMode(w1.try_into().unwrap()),
                _ => Unknown(raw_command),
            }
        }
        0xB8 => SPEndDisplayList,
        0xB7 => SPSetGeometryMode(GeometryModes::from_bits_truncate(w1)),
        0xB6 => SPClearGeometryMode(GeometryModes::from_bits_truncate(w1)),
        0xB4 => DPPerspNormalize(w1 as u16),
        // RDPHALF_X, not expected here
        0xB3 | 0xB2 | 0xB1 => Unknown(raw_command),

        // RDP commands
        0xFF => DPSetColorImage(Image {
            fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
            size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
            width: (w0 & 0xFFF) + 1,
            img: w1p,
        }),
        0xFE => DPSetDepthImage(w1p),
        0xFD => DPSetTextureImage(Image {
            fmt: (((w0 >> 21) & 0x7) as u8).try_into().unwrap(),
            size: (((w0 >> 19) & 0x3) as u8).try_into().unwrap(),
            width: (w0 & 0xFFF) + 1,
            img: w1p,
        }),
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
            DPSetCombineMode(CombineMode {
                color1: cc1.try_into().unwrap(),
                alpha1: ac1.try_into().unwrap(),
                color2: cc2.try_into().unwrap(),
                alpha2: ac2.try_into().unwrap(),
            })
        }
        0xFB => DPSetEnvColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        }),
        0xFA => DPSetPrimColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        }),
        0xF9 => DPSetBlendColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        }),
        0xF8 => DPSetFogColor(Rgba32 {
            r: (w1 >> 24) as u8,
            g: (w1 >> 16) as u8,
            b: (w1 >> 8) as u8,
            a: w1 as u8,
        }),
        0xF7 => DPSetFillColor([
            FillColor((w1 >> 16) as u16),
            FillColor((w1 & 0xFFFF) as u16),
        ]),
        0xF6 => DPFillRectangle(Rectangle {
            ulx: (w1 >> 14) & 0x3FF,
            uly: (w1 >> 2) & 0x3FF,
            lrx: (w0 >> 14) & 0x3FF,
            lry: (w0 >> 2) & 0x3FF,
        }),
        0xF5 => DPSetTile(
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
        ),
        0xF4 => DPLoadTile(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TileSize {
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                lrt: w1 & 0xFFF,
            },
        ),
        0xF3 => DPLoadBlock(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TextureBlock {
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                dxt: w1 & 0xFFF,
            },
        ),
        0xF2 => DPSetTileSize(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            TileSize {
                uls: (w0 >> 12) & 0xFFF,
                ult: w0 & 0xFFF,
                lrs: (w1 >> 12) & 0xFFF,
                lrt: w1 & 0xFFF,
            },
        ),
        0xF0 => DPLoadTLUTCmd(
            TileIndex(((w1 >> 24) & 0x7) as u8),
            ((w1 >> 14) & 0x3FF) as u32,
        ),
        0xEF => DPSetOtherMode(Unimplemented { w0, w1 }),
        0xEE => DPSetPrimDepth(PrimDepth {
            z: (w1 >> 16) as u16,
            dz: (w1 & 0xFFFF) as u16,
        }),
        0xED => DPSetScissor(
            (((w1 >> 24) & 0xFF) as u8).try_into().unwrap(),
            Rectangle {
                ulx: ((w0 >> 12) & 0xFFF) as u16,
                uly: (w0 & 0xFFF) as u16,
                lrx: ((w1 >> 12) & 0xFFF) as u16,
                lry: (w1 & 0xFFF) as u16,
            },
        ),
        0xEC => DPSetConvert(Unimplemented { w0, w1 }),
        0xEB => DPSetKeyR(Unimplemented { w0, w1 }),
        0xEA => DPSetKeyGB(Unimplemented { w0, w1 }),
        0xE9 => DPFullSync,
        0xE8 => DPTileSync,
        0xE7 => DPPipeSync,
        0xE6 => DPLoadSync,
        0xE5 => {
            return DecodeResult::TextureRectangle1 {
                flip: true,
                rect: Rectangle {
                    ulx: (w1 >> 12) & 0xFFF,
                    uly: w1 & 0xFFF,
                    lrx: (w0 >> 12) & 0xFFF,
                    lry: w0 & 0xFFF,
                },
                tile: TileIndex(((w1 >> 24) & 0x7) as u8),
            };
        }
        0xE4 => {
            return DecodeResult::TextureRectangle1 {
                flip: false,
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

/// Decodes a stream of [RawF3DCommand]s into a stream of [F3DCommand]s.
pub fn decode_f3d_display_list<Ptr, E, I: Iterator<Item = Result<RawF3DCommand<Ptr>, E>>>(
    raw_dl: I,
) -> F3DCommandIter<I> {
    F3DCommandIter {
        raw_dl,
        ended: false,
    }
}

#[derive(Debug)]
pub struct F3DCommandIter<I> {
    raw_dl: I,
    ended: bool,
}

impl<Ptr, E, I> F3DCommandIter<I>
where
    Ptr: Copy,
    I: Iterator<Item = Result<RawF3DCommand<Ptr>, E>>,
{
    fn next_impl(
        &mut self,
        raw_command: Result<RawF3DCommand<Ptr>, E>,
    ) -> Result<F3DCommand<Ptr>, E> {
        let raw_command = raw_command?;
        let mut result = decode_f3d_command(raw_command);
        while !result.is_complete() {
            match self.raw_dl.next() {
                Some(raw_command_cont) => {
                    result = result.next(raw_command_cont?);
                }
                None => {
                    return Ok(F3DCommand::Unknown(raw_command));
                }
            }
        }
        Ok(result.unwrap())
    }
}

impl<Ptr, E, I> Iterator for F3DCommandIter<I>
where
    Ptr: Copy,
    I: Iterator<Item = Result<RawF3DCommand<Ptr>, E>>,
{
    type Item = Result<F3DCommand<Ptr>, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }

        let raw_cmd = self.raw_dl.next()?;
        let cmd = self.next_impl(raw_cmd);

        if matches!(
            cmd,
            Ok(F3DCommand::SPEndDisplayList | F3DCommand::SPBranchList(..))
        ) {
            self.ended = true;
        }

        Some(cmd)
    }
}
