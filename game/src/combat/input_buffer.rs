/**--------------------------------------------------------------------------------
*!  Tick-stamped combat input buffer.
*?  Bridges the gap between per-frame input sampling and fixed-rate FSM updates.
*?  Inputs are queued with a tick stamp and consumed within a configurable
*?  frame window, ensuring rapid clicks are never lost between fixed steps.
*--------------------------------------------------------------------------------**/
use super::fsm::{self, CombatState};
use super::moves::{MoveDatabase, MoveId};
use std::collections::VecDeque;

//? Default buffer window in fixed ticks (20 frames at 60Hz ≈ 333ms).
pub const DEFAULT_BUFFER_WINDOW: u16 = 20;

#[derive(Debug, Clone, Copy)]
pub struct BufferedAction {
    pub action: MoveId,
    pub tick_pressed: u64,
}

//? Queue of recent combat inputs, consumed by the FSM each fixed tick.
#[derive(Debug, Clone)]
pub struct CombatInputBuffer {
    queue: VecDeque<BufferedAction>,
    pub buffer_window: u16,
}

impl Default for CombatInputBuffer {
    fn default() -> Self {
        Self {
            queue: VecDeque::with_capacity(8),
            buffer_window: DEFAULT_BUFFER_WINDOW,
        }
    }
}

impl CombatInputBuffer {
    //? Push a new combat action into the buffer with the current tick.
    pub fn push(&mut self, action: MoveId, tick: u64) {
        self.queue.push_back(BufferedAction {
            action,
            tick_pressed: tick,
        });
    }

    //? Remove expired entries older than `buffer_window` ticks.
    pub fn expire(&mut self, current_tick: u64) {
        let window = self.buffer_window as u64;
        self.queue
            .retain(|b| current_tick.saturating_sub(b.tick_pressed) <= window);
    }

    //? Try to consume the oldest valid buffered action that can transition.
    //? Returns the MoveId if a valid action was found and consumed.
    pub fn consume(
        &mut self,
        state: &CombatState,
        move_db: &MoveDatabase,
        current_tick: u64,
    ) -> Option<MoveId> {
        self.expire(current_tick);

        let mut found_idx = None;
        for (i, buffered) in self.queue.iter().enumerate() {
            if fsm::can_transition(state, buffered.action, move_db) {
                found_idx = Some((i, buffered.action));
                break;
            }
        }

        if let Some((idx, move_id)) = found_idx {
            self.queue.remove(idx);
            Some(move_id)
        } else {
            None
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    //? Peek at the oldest buffered action without consuming it.
    pub fn peek(&self) -> Option<&BufferedAction> {
        self.queue.front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    //? Returns true if the buffer has no pending actions.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    //? Check if any unexpired attack action is buffered (for execute-attack timing).
    pub fn has_attack(&self, current_tick: u64) -> bool {
        let window = self.buffer_window as u64;
        self.queue.iter().any(|b| {
            current_tick.saturating_sub(b.tick_pressed) <= window
                && matches!(
                    b.action,
                    MoveId::AttackHorizontal | MoveId::AttackUp | MoveId::AttackDown
                )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_push_and_consume() {
        let mut buf = CombatInputBuffer::default();
        let state = CombatState::default(); //* Idle
        let db = MoveDatabase::default();

        buf.push(MoveId::AttackHorizontal, 100);
        assert!(buf.has_pending());

        let consumed = buf.consume(&state, &db, 100);
        assert_eq!(consumed, Some(MoveId::AttackHorizontal));
        assert!(!buf.has_pending());
    }

    #[test]
    fn buffer_expires_old_inputs() {
        let mut buf = CombatInputBuffer::default();
        buf.push(MoveId::AttackHorizontal, 10);

        buf.expire(31);
        assert!(!buf.has_pending());
    }

    #[test]
    fn buffer_keeps_fresh_inputs() {
        let mut buf = CombatInputBuffer::default();
        buf.push(MoveId::AttackHorizontal, 100);

        buf.expire(118);
        assert!(buf.has_pending());
    }

    #[test]
    fn buffer_rejects_invalid_transition() {
        let mut buf = CombatInputBuffer::default();
        let db = MoveDatabase::default();

        //? In startup   can't transition
        let mut state = CombatState::default();
        fsm::begin_move(&mut state, MoveId::AttackHorizontal, &db);
        assert_eq!(state.phase, fsm::CombatPhase::Startup);

        buf.push(MoveId::AttackUp, 100);
        let consumed = buf.consume(&state, &db, 100);
        assert_eq!(consumed, None);
        assert!(buf.has_pending());
    }

    #[test]
    fn multiple_inputs_consumed_in_order() {
        let mut buf = CombatInputBuffer::default();
        let state = CombatState::default(); //* Idle
        let db = MoveDatabase::default();

        buf.push(MoveId::AttackHorizontal, 100);
        buf.push(MoveId::Parry, 101);

        let first = buf.consume(&state, &db, 101);
        assert_eq!(first, Some(MoveId::AttackHorizontal));

        let second = buf.consume(&state, &db, 101);
        assert_eq!(second, Some(MoveId::Parry));
    }
}
