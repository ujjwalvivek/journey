/**--------------------------------------------------------------------------------
*!  Fixed-timestep timing system for deterministic game logic.
*?  Provides a monotonic tick counter and accumulator that drives fixed-rate
*?  updates independently of the display framerate.
*--------------------------------------------------------------------------------**/
pub const DEFAULT_FIXED_HZ: u32 = 60;
pub const MAX_STEPS: u32 = 5; //* Cap to prevent spiral of death on very slow frames.

//? Fixed-timestep timing state.
#[derive(Debug, Clone)]
pub struct FixedTime {
    //* Monotonic tick counter
    pub tick: u64,
    pub fixed_dt: f32,
    tick_rate: u32,
    //* Leftover time from the previous frame, carried into the next.
    accumulator: f32,
    //* Frame-perfect freeze counter. While > 0, the accumulator is not fed
    //* and no fixed steps run. Decrements once per render frame.
    freeze_frames: u16,
}

impl FixedTime {
    pub fn new(hz: u32) -> Self {
        Self {
            tick: 0,
            fixed_dt: 1.0 / hz as f32,
            tick_rate: hz,
            accumulator: 0.0,
            freeze_frames: 0,
        }
    }

    //? Feed wall-clock delta time. Returns the number of fixed steps to run
    //? this frame. During a freeze, returns 0 and decrements the counter.
    pub fn accumulate(&mut self, dt: f32) -> u32 {
        if self.freeze_frames > 0 {
            self.freeze_frames -= 1;
            return 0;
        }
        self.accumulator += dt;
        (self.accumulator / self.fixed_dt).min(MAX_STEPS as f32) as u32
    }

    //? Call this once per fixed_update invocation.
    pub fn advance(&mut self) {
        self.accumulator -= self.fixed_dt;
        self.tick += 1;
    }

    //? Fraction of a fixed step remaining after all whole steps have been consumed.
    //? Useful for render-time interpolation between physics states.
    pub fn interpolation_alpha(&self) -> f32 {
        self.accumulator / self.fixed_dt
    }

    pub fn tick_rate(&self) -> u32 {
        self.tick_rate
    }

    pub fn set_tick_rate(&mut self, hz: u32) {
        self.tick_rate = hz;
        self.fixed_dt = 1.0 / hz as f32;
    }

    //? FSM and physics pause.
    pub fn freeze(&mut self, frames: u16) {
        self.freeze_frames = self.freeze_frames.max(frames);
    }

    pub fn is_frozen(&self) -> bool {
        self.freeze_frames > 0
    }

    pub fn freeze_remaining(&self) -> u16 {
        self.freeze_frames
    }
}

impl Default for FixedTime {
    fn default() -> Self {
        Self::new(DEFAULT_FIXED_HZ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_step_at_exact_dt() {
        let mut ft = FixedTime::new(60);
        let steps = ft.accumulate(1.0 / 60.0);
        assert_eq!(steps, 1);
        ft.advance();
        assert_eq!(ft.tick, 1);
    }

    #[test]
    fn no_step_below_threshold() {
        let mut ft = FixedTime::new(60);
        let steps = ft.accumulate(0.005); //* ~5ms, less than 16.67ms
        assert_eq!(steps, 0);
        assert_eq!(ft.tick, 0);
    }

    #[test]
    fn multiple_steps_on_slow_frame() {
        let mut ft = FixedTime::new(60);
        let steps = ft.accumulate(1.0 / 30.0); //* 33ms → 2 steps at 60Hz
        assert_eq!(steps, 2);
        ft.advance();
        ft.advance();
        assert_eq!(ft.tick, 2);
    }

    #[test]
    fn accumulator_carries_remainder() {
        let mut ft = FixedTime::new(60);
        let dt = 1.0 / 60.0;
        //* Feed 1.5 fixed steps worth of time
        ft.accumulate(dt * 1.5);
        ft.advance(); //* consume one step
        assert_eq!(ft.tick, 1);
        //* Remaining should be ~0.5 * dt
        let alpha = ft.interpolation_alpha();
        assert!((alpha - 0.5).abs() < 0.01, "alpha was {alpha}");
    }

    #[test]
    fn set_tick_rate_changes_dt() {
        let mut ft = FixedTime::new(60);
        ft.set_tick_rate(30);
        assert_eq!(ft.tick_rate(), 30);
        assert!((ft.fixed_dt - 1.0 / 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tick_monotonically_increases() {
        let mut ft = FixedTime::new(60);
        let dt = 1.0 / 60.0;
        for expected in 1..=100u64 {
            ft.accumulate(dt);
            ft.advance();
            assert_eq!(ft.tick, expected);
        }
    }

    #[test]
    fn freeze_blocks_accumulation() {
        let mut ft = FixedTime::new(60);
        ft.freeze(3);
        assert!(ft.is_frozen());
        //? Each accumulate call during freeze returns 0 and decrements counter
        assert_eq!(ft.accumulate(1.0 / 60.0), 0);
        assert_eq!(ft.freeze_remaining(), 2);
        assert_eq!(ft.accumulate(1.0 / 60.0), 0);
        assert_eq!(ft.freeze_remaining(), 1);
        assert_eq!(ft.accumulate(1.0 / 60.0), 0);
        assert_eq!(ft.freeze_remaining(), 0);
        assert!(!ft.is_frozen());
        //? After freeze, normal accumulation resumes
        assert_eq!(ft.accumulate(1.0 / 60.0), 1);
    }

    #[test]
    fn freeze_stacks_by_max() {
        let mut ft = FixedTime::new(60);
        ft.freeze(3);
        ft.freeze(5);
        assert_eq!(ft.freeze_remaining(), 5);
        ft.freeze(2); //* smaller, ignored
        assert_eq!(ft.freeze_remaining(), 5);
    }
}
