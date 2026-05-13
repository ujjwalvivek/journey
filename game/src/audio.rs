/**----------------------------------------------------------------------
*!  Audio asset management: loads all game sounds from embedded bytes.
*?  Uses embedded OGG assets for authored sounds and procedural generation
*?  for synth UI/menu audio.
*?  Sound data is loaded once during init and stored for the game lifetime.
*----------------------------------------------------------------------**/
use cadence::{EuclideanPattern, Lfsr, MarkovChain, Transport};
use engine::{
    AudioManager, AudioTrack, StaticSoundData, UiAudioEvent, load_sound_data,
    sound_data_from_mono_samples,
};
use resonance::patch::{Patch, PatchVoice};
use resonance::sound::{Curve, FilterMode, Layer, SoundSpec, SoundVoice, Waveform};

const UI_SAMPLE_RATE: u32 = 48_000;
const MENU_SAMPLE_RATE: u32 = 48_000;
const MENU_BPM: f32 = 108.0;
const MENU_STEPS: usize = 64;
const MENU_TAIL_MS: f32 = 700.0;
const MENU_PATCH_VOICES: usize = 14;
const MENU_SOUND_VOICES: usize = 24;
const GAME_SAMPLE_RATE: u32 = 48_000;
const GAME_BPM: f32 = 112.0;
const GAME_STEPS: usize = 128;
const GAME_TAIL_MS: f32 = 900.0;
const GAME_PATCH_VOICES: usize = 16;
const GAME_SOUND_VOICES: usize = 28;

//? Game-specific one-shot audio events, produced during gameplay and drained per frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AudioEvent {
    Jump,
    Land,
    Dash,
    Run,
    WallGrab,
    WallSlide,
    Swing,
    Hit,
    Parry,
    Stagger,
    Death,
    Respawn,
    GrappleStatic,
    GrappleEnemy,
    Projectile,
    ProjectileBounce,
    RunStop,
}

pub struct AudioAssets {
    pub start_audio: Option<StaticSoundData>,
    pub bg_music: Option<StaticSoundData>,
    pub ambient_audio: Option<StaticSoundData>,
    pub ui_hover: Option<StaticSoundData>,
    pub ui_click: Option<StaticSoundData>,
    pub ui_tab_change: Option<StaticSoundData>,
    pub ui_checkbox_on: Option<StaticSoundData>,
    pub ui_checkbox_off: Option<StaticSoundData>,
    pub sfx_jump: Option<StaticSoundData>,
    pub sfx_land: Option<StaticSoundData>,
    pub sfx_dash: Option<StaticSoundData>,
    pub sfx_run: Option<StaticSoundData>,
    pub sfx_wall_grab: Option<StaticSoundData>,
    pub sfx_wall_slide: Option<StaticSoundData>,
    pub sfx_swing: Option<StaticSoundData>,
    pub sfx_hit: Option<StaticSoundData>,
    pub sfx_parry: Option<StaticSoundData>,
    pub sfx_stagger: Option<StaticSoundData>,
    pub sfx_death: Option<StaticSoundData>,
    pub sfx_respawn: Option<StaticSoundData>,
    pub sfx_grapple_static: Option<StaticSoundData>,
    pub sfx_grapple_enemy: Option<StaticSoundData>,
    pub sfx_projectile: Option<StaticSoundData>,
    pub sfx_projectile_bounce: Option<StaticSoundData>,
}

