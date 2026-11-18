//! No-op audio used when the `audio` feature (kira) is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTrack {
    Music,
    Ambience,
    Sfx,
    Ui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UiAudioEvent {
    Hover,
    Click,
    CheckboxOn,
    CheckboxOff,
    TabChange,
}

#[derive(Default)]
pub struct AudioManager;

impl AudioManager {
    pub fn new() -> Self {
        Self
    }
    pub fn notify_user_gesture(&mut self) {}
    pub fn has_active_music(&self) -> bool {
        false
    }
    pub fn has_active_ambience(&self) -> bool {
        false
    }
}
