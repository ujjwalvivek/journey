/**----------------------------------------------------------------------
*!  Common math helpers used across the engine and game code.
*?  Move `current` towards `target` by at most `max_delta`.
*?  Mirrors Unity's `Mathf.MoveTowards`: if the distance is smaller than
*?  `max_delta` the result snaps exactly to `target`, avoiding overshooting.
*----------------------------------------------------------------------**/
pub fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + max_delta.copysign(target - current)
    }
}

//? Unit tests for `move_towards` to verify correct behavior in various scenarios.
#[cfg(test)]
mod tests {
    //? Import the parent module to access `move_towards`.
    use super::*;

    //* Test that it snaps to target when within max_delta.
    #[test]
    fn snap_when_close() {
        assert_eq!(move_towards(9.5, 10.0, 1.0), 10.0);
    }

    //* Test that it moves by max_delta when far from target.
    #[test]
    fn advance_positive() {
        assert!((move_towards(0.0, 10.0, 3.0) - 3.0).abs() < f32::EPSILON);
    }

    //* Test that it moves by max_delta when far from target in the negative direction.
    #[test]
    fn advance_negative() {
        assert!((move_towards(5.0, 0.0, 2.0) - 3.0).abs() < f32::EPSILON);
    }

    //* Test that it does not overshoot the target when within max_delta.
    #[test]
    fn zero_delta_no_movement() {
        assert!((move_towards(5.0, 10.0, 0.0) - 5.0).abs() < f32::EPSILON);
    }
}
