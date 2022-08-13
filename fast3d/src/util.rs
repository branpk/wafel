//! Utilities for working with Fast3D-related data.

#![allow(missing_docs)]

use core::fmt;
use std::{num::Wrapping, ops};

use bytemuck::cast_slice_mut;

use crate::{
    cmd::{F3DWrapMode, MatrixOp},
    f3d_render_data::{TextureData, WrapMode},
    interpret::F3DMemory,
    trig_tables::{ARCTAN_TABLE, SINE_TABLE},
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

#[allow(clippy::excessive_precision)]
pub const M_PI_32: f32 = 3.14159265358979323846264338327950288;
#[allow(clippy::excessive_precision)]
pub const M_PI_64: f64 = 3.14159265358979323846264338327950288;

// TODO: sinf, cosf, sqrtf?

pub type Angle = Wrapping<i16>;

pub fn sins(x: Angle) -> f32 {
    SINE_TABLE[(x.0 as u16 >> 4) as usize]
}

pub fn coss(x: Angle) -> f32 {
    SINE_TABLE[(x.0 as u16 >> 4) as usize + 0x400]
}

fn atan2_lookup(y: f32, x: f32) -> Angle {
    if x == 0.0 {
        Wrapping(ARCTAN_TABLE[0])
    } else {
        Wrapping(ARCTAN_TABLE[(y / x * 1024.0 + 0.5) as i32 as usize])
    }
}

pub fn atan2s(mut x: f32, mut y: f32) -> Angle {
    if y >= 0.0 {
        if x >= 0.0 {
            if x >= y {
                atan2_lookup(y, x)
            } else {
                Wrapping(0x4000) - atan2_lookup(x, y)
            }
        } else {
            x = -x;
            if x < y {
                Wrapping(0x4000) + atan2_lookup(x, y)
            } else {
                Wrapping(-0x8000) - atan2_lookup(y, x)
            }
        }
    } else {
        y = -y;
        if x < 0.0 {
            x = -x;
            if x >= y {
                Wrapping(-0x8000) + atan2_lookup(y, x)
            } else {
                Wrapping(-0x4000) - atan2_lookup(x, y)
            }
        } else if x < y {
            Wrapping(-0x4000) + atan2_lookup(x, y)
        } else {
            -atan2_lookup(y, x)
        }
    }
}

pub fn atan2f(x: f32, y: f32) -> f32 {
    atan2s(x, y).0 as f32 * M_PI_32 / 0x8000 as f32
}

#[derive(Clone, PartialEq, Default)]
pub struct Matrixf {
    pub cols: [[f32; 4]; 4],
}

impl Matrixf {
    pub fn identity() -> Self {
        Self {
            cols: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn from_rows(rows: [[f32; 4]; 4]) -> Self {
        Self { cols: rows }.transpose()
    }

    pub fn from_rows_vec3(rows: [[f32; 3]; 3]) -> Self {
        Self::from_rows([
            [rows[0][0], rows[0][1], rows[0][2], 0.0],
            [rows[1][0], rows[1][1], rows[1][2], 0.0],
            [rows[2][0], rows[2][1], rows[2][2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn look_at(from: [f32; 3], to: [f32; 3], roll: Angle) -> Self {
        let mut dx = to[0] - from[0];
        let mut dz = to[2] - from[2];

        let inv_length = -1.0 / (dx * dx + dz * dz).sqrt();
        dx *= inv_length;
        dz *= inv_length;

        let mut y_col_y = coss(roll);
        let mut x_col_y = sins(roll) * dz;
        let mut z_col_y = -sins(roll) * dx;

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

        let mut mtx = Matrixf::default();

        mtx.cols[0][0] = x_col_x;
        mtx.cols[1][0] = y_col_x;
        mtx.cols[2][0] = z_col_x;
        mtx.cols[3][0] = -(from[0] * x_col_x + from[1] * y_col_x + from[2] * z_col_x);

        mtx.cols[0][1] = x_col_y;
        mtx.cols[1][1] = y_col_y;
        mtx.cols[2][1] = z_col_y;
        mtx.cols[3][1] = -(from[0] * x_col_y + from[1] * y_col_y + from[2] * z_col_y);

        mtx.cols[0][2] = x_col_z;
        mtx.cols[1][2] = y_col_z;
        mtx.cols[2][2] = z_col_z;
        mtx.cols[3][2] = -(from[0] * x_col_z + from[1] * y_col_z + from[2] * z_col_z);

        mtx.cols[0][3] = 0.0;
        mtx.cols[1][3] = 0.0;
        mtx.cols[2][3] = 0.0;
        mtx.cols[3][3] = 1.0;

        mtx
    }

    /// fov_y is in radians
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let mut mtx = Self::identity();

        let y_scale = (fov_y / 2.0).cos() / (fov_y / 2.0).sin();
        mtx.cols[0][0] = y_scale / aspect;
        mtx.cols[1][1] = y_scale;
        mtx.cols[2][2] = (near + far) / (near - far);
        mtx.cols[2][3] = -1.0;
        mtx.cols[3][2] = 2.0 * near * far / (near - far);
        mtx.cols[3][3] = 0.0;

        mtx
    }

    //     void guOrthoF(float m[4][4], float left, float right, float bottom, float top, float near, float far,
    //         float scale) {
    // int row;
    // int col;
    // guMtxIdentF(m);
    // m[0][0] = 2 / (right - left);
    // m[1][1] = 2 / (top - bottom);
    // m[2][2] = -2 / (far - near);
    // m[3][0] = -(right + left) / (right - left);
    // m[3][1] = -(top + bottom) / (top - bottom);
    // m[3][2] = -(far + near) / (far - near);
    // m[3][3] = 1;
    // for (row = 0; row < 4; row++) {
    //   for (col = 0; col < 4; col++) {
    //       m[row][col] *= scale;
    //   }
    // }
    // }

    pub fn ortho(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
        scale: f32,
    ) -> Self {
        let mut m = Self::identity();

        m.cols[0][0] = 2.0 / (right - left);
        m.cols[1][1] = 2.0 / (top - bottom);
        m.cols[2][2] = -2.0 / (far - near);
        m.cols[3][0] = -(right + left) / (right - left);
        m.cols[3][1] = -(top + bottom) / (top - bottom);
        m.cols[3][2] = -(far + near) / (far - near);
        m.cols[3][3] = 1.0;

        for i in 0..4 {
            for j in 0..4 {
                m.cols[i][j] *= scale;
            }
        }
        m
    }

    pub fn translate(b: [f32; 3]) -> Self {
        let mut mtx = Matrixf::identity();

        mtx.cols[3][0] = b[0];
        mtx.cols[3][1] = b[1];
        mtx.cols[3][2] = b[2];
        mtx.cols[3][3] = 1.0;

        mtx
    }

    pub fn rotate_xyz_and_translate(b: [f32; 3], c: [Angle; 3]) -> Self {
        let sx = sins(c[0]);
        let cx = coss(c[0]);

        let sy = sins(c[1]);
        let cy = coss(c[1]);

        let sz = sins(c[2]);
        let cz = coss(c[2]);

        let mut mtx = Matrixf::default();

        mtx.cols[0][0] = cy * cz;
        mtx.cols[0][1] = cy * sz;
        mtx.cols[0][2] = -sy;
        mtx.cols[0][3] = 0.0;

        mtx.cols[1][0] = sx * sy * cz - cx * sz;
        mtx.cols[1][1] = sx * sy * sz + cx * cz;
        mtx.cols[1][2] = sx * cy;
        mtx.cols[1][3] = 0.0;

        mtx.cols[2][0] = cx * sy * cz + sx * sz;
        mtx.cols[2][1] = cx * sy * sz - sx * cz;
        mtx.cols[2][2] = cx * cy;
        mtx.cols[2][3] = 0.0;

        mtx.cols[3][0] = b[0];
        mtx.cols[3][1] = b[1];
        mtx.cols[3][2] = b[2];
        mtx.cols[3][3] = 1.0;

        mtx
    }

    pub fn rotate_zxy_and_translate(b: [f32; 3], c: [Angle; 3]) -> Self {
        let sx = sins(c[0]);
        let cx = coss(c[0]);

        let sy = sins(c[1]);
        let cy = coss(c[1]);

        let sz = sins(c[2]);
        let cz = coss(c[2]);

        let mut mtx = Matrixf::default();

        mtx.cols[0][0] = cy * cz + sx * sy * sz;
        mtx.cols[1][0] = -cy * sz + sx * sy * cz;
        mtx.cols[2][0] = cx * sy;
        mtx.cols[3][0] = b[0];

        mtx.cols[0][1] = cx * sz;
        mtx.cols[1][1] = cx * cz;
        mtx.cols[2][1] = -sx;
        mtx.cols[3][1] = b[1];

        mtx.cols[0][2] = -sy * cz + sx * cy * sz;
        mtx.cols[1][2] = sy * sz + sx * cy * cz;
        mtx.cols[2][2] = cx * cy;
        mtx.cols[3][2] = b[2];

        mtx.cols[0][3] = 0.0;
        mtx.cols[1][3] = 0.0;
        mtx.cols[2][3] = 0.0;
        mtx.cols[3][3] = 1.0;

        mtx
    }

    pub fn rotate_xy(angle: Angle) -> Self {
        let mut temp = Self::identity();
        temp.cols[0][0] = coss(angle);
        temp.cols[0][1] = sins(angle);
        temp.cols[1][0] = -temp.cols[0][1];
        temp.cols[1][1] = temp.cols[0][0];
        temp
    }

    pub fn scale_vec3f(s: [f32; 3]) -> Self {
        let mut mtx = Self::identity();
        mtx.cols[0][0] = s[0];
        mtx.cols[1][1] = s[1];
        mtx.cols[2][2] = s[2];
        mtx
    }

    pub fn billboard(mtx: &Self, position: [f32; 3], angle: Angle) -> Self {
        let mut dest = Self::default();

        dest.cols[0][0] = coss(angle);
        dest.cols[0][1] = sins(angle);
        dest.cols[0][2] = 0.0;
        dest.cols[0][3] = 0.0;

        dest.cols[1][0] = -dest.cols[0][1];
        dest.cols[1][1] = dest.cols[0][0];
        dest.cols[1][2] = 0.0;
        dest.cols[1][3] = 0.0;

        dest.cols[2][0] = 0.0;
        dest.cols[2][1] = 0.0;
        dest.cols[2][2] = 1.0;
        dest.cols[2][3] = 0.0;

        dest.cols[3][0] = mtx.cols[0][0] * position[0]
            + mtx.cols[1][0] * position[1]
            + mtx.cols[2][0] * position[2]
            + mtx.cols[3][0];
        dest.cols[3][1] = mtx.cols[0][1] * position[0]
            + mtx.cols[1][1] * position[1]
            + mtx.cols[2][1] * position[2]
            + mtx.cols[3][1];
        dest.cols[3][2] = mtx.cols[0][2] * position[0]
            + mtx.cols[1][2] * position[1]
            + mtx.cols[2][2] * position[2]
            + mtx.cols[3][2];
        dest.cols[3][3] = 1.0;

        dest
    }

    pub fn pos_from_transform_mtx(&self, cam_mtx: &Matrixf) -> [f32; 3] {
        let cam_x = cam_mtx.cols[3][0] * cam_mtx.cols[0][0]
            + cam_mtx.cols[3][1] * cam_mtx.cols[0][1]
            + cam_mtx.cols[3][2] * cam_mtx.cols[0][2];
        let cam_y = cam_mtx.cols[3][0] * cam_mtx.cols[1][0]
            + cam_mtx.cols[3][1] * cam_mtx.cols[1][1]
            + cam_mtx.cols[3][2] * cam_mtx.cols[1][2];
        let cam_z = cam_mtx.cols[3][0] * cam_mtx.cols[2][0]
            + cam_mtx.cols[3][1] * cam_mtx.cols[2][1]
            + cam_mtx.cols[3][2] * cam_mtx.cols[2][2];

        let mut dest = [0.0; 3];
        dest[0] = self.cols[3][0] * cam_mtx.cols[0][0]
            + self.cols[3][1] * cam_mtx.cols[0][1]
            + self.cols[3][2] * cam_mtx.cols[0][2]
            - cam_x;
        dest[1] = self.cols[3][0] * cam_mtx.cols[1][0]
            + self.cols[3][1] * cam_mtx.cols[1][1]
            + self.cols[3][2] * cam_mtx.cols[1][2]
            - cam_y;
        dest[2] = self.cols[3][0] * cam_mtx.cols[2][0]
            + self.cols[3][1] * cam_mtx.cols[2][1]
            + self.cols[3][2] * cam_mtx.cols[2][2]
            - cam_z;

        dest
    }

    pub fn from_fixed(m: &[i32]) -> Self {
        assert_eq!(m.len(), 16, "incorrect fixed point matrix size");
        let mut r = Self::default();
        for j in 0..4 {
            for i in [0, 2] {
                let int_part = m[j * 2 + i / 2] as u32;
                let frac_part = m[8 + j * 2 + i / 2] as u32;
                r.cols[j][i] =
                    ((int_part & 0xFFFF0000) | (frac_part >> 16)) as i32 as f32 / 65536.0;
                r.cols[j][i + 1] =
                    ((int_part << 16) | (frac_part & 0xFFFF)) as i32 as f32 / 65536.0;
            }
        }
        r
    }

    pub fn to_fixed(&self) -> Vec<i32> {
        let mut r = vec![0; 16];
        for j in 0..4 {
            for i in [0, 2] {
                let v1 = (self.cols[j][i] * 65536.0) as i32 as u32;
                let v2 = (self.cols[j][i + 1] * 65536.0) as i32 as u32;
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
        for j in 0..4 {
            for i in 0..4 {
                r.cols[j][i] = self.cols[i][j];
            }
        }
        r
    }

    pub fn invert_isometry(&self) -> Matrixf {
        let mut rotation = self.clone();
        rotation.cols[3][0] = 0.0;
        rotation.cols[3][1] = 0.0;
        rotation.cols[3][2] = 0.0;

        let inv_rotation = rotation.transpose();

        let translate = [self.cols[3][0], self.cols[3][1], self.cols[3][2], 0.0];
        let new_translate = scalar_mul(&inv_rotation * translate, -1.0);

        let mut inv = inv_rotation;
        inv.cols[3][0] = new_translate[0];
        inv.cols[3][1] = new_translate[1];
        inv.cols[3][2] = new_translate[2];
        inv
    }
}

impl fmt::Debug for Matrixf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Matrixf [")?;
        for i in 0..4 {
            write!(f, "  [ ")?;
            for j in 0..4 {
                write!(f, "\t{:.3} ", self.cols[j][i])?;
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
                    out.cols[j][i] += self.cols[k][i] * rhs.cols[j][k];
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
                out[i] += self.cols[k][i] * rhs[k];
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

pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let n = normalize([v[0], v[1], v[2], 0.0]);
    [n[0], n[1], n[2]]
}

pub fn dot(v: [f32; 4], w: [f32; 4]) -> f32 {
    v[0] * w[0] + v[1] * w[1] + v[2] * w[2] + v[3] * w[3]
}

pub fn dot3(v: [f32; 3], w: [f32; 3]) -> f32 {
    v[0] * w[0] + v[1] * w[1] + v[2] * w[2]
}

pub fn cross(v: [f32; 3], w: [f32; 3]) -> [f32; 3] {
    [
        v[1] * w[2] - v[2] * w[1],
        v[2] * w[0] - v[0] * w[2],
        v[0] * w[1] - v[1] * w[0],
    ]
}

pub fn scalar_mul(v: [f32; 4], s: f32) -> [f32; 4] {
    [v[0] * s, v[1] * s, v[2] * s, v[3] * s]
}

#[derive(Debug)]
pub struct MatrixStack {
    pub stack: Vec<Matrixf>,
    pub cur: Matrixf,
}

impl Default for MatrixStack {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            cur: Matrixf::identity(),
        }
    }
}

impl MatrixStack {
    pub fn execute(&mut self, m: &Matrixf, op: MatrixOp, push: bool) {
        if push {
            self.stack.push(self.cur.clone());
        }
        match op {
            MatrixOp::Load => self.cur = m.clone(),
            MatrixOp::Mul => self.cur = &self.cur * m,
        }
    }

    pub fn push_mul(&mut self, m: &Matrixf) {
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
    pub uv: [i16; 2],
    pub cn: [u8; 4],
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
