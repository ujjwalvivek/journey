pub const DEFAULT_STEPS_PER_BEAT: u32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct Transport {
    sample_rate: u32,
    bpm: f32,
    steps_per_beat: u32,
    current_sample: u64,
    current_step: u64,
    next_step: u64,
    next_step_sample: u64,
}

impl Transport {
    pub fn new(sample_rate: u32, bpm: f32) -> Self {
        Self::with_subdivision(sample_rate, bpm, DEFAULT_STEPS_PER_BEAT)
    }

    pub fn with_subdivision(sample_rate: u32, bpm: f32, steps_per_beat: u32) -> Self {
        let sample_rate = sample_rate.max(1);
        let bpm = sanitize_bpm(bpm);
        let steps_per_beat = steps_per_beat.max(1);

        Self {
            sample_rate,
            bpm,
            steps_per_beat,
            current_sample: 0,
            current_step: 0,
            next_step: 0,
            next_step_sample: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        let triggered = self.current_sample >= self.next_step_sample;
        if triggered {
            self.current_step = self.next_step;
            self.next_step = self.next_step.saturating_add(1);
            self.next_step_sample = self.sample_for_step(self.next_step);
            if self.next_step_sample <= self.current_sample {
                self.next_step_sample = self.current_sample.saturating_add(1);
            }
        }

        self.current_sample = self.current_sample.saturating_add(1);
        triggered
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = sanitize_bpm(bpm);
        self.next_step_sample = self.sample_for_step(self.next_step);
        if self.next_step_sample <= self.current_sample {
            self.next_step_sample = self.current_sample.saturating_add(1);
        }
    }

    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub const fn bpm(&self) -> f32 {
        self.bpm
    }

    pub const fn steps_per_beat(&self) -> u32 {
        self.steps_per_beat
    }

    pub const fn current_sample(&self) -> u64 {
        self.current_sample
    }

    pub const fn current_step(&self) -> u64 {
        self.current_step
    }

    pub fn step_in_cycle(&self, cycle_steps: u32) -> u32 {
        if cycle_steps == 0 {
            0
        } else {
            (self.current_step % cycle_steps as u64) as u32
        }
    }

    pub fn samples_per_step(&self) -> f64 {
        60.0 * self.sample_rate as f64 / (self.bpm as f64 * self.steps_per_beat as f64)
    }

    fn sample_for_step(&self, step: u64) -> u64 {
        round_positive_to_u64(step as f64 * self.samples_per_step())
    }
}

fn sanitize_bpm(bpm: f32) -> f32 {
    if bpm.is_finite() && bpm > 0.0 {
        bpm
    } else {
        120.0
    }
}

fn round_positive_to_u64(value: f64) -> u64 {
    (value + 0.5) as u64
}