impl AudioAssets {
    pub fn load() -> Self {
        Self {
            start_audio: render_menu_loop(),
            bg_music: render_gameplay_loop(),
            ambient_audio: load_sound_data(include_bytes!("../assets/audio/ambient_audio.ogg")),

            ui_hover: render_ui_sound(ui_hover_spec()),
            ui_click: render_ui_sound(ui_click_spec()),
            ui_tab_change: render_ui_sound(ui_tab_change_spec()),
            ui_checkbox_on: render_ui_sound(ui_checkbox_on_spec()),
            ui_checkbox_off: render_ui_sound(ui_checkbox_off_spec()),

            sfx_jump: load_sound_data(include_bytes!("../assets/audio/sfx_jump.ogg")),
            sfx_land: load_sound_data(include_bytes!("../assets/audio/sfx_land.ogg")),
            sfx_dash: load_sound_data(include_bytes!("../assets/audio/sfx_dash.ogg")),
            sfx_run: load_sound_data(include_bytes!("../assets/audio/sfx_run.ogg")),
            sfx_wall_grab: load_sound_data(include_bytes!("../assets/audio/sfx_wall_grab.ogg")),
            sfx_wall_slide: render_sfx_sound(sfx_wall_slide_spec()),

            sfx_swing: load_sound_data(include_bytes!("../assets/audio/sfx_swing.ogg")),
            sfx_hit: load_sound_data(include_bytes!("../assets/audio/sfx_hit.ogg")),
            sfx_parry: load_sound_data(include_bytes!("../assets/audio/sfx_parry.ogg")),
            sfx_stagger: load_sound_data(include_bytes!("../assets/audio/sfx_stagger.ogg")),
            sfx_death: render_sfx_sound(sfx_death_spec()),
            sfx_respawn: load_sound_data(include_bytes!("../assets/audio/sfx_respawn.ogg")),

            sfx_grapple_static: load_sound_data(include_bytes!(
                "../assets/audio/sfx_grapple_static.ogg"
            )),
            sfx_grapple_enemy: render_sfx_sound(sfx_grapple_enemy_spec()),

            sfx_projectile: load_sound_data(include_bytes!("../assets/audio/sfx_projectile.ogg")),
            sfx_projectile_bounce: load_sound_data(include_bytes!(
                "../assets/audio/sfx_projectile_bounce.ogg"
            )),
        }
    }

    //? Dispatch a game-specific `AudioEvent` to the appropriate sound data.
    pub fn dispatch(&self, event: AudioEvent, audio: &mut AudioManager) {
        match event {
            AudioEvent::Run => {
                if let Some(ref data) = self.sfx_run {
                    audio.play_loop_sfx(data);
                }
                return;
            }
            AudioEvent::RunStop => {
                audio.stop_loop_sfx(0.1);
                return;
            }
            _ => {}
        }

        let (data, track): (&Option<StaticSoundData>, AudioTrack) = match event {
            AudioEvent::Jump => (&self.sfx_jump, AudioTrack::Sfx),
            AudioEvent::Land => (&self.sfx_land, AudioTrack::Sfx),
            AudioEvent::Dash => (&self.sfx_dash, AudioTrack::Sfx),
            AudioEvent::WallGrab => (&self.sfx_wall_grab, AudioTrack::Sfx),
            AudioEvent::WallSlide => (&self.sfx_wall_slide, AudioTrack::Sfx),
            AudioEvent::Swing => (&self.sfx_swing, AudioTrack::Sfx),
            AudioEvent::Hit => (&self.sfx_hit, AudioTrack::Sfx),
            AudioEvent::Parry => (&self.sfx_parry, AudioTrack::Sfx),
            AudioEvent::Stagger => (&self.sfx_stagger, AudioTrack::Sfx),
            AudioEvent::Death => (&self.sfx_death, AudioTrack::Sfx),
            AudioEvent::Respawn => (&self.sfx_respawn, AudioTrack::Sfx),
            AudioEvent::GrappleStatic => (&self.sfx_grapple_static, AudioTrack::Sfx),
            AudioEvent::GrappleEnemy => (&self.sfx_grapple_enemy, AudioTrack::Sfx),
            AudioEvent::Projectile => (&self.sfx_projectile, AudioTrack::Sfx),
            AudioEvent::ProjectileBounce => (&self.sfx_projectile_bounce, AudioTrack::Sfx),
            AudioEvent::Run | AudioEvent::RunStop => return,
        };
        if let Some(sound) = data {
            audio.play_oneshot(sound, track);
        }
    }

    //? Dispatch an engine-level `UiAudioEvent` to the appropriate UI sound.
    pub fn dispatch_ui(&self, event: UiAudioEvent, audio: &mut AudioManager) {
        let data: &Option<StaticSoundData> = match event {
            UiAudioEvent::Hover => &self.ui_hover,
            UiAudioEvent::Click => &self.ui_click,
            UiAudioEvent::TabChange => &self.ui_tab_change,
            UiAudioEvent::CheckboxOn => &self.ui_checkbox_on,
            UiAudioEvent::CheckboxOff => &self.ui_checkbox_off,
        };
        if let Some(sound) = data {
            audio.play_oneshot(sound, AudioTrack::Ui);
        }
    }
}

