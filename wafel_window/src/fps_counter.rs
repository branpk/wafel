use std::time;

#[derive(Debug, Clone)]
pub struct FpsCounter {
    frames: u32,
    last_time: time::Instant,
    measurement: (u32, time::Duration),
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frames: 0,
            last_time: time::Instant::now(),
            measurement: (0, time::Duration::ZERO),
        }
    }

    pub fn end_frame(&mut self) {
        self.frames += 1;
        let now = time::Instant::now();
        let elapsed = now - self.last_time;
        if elapsed.as_secs_f32() >= 3.0 {
            self.measurement = (self.frames, elapsed);
            self.last_time = now;
            self.frames = 0;
        }
    }

    pub fn fps(&self) -> f32 {
        let (frames, elapsed) = self.measurement;
        if frames == 0 || elapsed.is_zero() {
            0.0
        } else {
            frames as f32 / elapsed.as_secs_f32()
        }
    }

    pub fn mspf(&self) -> f32 {
        let (frames, elapsed) = self.measurement;
        if frames == 0 || elapsed.is_zero() {
            0.0
        } else {
            elapsed.as_secs_f32() / frames as f32 * 1000.0
        }
    }
}
