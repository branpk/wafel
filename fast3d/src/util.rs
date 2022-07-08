//! Utilities for working with Fast3D-related data.

#![allow(missing_docs)]

use core::fmt;
use std::{mem, ops};

use bytemuck::cast_slice_mut;

use crate::{
    decode::{F3DWrapMode, MatrixOp},
    f3d_render_data::{TextureData, WrapMode},
    interpret::F3DMemory,
};

impl From<F3DWrapMode> for WrapMode {
    fn from(m: F3DWrapMode) -> Self {
        if m.clamp {
            WrapMode::Clamp
        } else if m.mirror {
            WrapMode::MirrorRepeat
        } else {
            WrapMode::Repeat
        }
    }
}

#[derive(Clone, Default)]
pub struct Matrixf(pub [[f32; 4]; 4]);

impl Matrixf {
    pub fn identity() -> Self {
        Self([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Note: roll is in radians
    pub fn look_at(from: [f32; 3], to: [f32; 3], roll: f32) -> Self {
        let mut dx = to[0] - from[0];
        let mut dz = to[2] - from[2];

        let inv_length = -1.0 / (dx * dx + dz * dz).sqrt();
        dx *= inv_length;
        dz *= inv_length;

        let mut y_col_y = roll.cos();
        let mut x_col_y = roll.sin() * dz;
        let mut z_col_y = -roll.sin() * dx;

        let mut x_col_z = to[0] - from[0];
        let mut y_col_z = to[1] - from[1];
        let mut z_col_z = to[2] - from[2];

        let inv_length = -1.0 / (x_col_z * x_col_z + y_col_z * y_col_z + z_col_z * z_col_z).sqrt();
        x_col_z *= inv_length;
        y_col_z *= inv_length;
        z_col_z *= inv_length;

        let mut x_col_x = y_col_y * z_col_z - z_col_y * y_col_z;
        let mut y_col_x = z_col_y * x_col_z - x_col_y * z_col_z;
        let mut z_col_x = x_col_y * y_col_z - y_col_y * x_col_z;

        let inv_length = 1.0 / (x_col_x * x_col_x + y_col_x * y_col_x + z_col_x * z_col_x).sqrt();
        x_col_x *= inv_length;
        y_col_x *= inv_length;
        z_col_x *= inv_length;

        x_col_y = y_col_z * z_col_x - z_col_z * y_col_x;
        y_col_y = z_col_z * x_col_x - x_col_z * z_col_x;
        z_col_y = x_col_z * y_col_x - y_col_z * x_col_x;

        let inv_length = 1.0 / (x_col_y * x_col_y + y_col_y * y_col_y + z_col_y * z_col_y).sqrt();
        x_col_y *= inv_length;
        y_col_y *= inv_length;
        z_col_y *= inv_length;

        let mut mtx = [[0.0; 4]; 4];

        mtx[0][0] = x_col_x;
        mtx[0][1] = y_col_x;
        mtx[0][2] = z_col_x;
        mtx[0][3] = -(from[0] * x_col_x + from[1] * y_col_x + from[2] * z_col_x);

        mtx[1][0] = x_col_y;
        mtx[1][1] = y_col_y;
        mtx[1][2] = z_col_y;
        mtx[1][3] = -(from[0] * x_col_y + from[1] * y_col_y + from[2] * z_col_y);

        mtx[2][0] = x_col_z;
        mtx[2][1] = y_col_z;
        mtx[2][2] = z_col_z;
        mtx[2][3] = -(from[0] * x_col_z + from[1] * y_col_z + from[2] * z_col_z);

        mtx[3][0] = 0.0;
        mtx[3][1] = 0.0;
        mtx[3][2] = 0.0;
        mtx[3][3] = 1.0;

        Self(mtx)
    }

    /// fov_y is in radians
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let mut mtx = Self::identity();

        let y_scale = (fov_y / 2.0).cos() / (fov_y / 2.0).sin();
        mtx.0[0][0] = y_scale / aspect;
        mtx.0[1][1] = y_scale;
        mtx.0[2][2] = (near + far) / (near - far);
        mtx.0[3][2] = -1.0;
        mtx.0[2][3] = 2.0 * near * far / (near - far);
        mtx.0[3][3] = 0.0;

        mtx
    }

    /// Rotate xyz by c in radians and translate by b.
    pub fn rotate_xyz_and_translate(b: [f32; 3], c: [f32; 3]) -> Self {
        let sx = c[0].sin();
        let cx = c[0].cos();

        let sy = c[1].sin();
        let cy = c[1].cos();

        let sz = c[2].sin();
        let cz = c[2].cos();

        let mut mtx = Matrixf::default();

        mtx.0[0][0] = cy * cz;
        mtx.0[1][0] = cy * sz;
        mtx.0[2][0] = -sy;
        mtx.0[3][0] = 0.0;

        mtx.0[0][1] = sx * sy * cz - cx * sz;
        mtx.0[1][1] = sx * sy * sz + cx * cz;
        mtx.0[2][1] = sx * cy;
        mtx.0[3][1] = 0.0;

        mtx.0[0][2] = cx * sy * cz + sx * sz;
        mtx.0[1][2] = cx * sy * sz - sx * cz;
        mtx.0[2][2] = cx * cy;
        mtx.0[3][2] = 0.0;

        mtx.0[0][3] = b[0];
        mtx.0[1][3] = b[1];
        mtx.0[2][3] = b[2];
        mtx.0[3][3] = 1.0;

        mtx
    }

    pub fn from_fixed(m: &[i32]) -> Self {
        assert_eq!(m.len(), 16, "incorrect fixed point matrix size");
        let mut r = Self::default();
        for i in [0, 2] {
            for j in 0..4 {
                let int_part = m[j * 2 + i / 2] as u32;
                let frac_part = m[8 + j * 2 + i / 2] as u32;
                r.0[i][j] = ((int_part & 0xFFFF0000) | (frac_part >> 16)) as i32 as f32 / 65536.0;
                r.0[i + 1][j] = ((int_part << 16) | (frac_part & 0xFFFF)) as i32 as f32 / 65536.0;
            }
        }
        r
    }

    pub fn to_fixed(&self) -> Vec<i32> {
        let mut r = vec![0; 16];
        for i in [0, 2] {
            for j in 0..4 {
                let v1 = (self.0[i][j] * 65536.0) as i32 as u32;
                let v2 = (self.0[i + 1][j] * 65536.0) as i32 as u32;
                let frac_part = (v1 << 16) | (v2 & 0xFFFF);
                let int_part = (v1 & 0xFFFF0000) | (v2 >> 16);
                r[j * 2 + i / 2] = int_part as i32;
                r[8 + j * 2 + i / 2] = frac_part as i32;
            }
        }
        r
    }

    pub fn transpose(&self) -> Self {
        let mut r = Self::default();
        for i in 0..4 {
            for j in 0..4 {
                r.0[i][j] = self.0[j][i];
            }
        }
        r
    }

    pub fn invert_isometry(&self) -> Matrixf {
        let mut rotation = self.clone();
        rotation.0[0][3] = 0.0;
        rotation.0[1][3] = 0.0;
        rotation.0[2][3] = 0.0;

        let inv_rotation = rotation.transpose();

        let translate = [self.0[0][3], self.0[1][3], self.0[2][3], 0.0];
        let new_translate = scalar_mul(&inv_rotation * translate, -1.0);

        let mut inv = inv_rotation;
        inv.0[0][3] = new_translate[0];
        inv.0[1][3] = new_translate[1];
        inv.0[2][3] = new_translate[2];
        inv
    }
}

impl fmt::Debug for Matrixf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Matrixf [")?;
        for i in 0..4 {
            write!(f, "  [ ")?;
            for j in 0..4 {
                write!(f, "\t{:.3} ", self.0[i][j])?;
            }
            writeln!(f, "\t]")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl ops::Mul<&Matrixf> for &Matrixf {
    type Output = Matrixf;

    fn mul(self, rhs: &Matrixf) -> Self::Output {
        let mut out = Matrixf::default();
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    out.0[i][j] += self.0[i][k] * rhs.0[k][j];
                }
            }
        }
        out
    }
}

impl ops::Mul<[f32; 4]> for &Matrixf {
    type Output = [f32; 4];

    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        let mut out = [0.0; 4];
        for i in 0..4 {
            for k in 0..4 {
                out[i] += self.0[i][k] * rhs[k];
            }
        }
        out
    }
}