struct MenuPatchSlot {
    voice: PatchVoice,
    gain: f32,
}

fn render_menu_loop() -> Option<StaticSoundData> {
    const BASS_NOTES: [f32; 8] = [55.0, 65.41, 73.42, 82.41, 98.0, 110.0, 130.81, 146.83];
    const LEAD_NOTES: [f32; 8] = [
        440.0, 523.25, 587.33, 659.25, 783.99, 880.0, 1046.5, 1174.66,
    ];
    const CHORD_ROOTS: [f32; 4] = [110.0, 98.0, 130.81, 146.83];
    const BASS_MATRIX: [[u8; 8]; 8] = [
        [0, 35, 10, 0, 20, 5, 0, 0],
        [20, 0, 30, 10, 5, 0, 0, 0],
        [5, 20, 0, 35, 10, 0, 0, 0],
        [10, 5, 25, 0, 30, 5, 0, 0],
        [15, 5, 5, 15, 0, 25, 5, 0],
        [20, 0, 5, 5, 20, 0, 20, 0],
        [5, 15, 0, 0, 10, 25, 0, 15],
        [10, 5, 15, 0, 5, 10, 25, 0],
    ];
    const LEAD_MATRIX: [[u8; 8]; 8] = [
        [0, 32, 12, 4, 0, 0, 0, 4],
        [18, 0, 34, 12, 6, 0, 0, 0],
        [4, 18, 0, 34, 14, 0, 0, 0],
        [4, 4, 20, 0, 30, 10, 0, 0],
        [0, 4, 4, 20, 0, 34, 12, 0],
        [0, 0, 4, 4, 20, 0, 34, 12],
        [4, 0, 0, 4, 4, 24, 0, 28],
        [12, 4, 0, 0, 4, 12, 28, 0],
    ];

    let mut transport = Transport::new(MENU_SAMPLE_RATE, MENU_BPM);
    let loop_samples = (transport.samples_per_step() * MENU_STEPS as f64 + 0.5).max(1.0) as usize;
    let tail_samples = (MENU_TAIL_MS * MENU_SAMPLE_RATE as f32 / 1000.0) as usize;
    let total_samples = loop_samples + tail_samples;

    let kick_pattern = EuclideanPattern::<16>::new(4, 16, 0);
    let snare_pattern = EuclideanPattern::<16>::new(2, 16, 4);
    let hat_pattern = EuclideanPattern::<16>::new(8, 16, 2);
    let bass_pattern = EuclideanPattern::<16>::new(7, 16, 0);
    let lead_pattern = EuclideanPattern::<16>::new(5, 16, 3);
    let shimmer_pattern = EuclideanPattern::<16>::new(3, 16, 7);

    let mut lfsr = Lfsr::new(0xC0DE);
    let mut bass_chain = MarkovChain::<8>::new(BASS_MATRIX, 0);
    let mut lead_chain = MarkovChain::<8>::new(LEAD_MATRIX, 3);
    let mut patch_cursor = 0usize;
    let mut sound_cursor = 0usize;
    let mut patch_voices: [MenuPatchSlot; MENU_PATCH_VOICES] =
        core::array::from_fn(|_| MenuPatchSlot {
            voice: PatchVoice::new(MENU_SAMPLE_RATE),
            gain: 0.0,
        });
    let mut sound_voices: [SoundVoice; MENU_SOUND_VOICES] =
        core::array::from_fn(|_| SoundVoice::new(MENU_SAMPLE_RATE));
    let mut mix = vec![0.0f32; total_samples];

    for (sample_index, mixed_sample) in mix.iter_mut().enumerate() {
        if sample_index < loop_samples && transport.tick() {
            let step = (transport.current_step() % MENU_STEPS as u64) as usize;
            let step16 = step % 16;

            if kick_pattern.is_active(step16) {
                trigger_menu_patch(
                    &mut patch_voices,
                    &mut patch_cursor,
                    Patch::Kick,
                    if step16 == 0 { 0.72 } else { 0.58 },
                );
            }
            if snare_pattern.is_active(step16) {
                trigger_menu_patch(&mut patch_voices, &mut patch_cursor, Patch::Snare, 0.46);
            }
            if hat_pattern.is_active(step16) && step16 != 0 && step16 != 8 {
                trigger_menu_patch(&mut patch_voices, &mut patch_cursor, Patch::HiHat, 0.24);
            }
            if step.is_multiple_of(16) {
                let root = CHORD_ROOTS[(step / 16) % CHORD_ROOTS.len()];
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_chord_spec(root, 0.42),
                );
            }
            if bass_pattern.is_active(step16) {
                let note = BASS_NOTES[bass_chain.next(lfsr.next_u16())];
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_bass_spec(note, 0.72),
                );
            }
            if lead_pattern.is_active(step16) && step >= 16 {
                let note = LEAD_NOTES[lead_chain.next(lfsr.next_u16())];
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_lead_spec(note, 0.38),
                );
            }
            if shimmer_pattern.is_active(step16) && step >= 32 {
                let note = LEAD_NOTES[lead_chain.next(lfsr.next_u16())] * 2.0;
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_shimmer_spec(note, 0.18),
                );
            }
        }

        let mut sample = 0.0f32;
        for slot in &mut patch_voices {
            sample += slot.voice.next_sample() as f32 / 32768.0 * slot.gain;
        }
        for voice in &mut sound_voices {
            sample += voice.next_sample() as f32 / 32768.0;
        }
        *mixed_sample = soft_clip(sample * 0.62);
    }

    for i in 0..tail_samples {
        mix[i] = soft_clip(mix[i] + mix[loop_samples + i]);
    }

    let samples: Vec<i16> = mix[..loop_samples]
        .iter()
        .map(|sample| sample_to_i16(*sample * 0.86))
        .collect();
    sound_data_from_mono_samples(MENU_SAMPLE_RATE, &samples)
}

