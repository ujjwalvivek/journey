use core::cell::UnsafeCell;
use resonance::envelope::{Adsr, AdsrState, Stage};
use resonance::oscillator::{self, Noise};
use resonance::patch::{Patch, PatchVoice};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const WAVE_SINE: u8 = 0;
const WAVE_SQUARE: u8 = 1;
const WAVE_SAWTOOTH: u8 = 2;
const WAVE_TRIANGLE: u8 = 3;
const WAVE_NOISE: u8 = 4;

pub struct Synth {
    sample_rate: u32,
    frequency: f32,
    phase: u64,
    phase_inc: u64,
    band: usize,
    waveform: u8,
    adsr: Adsr,
    env_state: AdsrState,
    noise: Noise,
    patch_voice: PatchVoice,
    master_gain: f32,
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        let frequency = 440.0;
        Self {
            sample_rate,
            frequency,
            phase: 0,
            phase_inc: oscillator::phase_increment(frequency, sample_rate),
            band: oscillator::octave_for_freq(frequency),
            waveform: WAVE_SINE,
            adsr: Adsr {
                attack_ms: 10.0,
                decay_ms: 50.0,
                sustain: 0.7,
                release_ms: 200.0,
            },
            env_state: AdsrState::off(),
            noise: Noise::new(),
            patch_voice: PatchVoice::new(sample_rate),
            master_gain: 0.5,
        }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        let frequency = clamp_finite(frequency, 20.0, 20_000.0, self.frequency);
        self.frequency = frequency;
        self.phase_inc = oscillator::phase_increment(frequency, self.sample_rate);
        self.band = oscillator::octave_for_freq(frequency);
    }

    pub fn set_waveform(&mut self, waveform: u8) {
        self.waveform = if waveform <= WAVE_NOISE {
            waveform
        } else {
            WAVE_SINE
        };
    }

    pub fn set_adsr(&mut self, attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32) {
        self.adsr = Adsr {
            attack_ms: clamp_finite(attack_ms, 0.0, 5_000.0, self.adsr.attack_ms),
            decay_ms: clamp_finite(decay_ms, 0.0, 5_000.0, self.adsr.decay_ms),
            sustain: clamp_finite(sustain, 0.0, 1.0, self.adsr.sustain),
            release_ms: clamp_finite(release_ms, 0.0, 5_000.0, self.adsr.release_ms),
        };
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = clamp_finite(gain, 0.0, 1.0, self.master_gain);
    }

    pub fn note_on(&mut self) {
        self.env_state = AdsrState::new();
        self.phase = 0;
    }

    pub fn note_off(&mut self) {
        self.env_state.note_off(&self.adsr);
    }

    pub fn trigger_patch(&mut self, patch: u8) {
        if let Some(patch) = Patch::from_index(patch) {
            self.patch_voice.trigger(patch);
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let ms_per_sample = 1000.0 / self.sample_rate as f32;
        let gain = self.env_state.tick(&self.adsr, ms_per_sample);

        let raw = match self.waveform {
            WAVE_SINE => oscillator::sine(self.phase),
            WAVE_SQUARE => oscillator::square(self.phase, self.band),
            WAVE_SAWTOOTH => oscillator::sawtooth(self.phase, self.band),
            WAVE_TRIANGLE => oscillator::triangle(self.phase, self.band),
            WAVE_NOISE => self.noise.next_sample(),
            _ => 0,
        };

        if self.waveform != WAVE_NOISE {
            self.phase = self.phase.wrapping_add(self.phase_inc);
        }

        let held_tone = (raw as f32 / 32768.0) * gain * self.master_gain * 0.7;
        let patch = (self.patch_voice.next_sample() as f32 / 32768.0) * 0.85;
        (held_tone + patch).clamp(-1.0, 1.0)
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    pub fn stage(&self) -> u8 {
        match self.env_state.stage {
            Stage::Attack => 0,
            Stage::Decay => 1,
            Stage::Sustain => 2,
            Stage::Release => 3,
            Stage::Off => 4,
        }
    }
}

struct SynthCell(UnsafeCell<Option<Synth>>);
unsafe impl Sync for SynthCell {}
static SYNTH: SynthCell = SynthCell(UnsafeCell::new(None));

fn with_synth<R>(fallback: R, f: impl FnOnce(&mut Synth) -> R) -> R {
    unsafe {
        match &mut *SYNTH.0.get() {
            Some(synth) => f(synth),
            None => fallback,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_init(sample_rate: u32) {
    unsafe { *SYNTH.0.get() = Some(Synth::new(sample_rate)); }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_set_frequency(frequency: f32) {
    with_synth((), |s| s.set_frequency(frequency));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_set_waveform(waveform: u8) {
    with_synth((), |s| s.set_waveform(waveform));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_set_adsr(attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32) {
    with_synth((), |s| s.set_adsr(attack_ms, decay_ms, sustain, release_ms));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_set_master_gain(gain: f32) {
    with_synth((), |s| s.set_master_gain(gain));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_note_on() { with_synth((), Synth::note_on); }

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_note_off() { with_synth((), Synth::note_off); }

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_trigger_patch(patch: u8) {
    with_synth((), |s| s.trigger_patch(patch));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_next_sample() -> f32 { with_synth(0.0, Synth::next_sample) }

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_frequency() -> f32 { with_synth(0.0, |s| s.frequency()) }

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn resonance_stage() -> u8 { with_synth(4, |s| s.stage()) }

use cadence::{Sequencer, TrackEvent};

const SEQ_PATCH_POOL: usize = 8;
const SEQ_SOUND_POOL: usize = 2;

struct CadenceEngine {
    sequencer: Sequencer<16>,
    patch_voices: [PatchVoice; SEQ_PATCH_POOL],
    sound_voices: [resonance::sound::SoundVoice; SEQ_SOUND_POOL],
    patch_cursor: usize,
    sound_cursor: usize,
    sample_rate: u32,
    active: bool,
}

impl CadenceEngine {
    fn new(sample_rate: u32, bpm: f32, scene: cadence::Scene, seed: u16) -> Self {
        Self {
            sequencer: cadence::scene::build(scene, sample_rate, bpm, seed),
            patch_voices: core::array::from_fn(|_| PatchVoice::new(sample_rate)),
            sound_voices: core::array::from_fn(|_| resonance::sound::SoundVoice::new(sample_rate)),
            patch_cursor: 0,
            sound_cursor: 0,
            sample_rate,
            active: true,
        }
    }

    fn next_sample(&mut self) -> f32 {
        if !self.active {
            return self.mix_voices();
        }

        let output = self.sequencer.tick();
        if output.step_triggered {
            for event in &output.events {
                match event {
                    Some(TrackEvent::TriggerPatch { patch_index, .. }) => {
                        if let Some(patch) = Patch::from_index(*patch_index) {
                            self.fire_patch(patch);
                        }
                    }
                    Some(TrackEvent::TriggerNote { frequency, gain }) => {
                        self.fire_note(*frequency, *gain);
                    }
                    None => {}
                }
            }
        }
        self.mix_voices()
    }

    fn fire_patch(&mut self, patch: Patch) {
        let start = self.patch_cursor;
        for i in 0..SEQ_PATCH_POOL {
            let idx = (start + i) % SEQ_PATCH_POOL;
            if !self.patch_voices[idx].is_active() {
                self.patch_voices[idx].trigger(patch);
                self.patch_cursor = (idx + 1) % SEQ_PATCH_POOL;
                return;
            }
        }
        self.patch_voices[self.patch_cursor].trigger(patch);
        self.patch_cursor = (self.patch_cursor + 1) % SEQ_PATCH_POOL;
    }

    fn fire_note(&mut self, frequency: f32, gain: f32) {
        use resonance::sound::{Curve, Layer, SoundSpec, Waveform};
        let layers = [
            Layer::tone(Waveform::Triangle, gain * 0.6, 300.0, frequency, frequency, Curve::Hold, Curve::Decay),
            Layer::tone(Waveform::Sine, gain * 0.4, 250.0, frequency * 2.0, frequency * 2.0, Curve::Hold, Curve::Decay),
            Layer::silent(),
            Layer::silent(),
        ];
        let spec = SoundSpec::from_layers(300.0, 2, layers);
        let start = self.sound_cursor;
        for i in 0..SEQ_SOUND_POOL {
            let idx = (start + i) % SEQ_SOUND_POOL;
            if !self.sound_voices[idx].is_active() {
                self.sound_voices[idx].trigger(spec);
                self.sound_cursor = (idx + 1) % SEQ_SOUND_POOL;
                return;
            }
        }
        self.sound_voices[self.sound_cursor].trigger(spec);
        self.sound_cursor = (self.sound_cursor + 1) % SEQ_SOUND_POOL;
    }

    fn mix_voices(&mut self) -> f32 {
        let mut sum: f32 = 0.0;
        for v in &mut self.patch_voices { sum += v.next_sample() as f32 / 32768.0; }
        for v in &mut self.sound_voices { sum += v.next_sample() as f32 / 32768.0; }
        sum * 0.45
    }
}

struct CadenceCell(UnsafeCell<Option<CadenceEngine>>);
unsafe impl Sync for CadenceCell {}
static CADENCE: CadenceCell = CadenceCell(UnsafeCell::new(None));

fn with_cadence<R>(fallback: R, f: impl FnOnce(&mut CadenceEngine) -> R) -> R {
    unsafe {
        match &mut *CADENCE.0.get() {
            Some(engine) => f(engine),
            None => fallback,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_init(sample_rate: u32, bpm: f32, scene_id: u8) {
    let scene = cadence::Scene::from_index(scene_id).unwrap_or(cadence::Scene::Drums);
    unsafe { *CADENCE.0.get() = Some(CadenceEngine::new(sample_rate, bpm, scene, 0xACE1)); }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_set_bpm(bpm: f32) {
    with_cadence((), |e| e.sequencer.set_bpm(bpm));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_set_scene(scene_id: u8) {
    with_cadence((), |e| {
        if let Some(scene) = cadence::Scene::from_index(scene_id) {
            let bpm = e.sequencer.bpm();
            let sr = e.sample_rate;
            e.sequencer = cadence::scene::build(scene, sr, bpm, 0xACE1);
        }
    });
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_set_active(active: bool) {
    with_cadence((), |e| { e.active = active; });
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_next_sample() -> f32 {
    with_cadence(0.0, CadenceEngine::next_sample)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_current_step() -> u32 {
    with_cadence(0, |e| e.sequencer.current_step_in_cycle())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn cadence_bpm() -> f32 {
    with_cadence(120.0, |e| e.sequencer.bpm())
}

fn clamp_finite(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() { value.clamp(min, max) } else { fallback }
}
