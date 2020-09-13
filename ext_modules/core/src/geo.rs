pub type Matrix4f = nalgebra::Matrix4<f32>;
pub type Point3f = nalgebra::Point3<f32>;
pub type Vector3f = nalgebra::Vector3<f32>;
pub type Vector4f = nalgebra::Vector4<f32>;

pub fn direction_to_pitch_yaw(dir: &Vector3f) -> (f32, f32) {
    let xz = (dir.x * dir.x + dir.z * dir.z).sqrt();
    let pitch = f32::atan2(dir.y, xz);
    let yaw = f32::atan2(dir.x, dir.z);
    (pitch, yaw)
}

pub fn pitch_yaw_to_direction(pitch: f32, yaw: f32) -> Vector3f {
    Vector3f::new(
        pitch.cos() * yaw.sin(),
        pitch.sin(),
        pitch.cos() * yaw.cos(),
    )
}
