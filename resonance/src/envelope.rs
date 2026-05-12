#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

impl Adsr {
    pub fn gain(&self, stage: Stage, elapsed_ms: f32) -> f32 {
        match stage {
            Stage::Attack => {
                if self.attack_ms <= 0.0 {
                    return 1.0;
                }
                (elapsed_ms / self.attack_ms).min(1.0)
            }
            Stage::Decay => {
                if self.decay_ms <= 0.0 {
                    return self.sustain;
                }
                let t = (elapsed_ms / self.decay_ms).min(1.0);
                1.0 - t * (1.0 - self.sustain)
            }
            Stage::Sustain => self.sustain,
            Stage::Release => {
                if self.release_ms <= 0.0 {
                    return 0.0;
                }
                let t = (elapsed_ms / self.release_ms).min(1.0);
                self.sustain * (1.0 - t)
            }
            Stage::Off => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdsrState {
    pub stage: Stage,
    pub stage_elapsed_ms: f32,
    release_start_gain: f32,
}

impl AdsrState {
    pub const fn new() -> Self {
        Self {
            stage: Stage::Attack,
            stage_elapsed_ms: 0.0,
            release_start_gain: 0.0,
        }
    }
    pub const fn off() -> Self {
        Self {
            stage: Stage::Off,
            stage_elapsed_ms: 0.0,
            release_start_gain: 0.0,
        }
    }
    pub fn note_off(&mut self, adsr: &Adsr) {
        self.release_start_gain = adsr.gain(self.stage, self.stage_elapsed_ms);
        self.stage = Stage::Release;
        self.stage_elapsed_ms = 0.0;
    }
    pub fn tick(&mut self, adsr: &Adsr, ms_per_sample: f32) -> f32 {
        if self.stage == Stage::Off {
            return 0.0;
        }
        let gain = if self.stage == Stage::Release {
            if adsr.release_ms <= 0.0 {
                0.0
            } else {
                let t = (self.stage_elapsed_ms / adsr.release_ms).min(1.0);
                self.release_start_gain * (1.0 - t)
            }
        } else {
            adsr.gain(self.stage, self.stage_elapsed_ms)
        };
        self.stage_elapsed_ms += ms_per_sample;
        match self.stage {
            Stage::Attack if self.stage_elapsed_ms >= adsr.attack_ms => {
                self.stage = Stage::Decay;
                self.stage_elapsed_ms = 0.0;
            }
            Stage::Decay if self.stage_elapsed_ms >= adsr.decay_ms => {
                self.stage = Stage::Sustain;
                self.stage_elapsed_ms = 0.0;
            }
            Stage::Release if self.stage_elapsed_ms >= adsr.release_ms => {
                self.stage = Stage::Off;
                self.stage_elapsed_ms = 0.0;
            }
            _ => {}
        }
        gain
    }
}

impl Default for AdsrState {
    fn default() -> Self {
        Self::new()
    }
}
