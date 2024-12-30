use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use glam::vec3;
use rand::{thread_rng, Rng};
use rodio::Sink;

use crate::vertex;

pub struct SinkExtrapolator {
    pub sink: Sink,
    last_returned_duration: Duration,
    assumed_start: Instant,
}
impl SinkExtrapolator {
    pub fn new(sink: Sink) -> Self {
        let pos = sink.get_pos();
        Self {
            sink,
            last_returned_duration: pos,
            assumed_start: Instant::now() - pos,
        }
    }

    pub fn get_pos(&mut self) -> Duration {
        let pos = self.sink.get_pos();
        if pos != self.last_returned_duration {
            self.assumed_start = Instant::now() - pos;
            self.last_returned_duration = pos;
        }
        Instant::now() - self.assumed_start
    }
}

pub fn generate_quad_plane(width: u32, depth: u32) -> (Vec<vertex::CommonVertex>, Vec<u32>) {
    (
        (0..(width + 1) * (depth + 1))
            .map(|i| {
                let x = (i % (width + 1)) as f32;
                let z = (i / (width + 1)) as f32;

                vertex::CommonVertex {
                    position: (vec3(x, 0.0, z)
                        - vec3(width as f32 / 2.0, 0.0, depth as f32 / 2.0))
                    .into(),
                    color: [1.0; 4],
                    uv: [x / width as f32, z / depth as f32],
                }
            })
            .collect(),
        (0..width * depth)
            .flat_map(|i| {
                let x = i % width;
                let y = i / width;

                [
                    x + y * (width + 1),
                    x + y * (width + 1) + 1,
                    x + y * (width + 1) + width + 1,
                    x + y * (width + 1) + width + 2,
                    x + y * (width + 1) + width + 1,
                    x + y * (width + 1) + 1,
                ]
            })
            .collect(),
    )
}
