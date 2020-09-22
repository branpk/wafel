use std::{f32::consts::PI, num::Wrapping};

use lazy_static::lazy_static;

// TODO: Accurate trig functions

/// A wrapping 16 bit integer representing an angle.
pub type Angle = Wrapping<i16>;

fn sins(x: Angle) -> f32 {
    (x.0 as f32 / 0x8000 as f32 * PI).sin()
}

fn coss(x: Angle) -> f32 {
    (x.0 as f32 / 0x8000 as f32 * PI).cos()
}

fn atan2s(x: f32, y: f32) -> Angle {
    Wrapping(((y.atan2(x) / PI) * 0x8000 as f32) as i16)
}

/// The joystick's state after removing the dead zone and capping the magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AdjustedStick {
    /// Adjusted stick x.
    pub x: f32,
    /// Adjusted stick y.
    pub y: f32,
    /// Adjusted magnitude, [0, 64].
    pub mag: f32,
}

/// The joystick's state as stored in the mario struct.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IntendedStick {
    /// Intended yaw in world space.
    pub yaw: Angle,
    /// Intended magnitude, normally [0, 32].
    pub mag: f32,
}

/// In-game calculation converting raw stick inputs to adjusted.
pub fn stick_raw_to_adjusted(raw_stick_x: i16, raw_stick_y: i16) -> AdjustedStick {
    let mut x = 0.0;
    let mut y = 0.0;

    if raw_stick_x <= -8 {
        x = (raw_stick_x + 6) as f32;
    }
    if raw_stick_x >= 8 {
        x = (raw_stick_x - 6) as f32;
    }
    if raw_stick_y <= -8 {
        y = (raw_stick_y + 6) as f32;
    }
    if raw_stick_y >= 8 {
        y = (raw_stick_y - 6) as f32;
    }

    let mut mag = (x * x + y * y).sqrt();

    if mag > 64.0 {
        x *= 64.0 / mag;
        y *= 64.0 / mag;
        mag = 64.0;
    }

    AdjustedStick { x, y, mag }
}

/// In-game calculation converting adjusted stick to intended.
pub fn stick_adjusted_to_intended(
    stick: AdjustedStick,
    face_yaw: Angle,
    camera_yaw: Angle,
    squished: bool,
) -> IntendedStick {
    let mag = ((stick.mag / 64.0) * (stick.mag / 64.0)) * 64.0;

    let intended_mag = if !squished { mag / 2.0 } else { mag / 8.0 };

    let intended_yaw = if intended_mag > 0.0 {
        atan2s(-stick.y, stick.x) + camera_yaw
    } else {
        face_yaw
    };

    IntendedStick {
        yaw: intended_yaw,
        mag: intended_mag,
    }
}

lazy_static! {
    /// A table mapping adjusted yaws to raw stick values that achieve that yaw
    /// with maximum magnitude.
    static ref ADJUSTED_YAW_TABLE: Vec<Option<(i16, i16)>> = {
        let mut table = vec![None; 0x10000];
        for raw_stick_x in -128..128 {
            for raw_stick_y in -128..128 {
                let adjusted = stick_raw_to_adjusted(raw_stick_x, raw_stick_y);
                if adjusted.mag >= 64.0 {
                    let adjusted_yaw = atan2s(-adjusted.y, adjusted.x);
                    let index = adjusted_yaw.0 as u16 as usize;
                    table[index] = Some((raw_stick_x, raw_stick_y));
                }
            }
        }
        table
    };
}

/// Given a range of adjusted yaws (inclusive-exclusive), find raw stick values
/// that achieve an adjusted yaw in that range with maximum magnitude.
fn stick_adjusted_yaw_range_to_raw(start_yaw: Angle, end_yaw: Angle) -> Option<(i16, i16)> {
    let mut yaw = start_yaw;
    while yaw != end_yaw {
        let index = yaw.0 as u16 as usize;
        let raw = ADJUSTED_YAW_TABLE[index];

        if raw.is_some() {
            return raw;
        }

        yaw += Wrapping(1);
    }

    None
}

/// Return the start yaw of the HAU containing `angle`, relative to 0.
fn truncate_to_hau(angle: Angle) -> Angle {
    let hau = (angle.0 as u16) >> 4;
    Wrapping((hau << 4) as i16)
}