fn render_gameplay_loop() -> Option<StaticSoundData> {
    const BASS_NOTES: [f32; 8] = [49.0, 55.0, 65.41, 73.42, 82.41, 98.0, 110.0, 130.81];
    const LEAD_NOTES: [f32; 8] = [392.0, 440.0, 523.25, 587.33, 659.25, 783.99, 880.0, 1046.5];
    const CHORD_ROOTS: [f32; 8] = [98.0, 110.0, 82.41, 73.42, 98.0, 130.81, 82.41, 110.0];
    const BASS_MATRIX: [[u8; 8]; 8] = [
        [0, 34, 8, 0, 22, 8, 0, 0],
        [24, 0, 28, 6, 12, 0, 0, 0],
        [6, 18, 0, 34, 14, 0, 0, 0],
        [10, 4, 22, 0, 32, 8, 0, 0],
        [18, 6, 4, 16, 0, 28, 6, 0],
        [22, 0, 4, 8, 20, 0, 22, 0],
        [8, 10, 0, 0, 12, 26, 0, 18],
        [14, 4, 12, 0, 6, 12, 28, 0],
    ];
    const LEAD_MATRIX: [[u8; 8]; 8] = [
        [0, 30, 10, 0, 12, 0, 0, 0],
        [16, 0, 30, 10, 0, 6, 0, 0],
        [4, 16, 0, 32, 12, 0, 0, 0],
        [0, 6, 18, 0, 30, 14, 0, 0],
        [0, 0, 8, 18, 0, 30, 12, 0],
        [0, 0, 0, 6, 20, 0, 32, 14],
        [8, 0, 0, 0, 6, 22, 0, 26],
        [24, 8, 0, 0, 0, 10, 18, 0],
    ];

    let mut transport = Transport::new(GAME_SAMPLE_RATE, GAME_BPM);
    let loop_samples = (transport.samples_per_step() * GAME_STEPS as f64 + 0.5).max(1.0) as usize;
    let tail_samples = (GAME_TAIL_MS * GAME_SAMPLE_RATE as f32 / 1000.0) as usize;
    let total_samples = loop_samples + tail_samples;

    let hat_pattern = EuclideanPattern::<16>::new(11, 16, 2);
    let bass_pattern = EuclideanPattern::<16>::new(9, 16, 0);
    let pulse_pattern = EuclideanPattern::<16>::new(5, 16, 1);
    let lead_pattern = EuclideanPattern::<16>::new(3, 16, 6);
    let shimmer_pattern = EuclideanPattern::<16>::new(2, 16, 11);

    let mut lfsr = Lfsr::new(0xA11C);
    let mut bass_chain = MarkovChain::<8>::new(BASS_MATRIX, 0);
    let mut lead_chain = MarkovChain::<8>::new(LEAD_MATRIX, 2);
    let mut patch_cursor = 0usize;
    let mut sound_cursor = 0usize;
    let mut patch_voices: [MenuPatchSlot; GAME_PATCH_VOICES] =
        core::array::from_fn(|_| MenuPatchSlot {
            voice: PatchVoice::new(GAME_SAMPLE_RATE),
            gain: 0.0,
        });
    let mut sound_voices: [SoundVoice; GAME_SOUND_VOICES] =
        core::array::from_fn(|_| SoundVoice::new(GAME_SAMPLE_RATE));
    let mut mix = vec![0.0f32; total_samples];

    for (sample_index, mixed_sample) in mix.iter_mut().enumerate() {
        if sample_index < loop_samples && transport.tick() {
            let step = (transport.current_step() % GAME_STEPS as u64) as usize;
            let step16 = step % 16;
            let phrase = step / 16;
            let root = CHORD_ROOTS[phrase % CHORD_ROOTS.len()];

            if matches!(step16, 0 | 8) {
                let kick_gain = if step16 == 0 { 0.68 } else { 0.50 };
                trigger_menu_patch(&mut patch_voices, &mut patch_cursor, Patch::Kick, kick_gain);
            }
            if matches!(step16, 4 | 12) {
                trigger_menu_patch(&mut patch_voices, &mut patch_cursor, Patch::Snare, 0.34);
            }
            if hat_pattern.is_active(step16) && !matches!(step16, 0 | 4 | 8 | 12) {
                let hat_gain = if phrase >= 4 { 0.19 } else { 0.16 };
                trigger_menu_patch(&mut patch_voices, &mut patch_cursor, Patch::HiHat, hat_gain);
            }
            if matches!(step % 64, 30 | 62) {
                trigger_menu_patch(&mut patch_voices, &mut patch_cursor, Patch::HiHat, 0.12);
            }
            if step.is_multiple_of(16) {
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_chord_spec(root, 0.30),
                );
            }
            if pulse_pattern.is_active(step16) {
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    gameplay_pulse_spec(root * 2.0, 0.24),
                );
            }
            if bass_pattern.is_active(step16) {
                let note = BASS_NOTES[bass_chain.next(lfsr.next_u16())];
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_bass_spec(note, 0.64),
                );
            }
            if lead_pattern.is_active(step16) && step >= 32 && !phrase.is_multiple_of(4) {
                let note = LEAD_NOTES[lead_chain.next(lfsr.next_u16())];
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    gameplay_accent_spec(note, 0.20),
                );
            }
            if shimmer_pattern.is_active(step16) && step >= 64 {
                let note = LEAD_NOTES[lead_chain.next(lfsr.next_u16())] * 2.0;
                trigger_menu_sound(
                    &mut sound_voices,
                    &mut sound_cursor,
                    menu_shimmer_spec(note, 0.10),
                );
            }
        }

        let mut sample = 0.0f32;
        for slot in &mut patch_voices {
            sample += slot.voice.next_sample() as f32 / 32768.0 * slot.gain;
        }
        for voice in &mut sound_voices {
            sample += voice.next_sample() as f32 / 32768.0;
        }
        *mixed_sample = soft_clip(sample * 0.54);
    }

    for i in 0..tail_samples {
        mix[i] = soft_clip(mix[i] + mix[loop_samples + i]);
    }

    let samples: Vec<i16> = mix[..loop_samples]
        .iter()
        .map(|sample| sample_to_i16(*sample * 0.88))
        .collect();
    sound_data_from_mono_samples(GAME_SAMPLE_RATE, &samples)
}

