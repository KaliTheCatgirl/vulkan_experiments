use std::{ops::Deref, time::{Duration, Instant}};

pub struct Stopwatch(Instant);
impl Stopwatch {
    pub fn new() -> Self {
        Self(Instant::now())
    }
    pub fn get(&self) -> Duration {
        Instant::now() - self.0
    }
}
