/**--------------------------------------------------------------------------------
*!  Engine-level, asset-agnostic animation runtime.
*?  This module contains the generic animation definition (`AnimationDef`) and
*?  the runtime/state machine (`AnimationState`) that drive frame timing and
*?  animation switching. It is deliberately asset-agnostic so the `game` crate
*?  can keep its `AssetKey` enum and map frames->textures locally.
*--------------------------------------------------------------------------------**/
#[derive(Clone, Debug)]
pub struct AnimationDef {
    pub name: String,
    pub start_frame: usize,
    pub frame_count: usize,
    pub frame_duration: f32,
    pub looping: bool,
}

impl AnimationDef {
    pub fn new(
        name: impl Into<String>,
        start_frame: usize,
        frame_count: usize,
        frame_duration: f32,
        looping: bool,
    ) -> Self {
        Self {
            name: name.into(),
            start_frame,
            frame_count,
            frame_duration,
            looping,
        }
    }
}

//? Animation state machine. Does not know about textures or asset keys.
pub struct AnimationState {
    pub current_anim: String,
    pub frame_index: usize,
    pub timer: f32,
    animations: Vec<AnimationDef>,
}

//? Create a new runtime state from a list of `AnimationDef`s and a default name.
impl AnimationState {
    pub fn new(animations: Vec<AnimationDef>, default_anim: &str) -> Self {
        Self {
            current_anim: default_anim.to_string(),
            frame_index: 0,
            timer: 0.0,
            animations,
        }
    }

    //? Advance the timer and update the current frame index accordingly.
    pub fn update(&mut self, dt: f32) {
        let anim = match self.animations.iter().find(|a| a.name == self.current_anim) {
            Some(a) => a,
            None => return,
        };

        self.timer += dt;

        if self.timer >= anim.frame_duration {
            self.timer -= anim.frame_duration;
            self.frame_index += 1;

            if self.frame_index >= anim.frame_count {
                if anim.looping {
                    self.frame_index = 0;
                } else {
                    self.frame_index = anim.frame_count.saturating_sub(1);
                }
            }
        }
    }

    //? Return the current animation definition and the active frame index.
    //* Useful for callers that know how to map `AnimationDef` -> texture/rect.
    pub fn current(&self) -> Option<(&AnimationDef, usize)> {
        let anim = self
            .animations
            .iter()
            .find(|a| a.name == self.current_anim)?;
        Some((anim, self.frame_index))
    }

    //? Switch to a different animation by name (resets frame/timer).
    pub fn play(&mut self, anim_name: &str) {
        if self.current_anim != anim_name {
            self.current_anim = anim_name.to_string();
            self.frame_index = 0;
            self.timer = 0.0;
        }
    }

    //? Returns true if the current anim is non-looping and has reached its last frame.
    pub fn is_finished(&self) -> bool {
        let anim = match self.animations.iter().find(|a| a.name == self.current_anim) {
            Some(a) => a,
            None => return true,
        };

        !anim.looping && self.frame_index >= anim.frame_count.saturating_sub(1)
    }

    //? Name of the currently playing animation.
    pub fn current_animation_name(&self) -> Option<&str> {
        Some(&self.current_anim)
    }

    //? Progress through the current animation as a value in [0.0, 1.0].
    pub fn get_progress(&self) -> f32 {
        let anim = self.animations.iter().find(|a| a.name == self.current_anim);

        if let Some(anim) = anim {
            if anim.frame_count == 0 {
                return 0.0;
            }
            let total_frames = anim.frame_count;
            let progress_per_frame = 1.0 / total_frames as f32;
            let frame_progress = self.timer / anim.frame_duration;
            (self.frame_index as f32 + frame_progress) * progress_per_frame
        } else {
            0.0
        }
    }
}
