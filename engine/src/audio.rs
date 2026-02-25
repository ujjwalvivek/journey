/**------------------------------------------------------------------------------------------
*!  Cross-platform audio manager wrapping Kira.
*?  Provides lazy initialization (required for WASM where Web Audio API needs a
*?  user gesture before it can start), named sub-tracks for independent volume
*?  control, and a simple event-driven API for one-shot SFX.
*?  Architecture:
*?  - The `AudioManager` wraps `Option<kira::AudioManager>` for lazy/graceful init.
*?  - Four sub-tracks: Music, Ambience, SFX, UI each with independent volume.
*?  - Music/Ambience use handle tracking to prevent overlapping loops.
*?  - One-shot SFX are fire-and-forget via `AudioEvent` queue drained per frame.
*?  - And, `AudioResponse` extension trait auto-wires egui widgets to UI sounds.
*------------------------------------------------------------------------------------------**/
use kira::AudioManager as KiraManager;
use kira::AudioManagerSettings;
use kira::DefaultBackend;
use kira::Tween;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::{TrackBuilder, TrackHandle};
use std::io::Cursor;
use std::time::Duration;

//? Identifies which sub-track a sound should play on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTrack {
    Music,
    Ambience,
    Sfx,
    Ui,
}

//? One-shot audio event queued during `fixed_update` and drained once per frame.
//? Game code pushes these into `Context::pending_audio`; the engine drains them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    UiHover,
    UiClick,
    UiTabChange,
    UiCheckboxOn,
    UiCheckboxOff,
}

struct Tracks {
    music: TrackHandle,
    ambience: TrackHandle,
    sfx: TrackHandle,
    ui: TrackHandle,
}

//? Active looping sound handles to prevent overlapping playback.
struct ActiveSounds {
    current_music: Option<StaticSoundHandle>,
    current_ambience: Option<StaticSoundHandle>,
    current_loop_sfx: Option<StaticSoundHandle>,
}

//? Engine-level audio manager. All methods are no-ops when the backend is unavailable.
pub struct AudioManager {
    inner: Option<KiraManager<DefaultBackend>>,
    tracks: Option<Tracks>,
    active: ActiveSounds,
    master_volume: f64,
    music_volume: f64,
    ambience_volume: f64,
    sfx_volume: f64,
    ui_volume: f64,
    init_attempted: bool,
    #[cfg(target_arch = "wasm32")]
    awaiting_user_gesture: bool,
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioManager {
    pub fn new() -> Self {
        let mgr = Self {
            inner: None,
            tracks: None,
            active: ActiveSounds {
                current_music: None,
                current_ambience: None,
                current_loop_sfx: None,
            },
            master_volume: 1.0,
            music_volume: 1.0,
            ambience_volume: 1.0,
            sfx_volume: 1.0,
            ui_volume: 1.0,
            init_attempted: false,
            #[cfg(target_arch = "wasm32")]
            awaiting_user_gesture: true,
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut mgr = mgr;
            mgr.try_init();
            mgr
        }
        #[cfg(target_arch = "wasm32")]
        {
            mgr
        }
    }

    //? Attempt to initialize the Kira backend. Safe to call multiple times.
    //? Subsequent calls after a successful init are no-ops.
    fn try_init(&mut self) {
        if self.inner.is_some() {
            return;
        }
        self.init_attempted = true;

        match KiraManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(mut manager) => {
                let music = manager.add_sub_track(TrackBuilder::default());
                let ambience = manager.add_sub_track(TrackBuilder::default());
                let sfx = manager.add_sub_track(TrackBuilder::default());
                let ui = manager.add_sub_track(TrackBuilder::default());

                match (music, ambience, sfx, ui) {
                    (Ok(music), Ok(ambience), Ok(sfx), Ok(ui)) => {
                        self.tracks = Some(Tracks {
                            music,
                            ambience,
                            sfx,
                            ui,
                        });
                        self.inner = Some(manager);
                        log::info!("Audio system initialized successfully");
                    }
                    _ => {
                        log::warn!("Audio: failed to create sub-tracks");
                    }
                }
            }
            Err(e) => {
                log::warn!("Audio: backend init failed ({e}), will retry on next play");
            }
        }
    }

    //? Ensure the backend is alive, retrying init if needed (WASM gesture unlock).
    fn ensure_init(&mut self) -> bool {
        #[cfg(target_arch = "wasm32")]
        if self.awaiting_user_gesture {
            //* Don't create/start AudioContext until a user gesture.
            return false;
        }

        if self.inner.is_none() {
            self.try_init();
        }
        self.inner.is_some()
    }

