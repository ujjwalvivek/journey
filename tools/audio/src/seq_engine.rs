#![allow(dead_code)]

use cadence::{Scene, Sequencer, TrackEvent};
use resonance::patch::{Patch, PatchVoice};
use resonance::sound::SoundVoice;

const PATCH_POOL: usize = 8;
const SOUND_POOL: usize = 2;

pub struct SequencerEngine {
    sequencer: Sequencer<16>,
    patch_voices: [PatchVoice; PATCH_POOL],
    sound_voices: [SoundVoice; SOUND_POOL],
    patch_cursor: usize,
    sound_cursor: usize,
    pub active: bool,
    pub scene: Scene,
}

impl SequencerEngine {
    pub fn new(sample_rate: u32, bpm: f32, scene: Scene, seed: u16) -> Self {
        Self {
            sequencer: cadence::scene::build(scene, sample_rate, bpm, seed),
            patch_voices: core::array::from_fn(|_| PatchVoice::new(sample_rate)),
            sound_voices: core::array::from_fn(|_| SoundVoice::new(sample_rate)),
            patch_cursor: 0,
            sound_cursor: 0,
            active: false,
            scene,
        }
    }

    pub fn set_scene(&mut self, scene: Scene, sample_rate: u32, bpm: f32, seed: u16) {
        self.scene = scene;
        self.sequencer = cadence::scene::build(scene, sample_rate, bpm, seed);
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.sequencer.set_bpm(bpm);
    }

    pub fn bpm(&self) -> f32 {
        self.sequencer.bpm()
    }

    pub fn track_count(&self) -> usize {
        self.sequencer.track_count()
    }

    pub fn cycle_steps(&self) -> u32 {
        self.sequencer.cycle_steps()
    }

    pub fn current_step(&self) -> u32 {
        self.sequencer.current_step_in_cycle()
    }

    pub fn mute(&mut self, index: usize) {
        self.sequencer.mute(index);
    }

    pub fn unmute(&mut self, index: usize) {
        self.sequencer.unmute(index);
    }

    pub fn is_track_active(&self, index: usize) -> bool {
        self.sequencer.is_track_active(index)
    }

    pub fn next_sample(&mut self) -> f32 {
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
        for i in 0..PATCH_POOL {
            let idx = (start + i) % PATCH_POOL;
            if !self.patch_voices[idx].is_active() {
                self.patch_voices[idx].trigger(patch);
                self.patch_cursor = (idx + 1) % PATCH_POOL;
                return;
            }
        }
        self.patch_voices[self.patch_cursor].trigger(patch);
        self.patch_cursor = (self.patch_cursor + 1) % PATCH_POOL;
    }

    fn fire_note(&mut self, frequency: f32, gain: f32) {
        use resonance::sound::{Curve, Layer, SoundSpec, Waveform};

        let layers = [
            Layer::tone(
                Waveform::Triangle,
                gain * 0.6,
                300.0,
                frequency,
                frequency,
                Curve::Hold,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Sine,
                gain * 0.4,
                250.0,
                frequency * 2.0,
                frequency * 2.0,
                Curve::Hold,
                Curve::Decay,
            ),
            Layer::silent(),
            Layer::silent(),
        ];
        let spec = SoundSpec::from_layers(300.0, 2, layers);

        let start = self.sound_cursor;
        for i in 0..SOUND_POOL {
            let idx = (start + i) % SOUND_POOL;
            if !self.sound_voices[idx].is_active() {
                self.sound_voices[idx].trigger(spec);
                self.sound_cursor = (idx + 1) % SOUND_POOL;
                return;
            }
        }
        self.sound_voices[self.sound_cursor].trigger(spec);
        self.sound_cursor = (self.sound_cursor + 1) % SOUND_POOL;
    }

    fn mix_voices(&mut self) -> f32 {
        let mut sum: f32 = 0.0;
        for voice in &mut self.patch_voices {
            sum += voice.next_sample() as f32 / 32768.0;
        }
        for voice in &mut self.sound_voices {
            sum += voice.next_sample() as f32 / 32768.0;
        }
        sum * 0.45
    }
}