/// Find a raw stick value with maximum adjusted magnitude and whose adjusted yaw
// is in the nearest HAU to `target_yaw` relative to `relative_to` as possible.
fn stick_adjusted_yaw_to_raw(target_yaw: Angle, relative_to: Angle) -> (i16, i16) {
    let target_hau_start_yaw = truncate_to_hau(target_yaw - relative_to) + relative_to;

    let mut distance = 0;
    loop {
        let start_yaw = target_hau_start_yaw + Wrapping(16 * distance);
        let end_yaw = start_yaw + Wrapping(16);

        if let Some(raw) = stick_adjusted_yaw_range_to_raw(start_yaw, end_yaw) {
            return raw;
        }

        distance = -distance;
        if distance >= 0 {
            distance += 1;
        }
    }
}

/// Return a raw stick value whose adjusted stick is approximately equal to `adjusted`.
///
/// May not be accurate for `adjusted.mag >= 64`.
fn stick_adjusted_to_raw_approx(adjusted: AdjustedStick) -> (i16, i16) {
    let mut raw_stick_x = 0;
    let mut raw_stick_y = 0;

    if adjusted.x <= -2.0 {
        raw_stick_x = (adjusted.x - 6.0) as i16;
    }
    if adjusted.x >= 2.0 {
        raw_stick_x = (adjusted.x + 6.0) as i16;
    }
    if adjusted.y <= -2.0 {
        raw_stick_y = (adjusted.y - 6.0) as i16;
    }
    if adjusted.y >= 2.0 {
        raw_stick_y = (adjusted.y + 6.0) as i16;
    }

    (raw_stick_x, raw_stick_y)
}

/// Return an adjusted stick whose intended stick is approximately equal to `intended`.
///
/// May not be accurate if the resulting magnitude is >= 64.
fn stick_intended_to_adjusted_approx(
    intended: IntendedStick,
    _face_yaw: Angle,
    camera_yaw: Angle,
    squished: bool,
) -> AdjustedStick {
    let mag = if !squished {
        intended.mag * 2.0
    } else {
        intended.mag * 8.0
    };

    let adjusted_mag = (mag / 64.0).sqrt() * 64.0;

    AdjustedStick {
        x: (sins(intended.yaw - camera_yaw) * adjusted_mag).round(),
        y: (-coss(intended.yaw - camera_yaw) * adjusted_mag).round(),
        mag: adjusted_mag,
    }
}

/// Return the raw stick value whose adjusted stick is closest to the given
/// adjusted inputs, based on Euclidean distance.
pub fn stick_adjusted_to_raw_euclidean(
    target_adjusted_x: f32,
    target_adjusted_y: f32,
) -> (i16, i16) {
    let mut nearest = (f32::MAX, None);
    for raw_x in -128..128 {
        for raw_y in -128..128 {
            let adjusted = stick_raw_to_adjusted(raw_x, raw_y);
            let dx = adjusted.x - target_adjusted_x;
            let dy = adjusted.y - target_adjusted_y;
            let distance = dx * dx + dy * dy;

            if distance < nearest.0 {
                nearest = (distance, Some((raw_x, raw_y)));
            }
        }
    }
    nearest.1.unwrap()
}

/// Find a raw josytick value that approximately maps to the given intended inputs.
///
/// If the given input has maximum magnitude, then try to produce a raw input in a nearby
/// HAU of the intended yaw (relative to `relative_to`).
/// If it does not have maximum magnitude, then return a raw joystick that maps to a nearby
/// adjusted input, without worrying about exact angle or magnitude.
pub fn stick_intended_to_raw_heuristic(
    intended: IntendedStick,
    face_yaw: Angle,
    camera_yaw: Angle,
    squished: bool,
    relative_to: Angle,
) -> (i16, i16) {
    let adjusted = stick_intended_to_adjusted_approx(intended, face_yaw, camera_yaw, squished);

    if adjusted.mag >= 64.0 {
        stick_adjusted_yaw_to_raw(intended.yaw - camera_yaw, relative_to - camera_yaw)
    } else {
        stick_adjusted_to_raw_approx(adjusted)
    }
}