pub fn read_matrix<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    offset: usize,
) -> Result<Vec<i32>, M::Error> {
    let mut m = vec![0; 16];
    memory.read_u32(cast_slice_mut(&mut m), ptr, offset)?;
    Ok(m)
}

pub fn normalize(v: [f32; 4]) -> [f32; 4] {
    let mag = dot(v, v).sqrt();
    if mag == 0.0 {
        v
    } else {
        scalar_mul(v, 1.0 / mag)
    }
}

pub fn dot(v: [f32; 4], w: [f32; 4]) -> f32 {
    v[0] * w[0] + v[1] * w[1] + v[2] * w[2] + v[3] * w[3]
}

pub fn scalar_mul(v: [f32; 4], s: f32) -> [f32; 4] {
    [v[0] * s, v[1] * s, v[2] * s, v[3] * s]
}

#[derive(Debug)]
pub struct MatrixState {
    pub stack: Vec<Matrixf>,
    pub cur: Matrixf,
}

impl Default for MatrixState {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            cur: Matrixf::identity(),
        }
    }
}

impl MatrixState {
    pub fn execute(&mut self, m: Matrixf, op: MatrixOp, push: bool) {
        if push {
            self.stack.push(self.cur.clone());
        }
        match op {
            MatrixOp::Load => self.cur = m,
            MatrixOp::Mul => self.cur = &self.cur * &m,
        }
    }

