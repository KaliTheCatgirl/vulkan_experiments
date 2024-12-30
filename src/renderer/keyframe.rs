use std::f32::consts::{E, PI};

use crate::ext::LerpExt;

pub enum EasingType {
    Constant,
    Linear,
    Sine,
    Power(f32),
    Exponential(f32),
}
impl EasingType {
    pub fn sample(&self, t: f32) -> f32 {
        match self {
            EasingType::Constant => 0.0,
            EasingType::Linear => t,
            EasingType::Sine => -(t * PI * 0.5).cos() * 0.5 + 0.5,
            EasingType::Power(power) => t.powf(*power),
            EasingType::Exponential(v) => {
                // f(x) == x^2 as v approaches zero
                if *v == 0.0 {
                    return t * t;
                }

                // b *could* be passed in raw but, through pure experimentation,
                // i found that it makes more sense to have the base be an exponential of `v`
                let b = E.powf(*v);

                // this complicated formula just makes sure that `f(0) == 0` and `f'(0) == 0` while still ensuring that `f(1) == 1`.
                // see https://www.desmos.com/calculator/qt9mfjnoix for a visualisation
                let common_denom = b - 1.0;
                let derivative_factor = b.ln() / (common_denom);

                let exp_value = (b.powf(t) - 1.0) / common_denom;

                (exp_value - derivative_factor * t) / (1.0 - derivative_factor)
            }
        }
    }
}

pub enum EasingDirection {
    In,
    Out,
    InOut,
}
impl EasingDirection {
    pub fn get_sample_point(&self, t: f32) -> f32 {
        match self {
            EasingDirection::In => t,
            EasingDirection::Out => 1.0 - t,
            EasingDirection::InOut => 1.0 - 2.0 * (t - 0.5).abs(),
        }
    }
    pub fn get_ease_factor(&self, t: f32) -> f32 {
        match self {
            EasingDirection::In => 1.0,
            EasingDirection::Out => -1.0,
            EasingDirection::InOut => {
                if t < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
    pub fn get_ease_offset(&self, t: f32) -> f32 {
        match self {
            EasingDirection::In => 0.0,
            EasingDirection::Out => 1.0,
            EasingDirection::InOut => {
                if t < 0.5 {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }
}

pub struct Easing {
    pub etype: EasingType,
    pub direction: EasingDirection,
}
impl Easing {
    pub fn apply(&self, t: f32) -> f32 {
        self.direction.get_ease_factor(t) * self.etype.sample(self.direction.get_sample_point(t))
            + self.direction.get_ease_offset(t)
    }
}

pub struct KeyframeSequence<T> {
    samples: Vec<(f32, T, Easing)>,
}
impl<T> KeyframeSequence<T> {
    pub fn sort(&mut self) {
        self.samples.sort_unstable_by(|l, r| l.0.total_cmp(&r.0));
    }
    pub fn sample(&mut self, t: f32) -> T
    where
        T: Default + Clone + LerpExt<f32>,
    {
        if self.samples.is_empty() {
            return T::default();
        }
        if t <= self.samples[0].0 {}
        if t >= self.samples.last().unwrap().0 {
            return self.samples.last().unwrap().1.clone();
        }

        let second_index = match self.samples.binary_search_by(|i| i.0.total_cmp(&t)) {
            Ok(i) => i,
            Err(i) => i,
        };
        if second_index == 0 {
            return self.samples[0].1.clone();
        }
        if second_index >= self.samples.len() {
            return self.samples.last().unwrap().1.clone();
        }
        let first_index = second_index - 1;

        let first = &self.samples[first_index];
        let second = &self.samples[second_index];

        let offset = first.0;
        let distance = second.0 - first.0;
        // ranges from 0-1
        let lerp_factor = (t - offset) / distance;

        first
            .1
            .clone()
            .lerp(second.1.clone(), first.2.apply(lerp_factor))
    }
}
