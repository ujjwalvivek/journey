/**---------------------------------------------------------------------------
*!  Grid-based spritesheet animation system.
*?  Each animation references a range of frames within a single spritesheet
*?  grid and calculates frame rects dynamically.
*---------------------------------------------------------------------------**/
use engine::Rect;

#[derive(Clone)]
pub struct Animation {
    pub name: String,
    pub start_frame: usize,
    pub frame_count: usize,
    pub frame_duration: f32,
    pub looping: bool,
}

impl Animation {
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

    pub fn new_with_range(
        name: impl Into<String>,
        start_frame: usize,
        end_frame: usize,
        frame_duration: f32,
        looping: bool,
    ) -> Self {
        Self {
            name: name.into(),
            start_frame,
            frame_count: (end_frame - start_frame) + 1,
            frame_duration,
            looping,
        }
    }

    pub fn get_frame_rect(
        &self,
        frame_idx: usize,
        frame_width: f32,
        frame_height: f32,
        sheet_cols: usize,
    ) -> Rect {
        let actual_frame = self.start_frame + frame_idx;
        let col = actual_frame % sheet_cols;
        let row = actual_frame / sheet_cols;
        Rect::new(
            col as f32 * frame_width,
            row as f32 * frame_height,
            frame_width,
            frame_height,
        )
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

    pub fn current_frame(
        &self,
        frame_width: f32,
        frame_height: f32,
        sheet_cols: usize,
    ) -> Option<Rect> {
        let (def, frame_idx) = self.inner.current()?;
        let anim = self.animations.iter().find(|a| a.name == def.name)?;
        Some(anim.get_frame_rect(frame_idx, frame_width, frame_height, sheet_cols))
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
