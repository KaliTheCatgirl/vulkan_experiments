use std::f32::consts::TAU;

use glam::{vec3, vec4, Vec3, Vec4};
use rand::{thread_rng, Rng};

pub type Color = Vec4;

pub fn sinebow(t: f32) -> Color {
    vec4(
        (t * TAU).cos() * 0.5 + 0.5,
        ((t + 2.0 / 3.0) * TAU).cos() * 0.5 + 0.5,
        ((t + 1.0 / 3.0) * TAU).cos() * 0.5 + 0.5,
        1.0,
    )
}
pub const WHITE: Color = Color::splat(1.0);
pub const BLACK: Color = vec4(0.0, 0.0, 0.0, 1.0);
pub const TRANSPARENT: Color = vec4(0.0, 0.0, 0.0, 0.0);
pub fn randcolor() -> Color {
    vec4(
        thread_rng().gen(),
        thread_rng().gen(),
        thread_rng().gen(),
        1.0,
    )
}
pub fn hueflip(color: Color) -> Color {
    let max_component = color.x.max(color.y).max(color.z);
    let min_component = color.x.min(color.y).min(color.z);
    (Vec3::splat(max_component + min_component) - vec3(color.x, color.y, color.z)).extend(color.w)
}
