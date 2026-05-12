use crate::oscillator::{self, Noise};

pub const MAX_LAYERS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Noise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    Hold,
    Linear,
    Decay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    None,
    LowPass,
    HighPass,
}

#[derive(Debug, Clone, Copy)]
pub struct Layer {
    pub waveform: Waveform,
    pub gain: f32,
    pub start_ms: f32,
    pub duration_ms: f32,
    pub start_freq_hz: f32,
    pub end_freq_hz: f32,
    pub pitch_curve: Curve,
    pub amp_curve: Curve,
    pub filter_mode: FilterMode,
    pub filter_coeff: f32,
}

impl Layer {
    pub const fn silent() -> Self {
        Self {
            waveform: Waveform::Sine,
            gain: 0.0,
            start_ms: 0.0,
            duration_ms: 0.0,
            start_freq_hz: 440.0,
            end_freq_hz: 440.0,
            pitch_curve: Curve::Hold,
            amp_curve: Curve::Hold,
            filter_mode: FilterMode::None,
            filter_coeff: 0.0,
        }
    }
    pub const fn tone(
        waveform: Waveform,
        gain: f32,
        duration_ms: f32,
        start_freq_hz: f32,
        end_freq_hz: f32,
        pitch_curve: Curve,
        amp_curve: Curve,
    ) -> Self {
        Self {
            waveform,
            gain,
            start_ms: 0.0,
            duration_ms,
            start_freq_hz,
            end_freq_hz,
            pitch_curve,
            amp_curve,
            filter_mode: FilterMode::None,
            filter_coeff: 0.0,
        }
    }
    pub const fn noise(
        gain: f32,
        duration_ms: f32,
        amp_curve: Curve,
        filter_mode: FilterMode,
        filter_coeff: f32,
    ) -> Self {
        Self {
            waveform: Waveform::Noise,
            gain,
            start_ms: 0.0,
            duration_ms,
            start_freq_hz: 0.0,
            end_freq_hz: 0.0,
            pitch_curve: Curve::Hold,
            amp_curve,
            filter_mode,
            filter_coeff,
        }
    }
    pub const fn starting_at(mut self, start_ms: f32) -> Self {
        self.start_ms = start_ms;
        self
    }
    pub const fn filtered(mut self, mode: FilterMode, coeff: f32) -> Self {
        self.filter_mode = mode;
        self.filter_coeff = coeff;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SoundSpec {
    pub duration_ms: f32,
    pub layer_count: usize,
    pub layers: [Layer; MAX_LAYERS],
}

impl SoundSpec {
    pub const fn empty() -> Self {
        Self {
            duration_ms: 0.0,
            layer_count: 0,
            layers: [Layer::silent(); MAX_LAYERS],
        }
    }
    pub const fn from_layers(
        duration_ms: f32,
        layer_count: usize,
        layers: [Layer; MAX_LAYERS],
    ) -> Self {
        Self {
            duration_ms,
            layer_count,
            layers,
        }
    }
}

pub struct SoundVoice {
    sample_rate: u32,
    spec: SoundSpec,
    active: bool,
    elapsed_samples: u32,
    phases: [u64; MAX_LAYERS],
    filters: [f32; MAX_LAYERS],
    noise: Noise,
}

impl SoundVoice {
    pub const fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            spec: SoundSpec::empty(),
            active: false,
            elapsed_samples: 0,
            phases: [0; MAX_LAYERS],
            filters: [0.0; MAX_LAYERS],
            noise: Noise::new(),
        }
    }
    pub fn trigger(&mut self, spec: SoundSpec) {
        self.spec = spec;
        self.active = true;
        self.elapsed_samples = 0;
        self.phases = [0; MAX_LAYERS];
        self.filters = [0.0; MAX_LAYERS];
        self.noise = Noise::with_seed(0xBEEF);
    }
    pub const fn is_active(&self) -> bool {
        self.active
    }
    pub fn elapsed_ms(&self) -> f32 {
        self.elapsed_samples as f32 * 1000.0 / self.sample_rate as f32
    }
    pub fn next_sample(&mut self) -> i16 {
        if !self.active {
            return 0;
        }

        let t_ms = self.elapsed_ms();
        if t_ms >= self.spec.duration_ms {
            self.active = false;
            return 0;
        }

        let mut sum = 0.0;
        let layer_count = self.spec.layer_count.min(MAX_LAYERS);
        for index in 0..layer_count {
            let layer = self.spec.layers[index];
            sum += self.render_layer(index, layer, t_ms);
        }

        self.elapsed_samples = self.elapsed_samples.saturating_add(1);
        to_i16(sum * 32767.0)
    }
    fn render_layer(&mut self, index: usize, layer: Layer, t_ms: f32) -> f32 {
        if layer.gain == 0.0 || layer.duration_ms <= 0.0 || t_ms < layer.start_ms {
            return 0.0;
        }

        let local_t = t_ms - layer.start_ms;
        if local_t >= layer.duration_ms {
            return 0.0;
        }

        let amp = layer.gain * amp_at(layer.amp_curve, local_t, layer.duration_ms);
        let freq = value_at(
            layer.pitch_curve,
            layer.start_freq_hz,
            layer.end_freq_hz,
            local_t,
            layer.duration_ms,
        );
        let raw = match layer.waveform {
            Waveform::Sine => self.osc(index, freq, oscillator::sine),
            Waveform::Square => self.osc_banded(index, freq, oscillator::square),
            Waveform::Sawtooth => self.osc_banded(index, freq, oscillator::sawtooth),
            Waveform::Triangle => self.osc_banded(index, freq, oscillator::triangle),
            Waveform::Noise => self.noise.next_sample() as f32 / 32768.0,
        };

        self.apply_filter(index, layer, raw) * amp
    }
    fn osc(&mut self, index: usize, freq: f32, osc_fn: fn(u64) -> i16) -> f32 {
        let sample = osc_fn(self.phases[index]) as f32 / 32768.0;
        self.phases[index] =
            self.phases[index].wrapping_add(oscillator::phase_increment(freq, self.sample_rate));
        sample
    }
    fn osc_banded(&mut self, index: usize, freq: f32, osc_fn: fn(u64, usize) -> i16) -> f32 {
        let band = oscillator::octave_for_freq(freq);
        let sample = osc_fn(self.phases[index], band) as f32 / 32768.0;
        self.phases[index] =
            self.phases[index].wrapping_add(oscillator::phase_increment(freq, self.sample_rate));
        sample
    }
    fn apply_filter(&mut self, index: usize, layer: Layer, input: f32) -> f32 {
        let coeff = layer.filter_coeff.clamp(0.0, 1.0);
        match layer.filter_mode {
            FilterMode::None => input,
            FilterMode::LowPass => {
                self.filters[index] += coeff * (input - self.filters[index]);
                self.filters[index]
            }
            FilterMode::HighPass => {
                self.filters[index] += coeff * (input - self.filters[index]);
                input - self.filters[index]
            }
        }
    }
}

fn amp_at(curve: Curve, t_ms: f32, duration_ms: f32) -> f32 {
    match curve {
        Curve::Hold => 1.0,
        Curve::Linear => linear_down(t_ms, duration_ms),
        Curve::Decay => decay2(t_ms, duration_ms * 0.35),
    }
}

fn value_at(curve: Curve, start: f32, end: f32, t_ms: f32, duration_ms: f32) -> f32 {
    match curve {
        Curve::Hold => start,
        Curve::Linear => {
            let t = (t_ms / duration_ms.max(0.001)).clamp(0.0, 1.0);
            start + (end - start) * t
        }
        Curve::Decay => end + (start - end) * decay2(t_ms, duration_ms * 0.35),
    }
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