fn trigger_menu_patch(voices: &mut [MenuPatchSlot], cursor: &mut usize, patch: Patch, gain: f32) {
    let start = *cursor;
    for offset in 0..voices.len() {
        let index = (start + offset) % voices.len();
        if !voices[index].voice.is_active() {
            voices[index].voice.trigger(patch);
            voices[index].gain = gain;
            *cursor = (index + 1) % voices.len();
            return;
        }
    }

    voices[*cursor].voice.trigger(patch);
    voices[*cursor].gain = gain;
    *cursor = (*cursor + 1) % voices.len();
}

fn trigger_menu_sound(voices: &mut [SoundVoice], cursor: &mut usize, spec: SoundSpec) {
    let start = *cursor;
    for offset in 0..voices.len() {
        let index = (start + offset) % voices.len();
        if !voices[index].is_active() {
            voices[index].trigger(spec);
            *cursor = (index + 1) % voices.len();
            return;
        }
    }

    voices[*cursor].trigger(spec);
    *cursor = (*cursor + 1) % voices.len();
}

fn menu_bass_spec(freq: f32, gain: f32) -> SoundSpec {
    SoundSpec::from_layers(
        360.0,
        3,
        [
            Layer::tone(
                Waveform::Sawtooth,
                gain * 0.34,
                330.0,
                freq,
                freq * 0.995,
                Curve::Hold,
                Curve::Decay,
            )
            .filtered(FilterMode::LowPass, 0.035),
            Layer::tone(
                Waveform::Square,
                gain * 0.14,
                240.0,
                freq * 0.5,
                freq * 0.5,
                Curve::Hold,
                Curve::Decay,
            )
            .filtered(FilterMode::LowPass, 0.028),
            Layer::tone(
                Waveform::Sine,
                gain * 0.32,
                320.0,
                freq,
                freq,
                Curve::Hold,
                Curve::Decay,
            ),
            Layer::silent(),
        ],
    )
}

