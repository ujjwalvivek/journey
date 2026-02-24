/**--------------------------------------------------------------------------------
*!  Frame-deterministic combat finite state machine.
*?  Tracks the current combat phase (Idle/Startup/Active/Recovery), a tick
*?  counter that increments each fixed step, and the current move. Transitions
*?  are explicit and driven entirely by integer frame counts from MoveData.
*--------------------------------------------------------------------------------**/
use super::moves::{MoveDatabase, MoveId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatPhase {
    Idle,
    Startup,
    Active,
    Recovery,
}

#[derive(Debug, Clone)]
pub struct CombatState {
    pub phase: CombatPhase,
    pub frame_timer: u16,
    pub current_move: Option<MoveId>,
    pub invincible: bool,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            phase: CombatPhase::Idle,
            frame_timer: 0,
            current_move: None,
            invincible: false,
        }
    }
}

impl CombatState {
    pub fn is_idle(&self) -> bool {
        self.phase == CombatPhase::Idle
    }

    pub fn is_active(&self) -> bool {
        self.phase == CombatPhase::Active
    }

    pub fn in_cancel_window(&self, move_db: &MoveDatabase) -> bool {
        if self.phase != CombatPhase::Recovery {
            return false;
        }
        if let Some(move_id) = self.current_move {
            let data = move_db.get(move_id);
            self.frame_timer >= data.cancel_window_start()
        } else {
            false
        }
    }
}

//? Advance the FSM by one tick. Auto-transitions between phases
//? when frame_timer reaches the phase duration boundary.
//? Returns the phase transition that occurred, if any.
pub fn advance_combat_fsm(state: &mut CombatState, move_db: &MoveDatabase) -> Option<CombatPhase> {
    if state.phase == CombatPhase::Idle {
        return None;
    }

    let move_id = state.current_move?;

    state.frame_timer += 1;
    let data = move_db.get(move_id);

    let old_phase = state.phase;

    match state.phase {
        CombatPhase::Startup => {
            if state.frame_timer >= data.active_start() {
                state.phase = CombatPhase::Active;
                //? Set i-frames for Dash
                state.invincible = move_id == MoveId::Dash;
            }
        }
        CombatPhase::Active => {
            if state.frame_timer >= data.recovery_start() {
                state.phase = CombatPhase::Recovery;
                state.invincible = false;
            }
            //? Dash i-frames end at half the active phase
            if move_id == MoveId::Dash {
                let iframe_end = data.active_frames / 2;
                if state.frame_timer >= iframe_end {
                    state.invincible = false;
                }
            }
        }
        CombatPhase::Recovery => {
            if state.frame_timer >= data.total_frames() {
                state.phase = CombatPhase::Idle;
                state.current_move = None;
                state.frame_timer = 0;
                state.invincible = false;
            }
        }
        CombatPhase::Idle => {}
    }

    //? Handle moves with 0 recovery (Dash) ending at active phase end
    if move_id == MoveId::Dash && state.phase == CombatPhase::Recovery && data.recovery_frames == 0
    {
        state.phase = CombatPhase::Idle;
        state.current_move = None;
        state.frame_timer = 0;
        state.invincible = false;
    }

    if state.phase != old_phase {
        Some(state.phase)
    } else {
        None
    }
}

//? Check whether a transition to `input_move` is valid from the current state.
pub fn can_transition(state: &CombatState, _input_move: MoveId, move_db: &MoveDatabase) -> bool {
    match state.phase {
        CombatPhase::Idle => true,
        CombatPhase::Recovery => state.in_cancel_window(move_db),
        CombatPhase::Startup | CombatPhase::Active => false,
    }
}

