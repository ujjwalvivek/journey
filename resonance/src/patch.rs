use crate::oscillator::{self, Noise};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Patch {
    Kick,
    Snare,
    HiHat,
    Laser,
    Coin,
    Explosion,
}

impl Patch {
    pub const COUNT: usize = 6;

    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Kick),
            1 => Some(Self::Snare),
            2 => Some(Self::HiHat),
            3 => Some(Self::Laser),
            4 => Some(Self::Coin),
            5 => Some(Self::Explosion),
            _ => None,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Kick => 0,
            Self::Snare => 1,
            Self::HiHat => 2,
            Self::Laser => 3,
            Self::Coin => 4,
            Self::Explosion => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Kick => "Kick",
            Self::Snare => "Snare",
            Self::HiHat => "HiHat",
            Self::Laser => "Laser",
            Self::Coin => "Coin",
            Self::Explosion => "Explosion",
        }
    }

    pub const fn duration_ms(self) -> f32 {
        match self {
            Self::Kick => 460.0,
            Self::Snare => 360.0,
            Self::HiHat => 120.0,
            Self::Laser => 520.0,
            Self::Coin => 440.0,
            Self::Explosion => 900.0,
        }
    }
}

pub struct PatchVoice {
    sample_rate: u32,
    patch: Patch,
    active: bool,
    elapsed_samples: u32,
    phase_a: u64,
    phase_b: u64,
    noise: Noise,
    lp_a: f32,
    lp_b: f32,
}