fn menu_chord_spec(root: f32, gain: f32) -> SoundSpec {
    SoundSpec::from_layers(
        1180.0,
        4,
        [
            Layer::tone(
                Waveform::Sawtooth,
                gain * 0.12,
                1120.0,
                root,
                root * 1.002,
                Curve::Hold,
                Curve::Linear,
            )
            .filtered(FilterMode::LowPass, 0.018),
            Layer::tone(
                Waveform::Triangle,
                gain * 0.10,
                1080.0,
                root * 1.5,
                root * 1.502,
                Curve::Hold,
                Curve::Linear,
            )
            .filtered(FilterMode::LowPass, 0.014),
            Layer::tone(
                Waveform::Square,
                gain * 0.055,
                900.0,
                root * 2.0,
                root * 2.0,
                Curve::Hold,
                Curve::Linear,
            )
            .filtered(FilterMode::LowPass, 0.012),
            Layer::noise(0.018, 700.0, Curve::Linear, FilterMode::LowPass, 0.006).starting_at(90.0),
        ],
    )
}

fn menu_lead_spec(freq: f32, gain: f32) -> SoundSpec {
    SoundSpec::from_layers(
        220.0,
        3,
        [
            Layer::tone(
                Waveform::Triangle,
                gain * 0.30,
                200.0,
                freq,
                freq * 1.002,
                Curve::Hold,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Sawtooth,
                gain * 0.12,
                160.0,
                freq * 2.0,
                freq * 2.0,
                Curve::Hold,
                Curve::Decay,
            )
            .filtered(FilterMode::HighPass, 0.03),
            Layer::noise(0.018, 34.0, Curve::Linear, FilterMode::HighPass, 0.08),
            Layer::silent(),
        ],
    )
}

fn gameplay_pulse_spec(freq: f32, gain: f32) -> SoundSpec {
    SoundSpec::from_layers(
        180.0,
        3,
        [
            Layer::tone(
                Waveform::Sawtooth,
                gain * 0.20,
                160.0,
                freq,
                freq * 0.998,
                Curve::Hold,
                Curve::Decay,
            )
            .filtered(FilterMode::LowPass, 0.035),
            Layer::tone(
                Waveform::Square,
                gain * 0.08,
                120.0,
                freq * 0.5,
                freq * 0.5,
                Curve::Hold,
                Curve::Decay,
            )
            .filtered(FilterMode::LowPass, 0.026),
            Layer::noise(0.010, 32.0, Curve::Linear, FilterMode::HighPass, 0.09).starting_at(8.0),
            Layer::silent(),
        ],
    )
}