    pub fn push_mul(&mut self, m: Matrixf) {
        self.execute(m, MatrixOp::Mul, true);
    }

    pub fn pop(&mut self) {
        self.cur = self.stack.pop().expect("popMatrix without push");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Viewport {
    pub scale: [i16; 4],
    pub trans: [i16; 4],
}

pub fn read_viewport<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    offset: usize,
) -> Result<Viewport, M::Error> {
    let mut v = Viewport::default();
    memory.read_u16(cast_slice_mut(&mut v.scale), ptr, offset)?;
    memory.read_u16(cast_slice_mut(&mut v.trans), ptr, offset + 8)?;
    Ok(v)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Vertex {
    pub pos: [i16; 3],
    pub padding: u16,
    pub uv: [i16; 2],
    pub cn: [u8; 4],
}

pub fn read_vertices<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    offset: usize,
    count: usize,
) -> Result<Vec<Vertex>, M::Error> {
    let stride = mem::size_of::<Vertex>();
    let mut vs = Vec::new();
    for i in 0..count {
        let mut v = Vertex::default();
        let voffset = offset + i * stride;
        memory.read_u16(cast_slice_mut(&mut v.pos), ptr, voffset)?;
        memory.read_u16(cast_slice_mut(&mut v.uv), ptr, voffset + 8)?;
        memory.read_u8(cast_slice_mut(&mut v.cn), ptr, voffset + 12)?;
        vs.push(v);
    }
    Ok(vs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Light {
    pub color: [u8; 3],
    pub pad1: u8,
    pub color_copy: [u8; 3],
    pub pad2: u8,
    pub dir: [i8; 3],
    pub pad3: u8,
}

pub fn read_light<M: F3DMemory>(memory: &M, ptr: M::Ptr) -> Result<Light, M::Error> {
    let mut light = Light::default();
    memory.read_u8(&mut light.color, ptr, 0)?;
    memory.read_u8(&mut light.color_copy, ptr, 4)?;
    memory.read_u8(cast_slice_mut(&mut light.dir), ptr, 8)?;
    Ok(light)
}

impl TextureData {
    #[track_caller]
    pub fn new(width: u32, height: u32, rgba8: Vec<u8>) -> Self {
        assert!(4 * width * height <= rgba8.len() as u32);
        Self {
            width,
            height,
            rgba8,
        }
    }

    #[allow(dead_code)]
    pub fn dbg_constant(r: u8, g: u8, b: u8, a: u8) -> Self {
        let width = 32;
        let height = 32;
        let mut data = Vec::new();
        for _ in 0..width * height {
            data.extend(&[r, g, b, a]);
        }
        Self::new(width, height, data)
    }

    #[allow(dead_code)]
    pub fn dbg_gradient() -> Self {
        let width = 32;
        let height = 32;
        let mut data = Vec::new();
        for i in 0..height {
            for j in 0..width {
                let u = i as f32 / height as f32;
                let v = j as f32 / width as f32;
                let r = 0.0;
                let g = u;
                let b = v;
                data.extend(&[(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]);
            }
        }
        Self::new(width, height, data)
    }
}

pub fn read_rgba16<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> Result<TextureData, M::Error> {
    let mut rgba16_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut rgba16_data, ptr, 0)?;

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(2 * size_bytes as usize);

    for i in 0..size_bytes / 2 {
        let i0 = (2 * i) as usize;
        let rgba16 = ((rgba16_data[i0] as u16) << 8) | rgba16_data[i0 + 1] as u16;
        let rgba32 = rgba_16_to_32(rgba16);
        rgba32_data.extend(&rgba32);
    }

    Ok(TextureData::new(
        line_size_bytes / 2,
        size_bytes / line_size_bytes,
        rgba32_data,
    ))
}

pub fn rgba_16_to_32(rgba16: u16) -> [u8; 4] {
    [
        (((rgba16 >> 8) & 0xF8) as u32 * 255 / 0xF8) as u8,
        (((rgba16 >> 3) & 0xF8) as u32 * 255 / 0xF8) as u8,
        (((rgba16 << 2) & 0xF8) as u32 * 255 / 0xF8) as u8,
        (rgba16 & 0x1) as u8 * 255,
    ]
}

pub fn read_ia16<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> Result<TextureData, M::Error> {
    let mut ia16_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut ia16_data, ptr, 0)?;

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(2 * size_bytes as usize);

    for i in 0..size_bytes / 2 {
        let i0 = (2 * i) as usize;
        let intensity = ia16_data[i0] as u8;
        let alpha = ia16_data[i0 + 1] as u8;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha]);
    }

    Ok(TextureData::new(
        line_size_bytes / 2,
        size_bytes / line_size_bytes,
        rgba32_data,
    ))
}

pub fn read_ia8<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> Result<TextureData, M::Error> {
    let mut ia8_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut ia8_data, ptr, 0)?;

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(4 * size_bytes as usize);

    for i in 0..size_bytes {
        let i0 = i as usize;
        let intensity = (ia8_data[i0] >> 4) * 0x11;
        let alpha = (ia8_data[i0] & 0xF) * 0x11;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha]);
    }

    Ok(TextureData::new(
        line_size_bytes,
        size_bytes / line_size_bytes,
        rgba32_data,
    ))
}

pub fn read_ia4<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> Result<TextureData, M::Error> {
    let mut ia4_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut ia4_data, ptr, 0)?;

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(8 * size_bytes as usize);

    for i in 0..2 * size_bytes {
        let v = (ia4_data[(i / 2) as usize] >> ((1 - i % 2) * 4)) & 0xF;
        let intensity = (v >> 1) * 0x24;
        let alpha = v & 0x1;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha * 255]);
    }

    Ok(TextureData::new(
        line_size_bytes * 2,
        size_bytes / line_size_bytes,
        rgba32_data,
    ))
}