//? Begin executing a new move. Resets frame_timer, sets phase to Startup
//? (or Active if startup_frames == 0 at the current tick rate).
pub fn begin_move(state: &mut CombatState, move_id: MoveId, move_db: &MoveDatabase) {
    state.current_move = Some(move_id);
    state.frame_timer = 0;
    state.invincible = false;

    let data = move_db.get(move_id);
    if data.startup_frames == 0 {
        state.phase = CombatPhase::Active;
        state.invincible = move_id == MoveId::Dash;
    } else {
        state.phase = CombatPhase::Startup;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> MoveDatabase {
        MoveDatabase::default()
    }

    #[test]
    fn idle_can_start_any_move() {
        let state = CombatState::default();
        let db = db();
        assert!(can_transition(&state, MoveId::AttackHorizontal, &db));
        assert!(can_transition(&state, MoveId::Parry, &db));
        assert!(can_transition(&state, MoveId::Dash, &db));
    }

    #[test]
    fn startup_blocks_all_transitions() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::AttackHorizontal, &db);
        assert_eq!(state.phase, CombatPhase::Startup);
        assert!(!can_transition(&state, MoveId::AttackUp, &db));
        assert!(!can_transition(&state, MoveId::Parry, &db));
    }

    #[test]
    fn attack_horizontal_full_lifecycle() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::AttackHorizontal, &db);

        //? Startup: frames 0-2 (3 frames)
        assert_eq!(state.phase, CombatPhase::Startup);
        for _ in 0..2 {
            advance_combat_fsm(&mut state, &db);
        }
        assert_eq!(state.phase, CombatPhase::Startup);

        //? Frame 3 → Active
        let transition = advance_combat_fsm(&mut state, &db);
        assert_eq!(transition, Some(CombatPhase::Active));

        //? Active: frames 3-5 (3 frames)
        for _ in 0..2 {
            advance_combat_fsm(&mut state, &db);
        }
        assert_eq!(state.phase, CombatPhase::Active);

        //? Frame 6 → Recovery
        let transition = advance_combat_fsm(&mut state, &db);
        assert_eq!(transition, Some(CombatPhase::Recovery));

        //? Recovery: frames 6-13 (8 frames)
        for _ in 0..7 {
            advance_combat_fsm(&mut state, &db);
        }
        assert_eq!(state.phase, CombatPhase::Recovery);

        //? Frame 14 → Idle
        let transition = advance_combat_fsm(&mut state, &db);
        assert_eq!(transition, Some(CombatPhase::Idle));
        assert!(state.is_idle());
        assert!(state.current_move.is_none());
    }

    #[test]
    fn parry_starts_in_active_phase() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::Parry, &db);
        assert_eq!(state.phase, CombatPhase::Active);
    }

    #[test]
    fn dash_has_iframes_then_loses_them() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::Dash, &db);
        assert_eq!(state.phase, CombatPhase::Active);
        assert!(state.invincible);

        //? Advance to half active (8/2 = 4)
        for _ in 0..3 {
            advance_combat_fsm(&mut state, &db);
        }
        assert!(state.invincible);

        //? Frame 4 → i-frames end
        advance_combat_fsm(&mut state, &db);
        assert!(!state.invincible);

        //? Advance to end (0 recovery → Idle)
        for _ in 0..3 {
            advance_combat_fsm(&mut state, &db);
        }
        advance_combat_fsm(&mut state, &db);
        assert!(state.is_idle());
    }

    #[test]
    fn cancel_window_allows_transition() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::AttackHorizontal, &db);

        //? Advance into recovery (frame 6)
        for _ in 0..6 {
            advance_combat_fsm(&mut state, &db);
        }
        assert_eq!(state.phase, CombatPhase::Recovery);

        //? Before cancel window (frame < 10)
        assert!(!can_transition(&state, MoveId::AttackUp, &db));

        //? Advance to cancel window (frame 10)
        for _ in 0..4 {
            advance_combat_fsm(&mut state, &db);
        }
        assert_eq!(state.frame_timer, 10);
        assert!(state.in_cancel_window(&db));
        assert!(can_transition(&state, MoveId::AttackUp, &db));
        assert!(can_transition(&state, MoveId::Parry, &db));
    }

    #[test]
    fn cancel_into_new_move_during_window() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::AttackHorizontal, &db);

        //? Advance to cancel window (frame 10)
        for _ in 0..10 {
            advance_combat_fsm(&mut state, &db);
        }
        assert!(state.in_cancel_window(&db));

        //? Cancel into AttackUp
        assert!(can_transition(&state, MoveId::AttackUp, &db));
        begin_move(&mut state, MoveId::AttackUp, &db);
        assert_eq!(state.current_move, Some(MoveId::AttackUp));
        assert_eq!(state.frame_timer, 0);
    }

    #[test]
    fn frame_timer_counts_up() {
        let mut state = CombatState::default();
        let db = db();
        begin_move(&mut state, MoveId::AttackHorizontal, &db);
        assert_eq!(state.frame_timer, 0);
        advance_combat_fsm(&mut state, &db);
        assert_eq!(state.frame_timer, 1);
        advance_combat_fsm(&mut state, &db);
        assert_eq!(state.frame_timer, 2);
    }
}
