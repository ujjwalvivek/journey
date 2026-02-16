/**---------------------------------------------------------------------------
*!  Animation system for sprite strip animations.
*?  Each animation references a sprite sheet (AssetKey) and calculates
*?  frame rects dynamically based on frame count.
*---------------------------------------------------------------------------**/
use engine::Rect;

//? Which sprite sheet to use for an animation
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssetKey {
    Idle,
    Run,
    Jump,
    Fall,
    Attack,
    Block,
    Roll,
}

//? A single animation (references a sprite strip texture).
#[derive(Clone)]
pub struct Animation {
    pub name: String,
    pub asset_key: AssetKey,
    pub start_frame: usize,  //* Starting frame index in the strip
    pub frame_count: usize,  //* Total frames in this animation
    pub frame_duration: f32, //* Duration per frame in seconds
    pub looping: bool,
}

//? Create a simple animation that uses all frames in a sprite strip.
impl Animation {
    //? Convert game `Animation` (contains `AssetKey`) into `AnimationDef` used by the runtime.
    fn to_def(&self) -> engine::animation::AnimationDef {
        engine::animation::AnimationDef::new(
            &self.name,
            self.start_frame,
            self.frame_count,
            self.frame_duration,
            self.looping,
        )
    }

    pub fn new(
        name: impl Into<String>,
        asset_key: AssetKey,
        frame_count: usize,
        frame_duration: f32,
        looping: bool,
    ) -> Self {
        Self {
            name: name.into(),
            asset_key,
            start_frame: 0,
            frame_count,
            frame_duration,
            looping,
        }
    }

    //? Create an animation that uses a range of frames within a sprite strip.
    //* Used for attacks where multiple animations share one sprite sheet.
    pub fn new_with_range(
        name: impl Into<String>,
        asset_key: AssetKey,
        start_frame: usize,
        end_frame: usize,
        frame_duration: f32,
        looping: bool,
    ) -> Self {
        Self {
            name: name.into(),
            asset_key,
            start_frame,
            frame_count: (end_frame - start_frame) + 1,
            frame_duration,
            looping,
        }
    }

    //? Calculate the source rect for a given frame index
    //* Assumes horizontal sprite strip (all frames in one row) !IMPORTANT
    pub fn get_frame_rect(&self, frame_idx: usize, frame_width: f32, frame_height: f32) -> Rect {
        let actual_frame = self.start_frame + frame_idx;
        let x = actual_frame as f32 * frame_width;
        Rect::new(x, 0.0, frame_width, frame_height)
    }
}

//? Runtime animation state machine that tracks the current animation and frame based on time.
pub struct AnimationState {
    inner: engine::animation::AnimationState,
    animations: Vec<Animation>,
}

impl AnimationState {
    pub fn new(animations: Vec<Animation>, default_anim: &str) -> Self {
        let defs = animations.iter().map(|a| a.to_def()).collect();
        Self {
            inner: engine::animation::AnimationState::new(defs, default_anim),
            animations,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.inner.update(dt);
    }

    //? Get the current frame rectangle and asset key (keeps previous API).
    pub fn current_frame(&self, frame_width: f32, frame_height: f32) -> Option<(AssetKey, Rect)> {
        let (def, frame_idx) = self.inner.current()?;
        let anim = self.animations.iter().find(|a| a.name == def.name)?;
        let rect = anim.get_frame_rect(frame_idx, frame_width, frame_height);
        Some((anim.asset_key, rect))
    }

    //? Animation control methods (play new animation, check if finished, etc.)
    pub fn play(&mut self, anim_name: &str) {
        self.inner.play(anim_name);
    }
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
    pub fn current_animation_name(&self) -> Option<&str> {
        self.inner.current_animation_name()
    }
    pub fn get_progress(&self) -> f32 {
        self.inner.get_progress()
    }
}