    //? Notify the audio system that a user gesture occurred.
    pub fn notify_user_gesture(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if self.awaiting_user_gesture {
                self.awaiting_user_gesture = false;
                self.try_init();
            }
        }
    }

    fn track_handle(&mut self, track: AudioTrack) -> Option<&mut TrackHandle> {
        self.tracks.as_mut().map(|t| match track {
            AudioTrack::Music => &mut t.music,
            AudioTrack::Ambience => &mut t.ambience,
            AudioTrack::Sfx => &mut t.sfx,
            AudioTrack::Ui => &mut t.ui,
        })
    }

    //? Returns the final amplitude for a track.
    //* Use this when computing live volume targets for ducking/unducking.
    pub fn effective_volume(&self, track: AudioTrack) -> f64 {
        let track_vol = match track {
            AudioTrack::Music => self.music_volume,
            AudioTrack::Ambience => self.ambience_volume,
            AudioTrack::Sfx => self.sfx_volume,
            AudioTrack::Ui => self.ui_volume,
        };
        self.master_volume * track_vol
    }

    //? True when a looping music handle is currently active.
    pub fn has_active_music(&self) -> bool {
        self.active.current_music.is_some()
    }

    //? True when a looping ambience handle is currently active.
    pub fn has_active_ambience(&self) -> bool {
        self.active.current_ambience.is_some()
    }

    //? Convert linear amplitude (0.0–1.0) to kira `Decibels`.
    fn amplitude_to_db(amp: f64) -> kira::Decibels {
        if amp <= 0.0 {
            kira::Decibels::SILENCE
        } else {
            kira::Decibels((20.0 * (amp as f32).log10()).max(-60.0))
        }
    }

    pub fn play_oneshot(&mut self, data: &StaticSoundData, track: AudioTrack) {
        if !self.ensure_init() {
            return;
        }
        let vol = self.effective_volume(track);
        if let Some(track_handle) = self.track_handle(track) {
            let sound = data.clone().volume(Self::amplitude_to_db(vol));
            let _ = track_handle.play(sound);
        }
    }

    //? Play looping music. If music is already playing, crossfade.
    pub fn play_music(&mut self, data: &StaticSoundData, fade_in_secs: f32) {
        if !self.ensure_init() {
            return;
        }
        self.stop_music(fade_in_secs);

        let vol = self.effective_volume(AudioTrack::Music);
        let sound = data
            .clone()
            .loop_region(..)
            .volume(Self::amplitude_to_db(vol))
            .fade_in_tween(Tween {
                duration: Duration::from_secs_f32(fade_in_secs),
                ..Default::default()
            });

        if let Some(track_handle) = self.track_handle(AudioTrack::Music) {
            match track_handle.play(sound) {
                Ok(handle) => self.active.current_music = Some(handle),
                Err(e) => log::warn!("Audio: failed to play music: {e}"),
            }
        }
    }

    pub fn stop_music(&mut self, fade_out_secs: f32) {
        if let Some(ref mut handle) = self.active.current_music {
            handle.stop(Tween {
                duration: Duration::from_secs_f32(fade_out_secs),
                ..Default::default()
            });
        }
        self.active.current_music = None;
    }

    //? Play looping ambience. If already playing, crossfade.
    pub fn play_ambience(&mut self, data: &StaticSoundData, fade_in_secs: f32) {
        if !self.ensure_init() {
            return;
        }
        self.stop_ambience(fade_in_secs);

        let vol = self.effective_volume(AudioTrack::Ambience);
        let sound = data
            .clone()
            .loop_region(..)
            .volume(Self::amplitude_to_db(vol))
            .fade_in_tween(Tween {
                duration: Duration::from_secs_f32(fade_in_secs),
                ..Default::default()
            });

        if let Some(track_handle) = self.track_handle(AudioTrack::Ambience) {
            match track_handle.play(sound) {
                Ok(handle) => self.active.current_ambience = Some(handle),
                Err(e) => log::warn!("Audio: failed to play ambience: {e}"),
            }
        }
    }

    pub fn stop_ambience(&mut self, fade_out_secs: f32) {
        if let Some(ref mut handle) = self.active.current_ambience {
            handle.stop(Tween {
                duration: Duration::from_secs_f32(fade_out_secs),
                ..Default::default()
            });
        }
        self.active.current_ambience = None;
    }

    pub fn stop_loop_sfx(&mut self, fade_out_secs: f32) {
        if let Some(ref mut handle) = self.active.current_loop_sfx {
            handle.stop(Tween {
                duration: Duration::from_secs_f32(fade_out_secs),
                ..Default::default()
            });
        }
        self.active.current_loop_sfx = None;
    }

    //? Play a looping SFX. Stops any existing loop SFX first.
    pub fn play_loop_sfx(&mut self, data: &StaticSoundData) {
        if !self.ensure_init() {
            return;
        }
        self.stop_loop_sfx(0.05);
        let vol = self.effective_volume(AudioTrack::Sfx);
        let sound = data.clone().loop_region(..);
        if let Some(track_handle) = self.track_handle(AudioTrack::Sfx) {
            match track_handle.play(sound.volume(Self::amplitude_to_db(vol))) {
                Ok(handle) => self.active.current_loop_sfx = Some(handle),
                Err(e) => log::warn!("Audio: loop sfx failed: {e}"),
            }
        }
    }

    //? Smooth ducking/unducking.
    pub fn set_music_live_volume(&mut self, amp: f64, fade_secs: f32) {
        if let Some(ref mut handle) = self.active.current_music {
            handle.set_volume(
                Self::amplitude_to_db(amp.clamp(0.0, 1.0)),
                Tween {
                    duration: Duration::from_secs_f32(fade_secs),
                    ..Default::default()
                },
            );
        }
    }

    //? Live-update the volume of the currently-playing ambience handle.
    pub fn set_ambience_live_volume(&mut self, amp: f64, fade_secs: f32) {
        if let Some(ref mut handle) = self.active.current_ambience {
            handle.set_volume(
                Self::amplitude_to_db(amp.clamp(0.0, 1.0)),
                Tween {
                    duration: Duration::from_secs_f32(fade_secs),
                    ..Default::default()
                },
            );
        }
    }

    pub fn set_master_volume(&mut self, volume: f64) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_music_volume(&mut self, volume: f64) {
        self.music_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_ambience_volume(&mut self, volume: f64) {
        self.ambience_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_sfx_volume(&mut self, volume: f64) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_ui_volume(&mut self, volume: f64) {
        self.ui_volume = volume.clamp(0.0, 1.0);
    }

    pub fn master_volume(&self) -> f64 {
        self.master_volume
    }
    pub fn music_volume(&self) -> f64 {
        self.music_volume
    }
    pub fn ambience_volume(&self) -> f64 {
        self.ambience_volume
    }
    pub fn sfx_volume(&self) -> f64 {
        self.sfx_volume
    }
    pub fn ui_volume(&self) -> f64 {
        self.ui_volume
    }
}

//? Load a `StaticSoundData` from embedded bytes (compatible with `include_bytes!`).
pub fn load_sound_data(bytes: &'static [u8]) -> Option<StaticSoundData> {
    match StaticSoundData::from_cursor(Cursor::new(bytes)) {
        Ok(data) => Some(data),
        Err(e) => {
            log::warn!("Audio: failed to decode sound: {e}");
            None
        }
    }
}

//? Extension trait for `egui::Response` that queues UI audio events automatically.
//* `if ui.button("Play").with_ui_sound(&mut ctx.pending_audio).clicked() { ... }`
pub trait AudioResponse {
    fn with_ui_sound(self, pending: &mut Vec<AudioEvent>) -> Self;
    fn with_checkbox_sound(self, checked: bool, pending: &mut Vec<AudioEvent>) -> Self;
    fn with_tab_sound(self, pending: &mut Vec<AudioEvent>) -> Self;
}

impl AudioResponse for egui::Response {
    fn with_ui_sound(self, pending: &mut Vec<AudioEvent>) -> Self {
        let id = self.id;
        let was_hovered = self.ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
        let now_hovered = self.hovered();
        self.ctx.data_mut(|d| d.insert_temp(id, now_hovered));
        if now_hovered && !was_hovered {
            pending.push(AudioEvent::UiHover);
        }
        if self.clicked() {
            pending.push(AudioEvent::UiClick);
        }
        self
    }

    fn with_checkbox_sound(self, checked: bool, pending: &mut Vec<AudioEvent>) -> Self {
        if self.changed() {
            pending.push(if checked {
                AudioEvent::UiCheckboxOn
            } else {
                AudioEvent::UiCheckboxOff
            });
        }
        self
    }

    fn with_tab_sound(self, pending: &mut Vec<AudioEvent>) -> Self {
        if self.clicked() {
            pending.push(AudioEvent::UiTabChange);
        }
        self
    }
}
