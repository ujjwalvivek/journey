/**----------------------------------------------------------------------
*!  Audio asset management: loads all game sounds from embedded bytes.
*?  Uses `include_bytes!` for cross-platform compatibility (native + WASM).
*?  Sound data is loaded once during init and stored for the game lifetime.
*----------------------------------------------------------------------**/
use engine::{AudioManager, AudioTrack, StaticSoundData, UiAudioEvent, load_sound_data};

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
    pub ui_level_editor: Option<StaticSoundData>,
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
            start_audio: load_sound_data(include_bytes!("../assets/audio/start_audio.ogg")),
            bg_music: load_sound_data(include_bytes!("../assets/audio/bg_music.ogg")),
            ambient_audio: load_sound_data(include_bytes!("../assets/audio/ambient_audio.ogg")),
            ui_level_editor: load_sound_data(include_bytes!("../assets/audio/ui_level_editor.ogg")),

            ui_hover: load_sound_data(include_bytes!("../assets/audio/ui_hover.ogg")),
            ui_click: load_sound_data(include_bytes!("../assets/audio/ui_click.ogg")),
            ui_tab_change: load_sound_data(include_bytes!("../assets/audio/ui_tab_change.ogg")),
            ui_checkbox_on: load_sound_data(include_bytes!("../assets/audio/ui_checkbox_on.ogg")),
            ui_checkbox_off: load_sound_data(include_bytes!("../assets/audio/ui_checkbox_off.ogg")),

            sfx_jump: load_sound_data(include_bytes!("../assets/audio/sfx_jump.ogg")),
            sfx_land: load_sound_data(include_bytes!("../assets/audio/sfx_land.ogg")),
            sfx_dash: load_sound_data(include_bytes!("../assets/audio/sfx_dash.ogg")),
            sfx_run: load_sound_data(include_bytes!("../assets/audio/sfx_run.ogg")),
            sfx_wall_grab: load_sound_data(include_bytes!("../assets/audio/sfx_wall_grab.ogg")),
            sfx_wall_slide: load_sound_data(include_bytes!("../assets/audio/sfx_wall_slide.ogg")),

            sfx_swing: load_sound_data(include_bytes!("../assets/audio/sfx_swing.ogg")),
            sfx_hit: load_sound_data(include_bytes!("../assets/audio/sfx_hit.ogg")),
            sfx_parry: load_sound_data(include_bytes!("../assets/audio/sfx_parry.ogg")),
            sfx_stagger: load_sound_data(include_bytes!("../assets/audio/sfx_stagger.ogg")),
            sfx_death: load_sound_data(include_bytes!("../assets/audio/sfx_death.ogg")),
            sfx_respawn: load_sound_data(include_bytes!("../assets/audio/sfx_respawn.ogg")),

            sfx_grapple_static: load_sound_data(include_bytes!(
                "../assets/audio/sfx_grapple_static.ogg"
            )),
            sfx_grapple_enemy: load_sound_data(include_bytes!(
                "../assets/audio/sfx_grapple_enemy.ogg"
            )),

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