impl PatchVoice {
    pub const fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            patch: Patch::Kick,
            active: false,
            elapsed_samples: 0,
            phase_a: 0,
            phase_b: 0,
            noise: Noise::new(),
            lp_a: 0.0,
            lp_b: 0.0,
        }
    }

    pub fn trigger(&mut self, patch: Patch) {
        self.patch = patch;
        self.active = true;
        self.elapsed_samples = 0;
        self.phase_a = 0;
        self.phase_b = 0;
        self.noise = Noise::with_seed(0xACE1 ^ ((patch.index() as u16 + 1) * 0x1234));
        self.lp_a = 0.0;
        self.lp_b = 0.0;
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub const fn patch(&self) -> Patch {
        self.patch
    }

    pub fn elapsed_ms(&self) -> f32 {
        samples_to_ms(self.elapsed_samples, self.sample_rate)
    }

    pub fn next_sample(&mut self) -> i16 {
        if !self.active {
            return 0;
        }

        let t_ms = self.elapsed_ms();
        if t_ms >= self.patch.duration_ms() {
            self.active = false;
            return 0;
        }

        let sample = match self.patch {
            Patch::Kick => self.kick(t_ms),
            Patch::Snare => self.snare(t_ms),
            Patch::HiHat => self.hihat(t_ms),
            Patch::Laser => self.laser(t_ms),
            Patch::Coin => self.coin(t_ms),
            Patch::Explosion => self.explosion(t_ms),
        };

        self.elapsed_samples = self.elapsed_samples.saturating_add(1);
        sample
    }

    fn kick(&mut self, t_ms: f32) -> i16 {
        let pitch = 46.0 + 135.0 * decay2(t_ms, 38.0);
        let body = self.sine_a(pitch) * decay2(t_ms, 170.0);
        let click = self.noise_sample() * linear_down(t_ms, 7.0) * 0.28;
        to_i16((body * 1.08 + click) * 32767.0)
    }

    fn snare(&mut self, t_ms: f32) -> i16 {
        let raw_noise = self.noise_sample();
        let snap = self.high_pass_a(raw_noise, 0.045) * decay2(t_ms, 95.0);
        let body = self.sine_a(185.0) * decay2(t_ms, 150.0) * 0.38;
        let tail = raw_noise * decay2(t_ms, 220.0) * 0.25;
        to_i16((snap * 0.8 + body + tail) * 32767.0)
    }

    fn hihat(&mut self, t_ms: f32) -> i16 {
        let raw_noise = self.noise_sample();
        let metallic = self.high_pass_a(raw_noise, 0.018);
        let tick = self.square_b(7400.0) * 0.18;
        to_i16((metallic * 0.82 + tick) * decay2(t_ms, 34.0) * 32767.0)
    }

    fn laser(&mut self, t_ms: f32) -> i16 {
        let pitch = 120.0 + 1750.0 * decay2(t_ms, 150.0);
        let band = oscillator::octave_for_freq(pitch);
        let saw = self.saw_a(pitch, band);
        let square = self.square_b(pitch * 0.5);
        let amp = linear_down(t_ms, 520.0) * decay2(t_ms, 360.0);
        to_i16((saw * 0.72 + square * 0.28) * amp * 32767.0)
    }

    fn coin(&mut self, t_ms: f32) -> i16 {
        let base = if t_ms < 70.0 {
            1318.51
        } else if t_ms < 145.0 {
            1760.0
        } else {
            2637.02
        };
        let amp = linear_down(t_ms, 430.0) * decay2(t_ms, 520.0);
        let chime = self.sine_a(base) * 0.68 + self.sine_b(base * 2.01) * 0.32;
        to_i16(chime * amp * 32767.0)
    }

    fn explosion(&mut self, t_ms: f32) -> i16 {
        let raw_noise = self.noise_sample();
        let cutoff = 0.18 - 0.15 * (t_ms / Patch::Explosion.duration_ms()).min(1.0);
        let boom_noise = self.low_pass_a(raw_noise, cutoff.max(0.02));
        let rumble = self.sine_b(42.0 + 24.0 * decay2(t_ms, 220.0));
        let crack = self.high_pass_b(raw_noise, 0.08) * linear_down(t_ms, 45.0);
        let amp = linear_down(t_ms, 900.0) * decay2(t_ms, 700.0);
        to_i16((boom_noise * 0.82 + rumble * 0.34 + crack * 0.36) * amp * 32767.0)
    }

    fn sine_a(&mut self, freq_hz: f32) -> f32 {
        let sample = oscillator::sine(self.phase_a) as f32 / 32768.0;
        self.phase_a = self
            .phase_a
            .wrapping_add(oscillator::phase_increment(freq_hz, self.sample_rate));
        sample
    }

    fn sine_b(&mut self, freq_hz: f32) -> f32 {
        let sample = oscillator::sine(self.phase_b) as f32 / 32768.0;
        self.phase_b = self
            .phase_b
            .wrapping_add(oscillator::phase_increment(freq_hz, self.sample_rate));
        sample
    }

    fn square_b(&mut self, freq_hz: f32) -> f32 {
        let band = oscillator::octave_for_freq(freq_hz);
        let sample = oscillator::square(self.phase_b, band) as f32 / 32768.0;
        self.phase_b = self
            .phase_b
            .wrapping_add(oscillator::phase_increment(freq_hz, self.sample_rate));
        sample
    }

    fn saw_a(&mut self, freq_hz: f32, band: usize) -> f32 {
        let sample = oscillator::sawtooth(self.phase_a, band) as f32 / 32768.0;
        self.phase_a = self
            .phase_a
            .wrapping_add(oscillator::phase_increment(freq_hz, self.sample_rate));
        sample
    }

    fn noise_sample(&mut self) -> f32 {
        self.noise.next_sample() as f32 / 32768.0
    }

    fn low_pass_a(&mut self, input: f32, coeff: f32) -> f32 {
        self.lp_a += coeff * (input - self.lp_a);
        self.lp_a
    }

    fn high_pass_a(&mut self, input: f32, coeff: f32) -> f32 {
        input - self.low_pass_a(input, coeff)
    }

    fn high_pass_b(&mut self, input: f32, coeff: f32) -> f32 {
        self.lp_b += coeff * (input - self.lp_b);
        input - self.lp_b
    }
}

fn samples_to_ms(samples: u32, sample_rate: u32) -> f32 {
    samples as f32 * 1000.0 / sample_rate as f32
}

fn decay2(t_ms: f32, halfish_ms: f32) -> f32 {
    let x = 1.0 / (1.0 + t_ms / halfish_ms.max(0.001));
    x * x
}

fn linear_down(t_ms: f32, duration_ms: f32) -> f32 {
    (1.0 - t_ms / duration_ms.max(0.001)).clamp(0.0, 1.0)
}

fn to_i16(sample: f32) -> i16 {
    if sample >= i16::MAX as f32 {
        i16::MAX
    } else if sample <= i16::MIN as f32 {
        i16::MIN
    } else {
        sample as i16
    }
}
