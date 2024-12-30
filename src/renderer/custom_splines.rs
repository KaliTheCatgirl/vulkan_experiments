use std::ops::{Add, Mul};

use num::Float;

/// A collection of functions that can be passed into [`sample`]'s `weight` parameter.
pub mod example_weight_functions {
    use std::f32::consts::PI;

    /// A sine-based weighting function.
    pub fn sine(x: f32) -> f32 {
        if !(-1.0..=1.0).contains(&x) {
            return 0.0;
        }
        (x * PI).cos() * 0.5 + 0.5
    }

    /// A sharp, linear weighting function.
    pub fn linear(x: f32) -> f32 {
        (1.0 - x.abs()).max(0.0)
    }
}

/// Samples a weight-based local spline.
/// 
/// `weight` is a function with a few specifications to make it work ideally:
/// The function should expect to be queried within \[-1, 1\]. Outside of this range, it should return 0.
/// 
/// The area under the curve from \[-1, 1\] should sum to 1. Formally, the definite integral from -1 to 1 of `weight(x)dx` should return 1.
/// 
/// `weight(-1)` and `weight(1)` should return 0, and `weight(0)` should return 1.
/// 
/// For any x value in the range \[0, 1\], `weight(x) + weight(x - 1)` should return 1.
/// 
/// 
/// If `points` is empty, returns `P::default()`.
pub fn raw_sample<F: Float, P: Default + Mul<F, Output = P> + Add<P, Output = P> + Clone>(x: F, weight: fn(F) -> F, points: &[P], local_reach: F) -> P {
    let mut out = P::default();
    if points.is_empty() { return out; }
    let x_i = x.floor().to_isize().unwrap();
    let range = local_reach.ceil().to_isize().unwrap();
    let local_reach_recip = local_reach.recip();
    for i in x_i - range..=x_i + range + 1 {
        let index = i.clamp(0, points.len() as isize - 1) as usize;
        out = out + points[index].clone() * weight((x - F::from(i).unwrap()) / local_reach) * local_reach_recip;
    }
    out
}