fn gameplay_accent_spec(freq: f32, gain: f32) -> SoundSpec {
    SoundSpec::from_layers(
        190.0,
        3,
        [
            Layer::tone(
                Waveform::Triangle,
                gain * 0.22,
                170.0,
                freq,
                freq * 1.002,
                Curve::Linear,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Sine,
                gain * 0.12,
                140.0,
                freq * 1.5,
                freq * 1.5,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(22.0),
            Layer::noise(0.010, 24.0, Curve::Linear, FilterMode::HighPass, 0.08),
            Layer::silent(),
        ],
    )
}

fn menu_shimmer_spec(freq: f32, gain: f32) -> SoundSpec {
    SoundSpec::from_layers(
        360.0,
        2,
        [
            Layer::tone(
                Waveform::Sine,
                gain * 0.22,
                320.0,
                freq,
                freq * 0.5,
                Curve::Linear,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Triangle,
                gain * 0.18,
                260.0,
                freq * 1.5,
                freq,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(12.0),
            Layer::silent(),
            Layer::silent(),
        ],
    )
}

fn soft_clip(sample: f32) -> f32 {
    sample / (1.0 + sample.abs())
}

fn sample_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

fn sfx_death_spec() -> SoundSpec {
    SoundSpec::from_layers(
        620.0,
        4,
        [
            Layer::tone(
                Waveform::Sine,
                0.42,
                560.0,
                196.0,
                41.2,
                Curve::Decay,
                Curve::Linear,
            ),
            Layer::tone(
                Waveform::Sawtooth,
                0.18,
                420.0,
                392.0,
                73.42,
                Curve::Decay,
                Curve::Linear,
            )
            .filtered(FilterMode::LowPass, 0.026),
            Layer::noise(0.13, 120.0, Curve::Decay, FilterMode::LowPass, 0.075),
            Layer::tone(
                Waveform::Triangle,
                0.16,
                260.0,
                1174.66,
                220.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(70.0),
        ],
    )
}

fn sfx_wall_slide_spec() -> SoundSpec {
    SoundSpec::from_layers(
        460.0,
        4,
        [
            Layer::tone(
                Waveform::Sine,
                0.24,
                430.0,
                3300.0,
                520.0,
                Curve::Linear,
                Curve::Linear,
            )
            .filtered(FilterMode::HighPass, 0.025),
            Layer::tone(
                Waveform::Triangle,
                0.16,
                390.0,
                2400.0,
                420.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(12.0)
            .filtered(FilterMode::HighPass, 0.020),
            Layer::tone(
                Waveform::Sawtooth,
                0.055,
                340.0,
                1800.0,
                360.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(28.0)
            .filtered(FilterMode::HighPass, 0.030),
            Layer::noise(0.030, 42.0, Curve::Linear, FilterMode::HighPass, 0.14),
        ],
    )
}

fn sfx_grapple_enemy_spec() -> SoundSpec {
    SoundSpec::from_layers(
        360.0,
        4,
        [
            Layer::tone(
                Waveform::Sine,
                0.22,
                320.0,
                3600.0,
                640.0,
                Curve::Linear,
                Curve::Linear,
            )
            .filtered(FilterMode::HighPass, 0.030),
            Layer::tone(
                Waveform::Triangle,
                0.15,
                260.0,
                2600.0,
                520.0,
                Curve::Linear,
                Curve::Linear,
            )
            .starting_at(18.0)
            .filtered(FilterMode::HighPass, 0.026),
            Layer::tone(
                Waveform::Sawtooth,
                0.055,
                210.0,
                1800.0,
                420.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(38.0)
            .filtered(FilterMode::HighPass, 0.038),
            Layer::noise(0.045, 28.0, Curve::Linear, FilterMode::HighPass, 0.16),
        ],
    )
}

fn render_ui_sound(spec: SoundSpec) -> Option<StaticSoundData> {
    render_sound_spec(spec, UI_SAMPLE_RATE)
}

fn render_sfx_sound(spec: SoundSpec) -> Option<StaticSoundData> {
    render_sound_spec(spec, UI_SAMPLE_RATE)
}

fn render_sound_spec(spec: SoundSpec, sample_rate: u32) -> Option<StaticSoundData> {
    let sample_count = ((spec.duration_ms * sample_rate as f32) / 1000.0)
        .ceil()
        .max(1.0) as usize;
    let mut voice = SoundVoice::new(sample_rate);
    let mut samples = Vec::with_capacity(sample_count);

    voice.trigger(spec);
    for i in 0..sample_count {
        let fade = edge_fade(i, sample_count, 64);
        samples.push((voice.next_sample() as f32 * fade) as i16);
    }

    sound_data_from_mono_samples(sample_rate, &samples)
}

fn edge_fade(index: usize, len: usize, fade_samples: usize) -> f32 {
    let fade_in = if fade_samples == 0 {
        1.0
    } else {
        (index as f32 / fade_samples as f32).clamp(0.0, 1.0)
    };
    let remaining = len.saturating_sub(index + 1);
    let fade_out = if fade_samples == 0 {
        1.0
    } else {
        (remaining as f32 / fade_samples as f32).clamp(0.0, 1.0)
    };

    fade_in.min(fade_out)
}

fn ui_hover_spec() -> SoundSpec {
    SoundSpec::from_layers(
        46.0,
        2,
        [
            Layer::tone(
                Waveform::Sine,
                0.16,
                42.0,
                1320.0,
                1760.0,
                Curve::Linear,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Triangle,
                0.08,
                32.0,
                2640.0,
                2200.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(5.0),
            Layer::silent(),
            Layer::silent(),
        ],
    )
}

fn ui_click_spec() -> SoundSpec {
    SoundSpec::from_layers(
        74.0,
        3,
        [
            Layer::tone(
                Waveform::Triangle,
                0.24,
                62.0,
                760.0,
                420.0,
                Curve::Decay,
                Curve::Decay,
            ),
            Layer::noise(0.10, 16.0, Curve::Linear, FilterMode::HighPass, 0.08),
            Layer::tone(
                Waveform::Sine,
                0.08,
                45.0,
                1500.0,
                900.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(9.0),
            Layer::silent(),
        ],
    )
}

fn ui_tab_change_spec() -> SoundSpec {
    SoundSpec::from_layers(
        118.0,
        3,
        [
            Layer::tone(
                Waveform::Sine,
                0.18,
                58.0,
                740.0,
                980.0,
                Curve::Linear,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Triangle,
                0.14,
                70.0,
                1110.0,
                1480.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(34.0),
            Layer::noise(0.035, 24.0, Curve::Linear, FilterMode::HighPass, 0.04).starting_at(6.0),
            Layer::silent(),
        ],
    )
}

fn ui_checkbox_on_spec() -> SoundSpec {
    SoundSpec::from_layers(
        96.0,
        3,
        [
            Layer::tone(
                Waveform::Triangle,
                0.20,
                82.0,
                520.0,
                1040.0,
                Curve::Linear,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Sine,
                0.10,
                54.0,
                1560.0,
                2080.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(18.0),
            Layer::noise(0.025, 18.0, Curve::Linear, FilterMode::HighPass, 0.07),
            Layer::silent(),
        ],
    )
}

fn ui_checkbox_off_spec() -> SoundSpec {
    SoundSpec::from_layers(
        92.0,
        3,
        [
            Layer::tone(
                Waveform::Triangle,
                0.18,
                76.0,
                740.0,
                360.0,
                Curve::Linear,
                Curve::Decay,
            ),
            Layer::tone(
                Waveform::Sine,
                0.08,
                48.0,
                520.0,
                390.0,
                Curve::Linear,
                Curve::Decay,
            )
            .starting_at(14.0),
            Layer::noise(0.035, 22.0, Curve::Linear, FilterMode::LowPass, 0.10),
            Layer::silent(),
        ],
    )
}
