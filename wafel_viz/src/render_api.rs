#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShaderId(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct ShaderInfo {
    pub num_inputs: u8,
    pub used_textures: [bool; 2],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CCFeatures {
    pub c: [[ShaderItem; 4]; 2],
    pub opt_alpha: bool,
    pub opt_fog: bool,
    pub opt_texture_edge: bool,
    pub opt_noise: bool,
    pub used_textures: [bool; 2],
    pub num_inputs: u32,
    pub do_single: [bool; 2],
    pub do_multiply: [bool; 2],
    pub do_mix: [bool; 2],
    pub color_alpha_same: bool,
}

impl CCFeatures {
    pub fn uses_textures(&self) -> bool {
        self.used_textures[0] || self.used_textures[1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaderItem {
    Zero,
    Input1,
    Input2,
    Input3,
    Input4,
    Texel0,
    Texel0A,
    Texel1,
}

impl Default for ShaderItem {
    fn default() -> Self {
        Self::Zero
    }
}

impl ShaderItem {
    pub fn from_index(v: u32) -> Self {
        match v {
            0 => ShaderItem::Zero,
            1 => ShaderItem::Input1,
            2 => ShaderItem::Input2,
            3 => ShaderItem::Input3,
            4 => ShaderItem::Input4,
            5 => ShaderItem::Texel0,
            6 => ShaderItem::Texel0A,
            7 => ShaderItem::Texel1,
            _ => panic!("invalid shader item index: {}", v),
        }
    }

    pub fn to_index(self) -> u32 {
        match self {
            ShaderItem::Zero => 0,
            ShaderItem::Input1 => 1,
            ShaderItem::Input2 => 2,
            ShaderItem::Input3 => 3,
            ShaderItem::Input4 => 4,
            ShaderItem::Texel0 => 5,
            ShaderItem::Texel0A => 6,
            ShaderItem::Texel1 => 7,
        }
    }
}

pub fn decode_shader_id(shader_id: u32) -> CCFeatures {
    // Copied from src/pc/gfx/gfx_cc.c (to avoid needing to call it with DllGameMemory)

    use ShaderItem::*;

    let mut cc_features = CCFeatures::default();

    for i in 0..4 {
        cc_features.c[0][i] = ShaderItem::from_index((shader_id >> (i * 3)) & 7);
        cc_features.c[1][i] = ShaderItem::from_index((shader_id >> (12 + i * 3)) & 7);
    }

    cc_features.opt_alpha = (shader_id & (1 << 24)) != 0;
    cc_features.opt_fog = (shader_id & (1 << 25)) != 0;
    cc_features.opt_texture_edge = (shader_id & (1 << 26)) != 0;
    cc_features.opt_noise = (shader_id & (1 << 27)) != 0;

    cc_features.used_textures[0] = false;
    cc_features.used_textures[1] = false;
    cc_features.num_inputs = 0;

    for i in 0..2 {
        for j in 0..4 {
            if cc_features.c[i][j] >= Input1 && cc_features.c[i][j] <= Input4 {
                let index = cc_features.c[i][j].to_index();
                if index > cc_features.num_inputs {
                    cc_features.num_inputs = index;
                }
            }
            if cc_features.c[i][j] == Texel0 || cc_features.c[i][j] == Texel0A {
                cc_features.used_textures[0] = true;
            }
            if cc_features.c[i][j] == Texel1 {
                cc_features.used_textures[1] = true;
            }
        }
    }
    //
    cc_features.do_single[0] = cc_features.c[0][2] == Zero;
    cc_features.do_single[1] = cc_features.c[1][2] == Zero;
    cc_features.do_multiply[0] = cc_features.c[0][1] == Zero && cc_features.c[0][3] == Zero;
    cc_features.do_multiply[1] = cc_features.c[1][1] == Zero && cc_features.c[1][3] == Zero;
    cc_features.do_mix[0] = cc_features.c[0][1] == cc_features.c[0][3];
    cc_features.do_mix[1] = cc_features.c[1][1] == cc_features.c[1][3];
    cc_features.color_alpha_same = (shader_id & 0xfff) == ((shader_id >> 12) & 0xfff);

    cc_features
}

pub fn encode_shader_id(cc_features: CCFeatures) -> u32 {
    let mut shader_id = 0;

    for i in 0..4 {
        shader_id |= cc_features.c[0][i].to_index() << (i * 3);
        shader_id |= cc_features.c[1][i].to_index() << (12 + i * 3);
    }

    if cc_features.opt_alpha {
        shader_id |= 1 << 24;
    }
    if cc_features.opt_fog {
        shader_id |= 1 << 25;
    }
    if cc_features.opt_texture_edge {
        shader_id |= 1 << 26;
    }
    if cc_features.opt_noise {
        shader_id |= 1 << 27;
    }

    shader_id
}
