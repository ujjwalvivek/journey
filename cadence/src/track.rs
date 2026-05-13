use crate::euclid::EuclideanPattern;
use crate::markov::MarkovChain;

pub const MAX_NOTES: usize = 8;

#[derive(Debug, Clone, Copy)]
pub enum TrackAction {
    Patch(u8),
    Melody {
        chain: MarkovChain<MAX_NOTES>,
        frequencies: [f32; MAX_NOTES],
        note_count: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackEvent {
    TriggerPatch { patch_index: u8, gain: f32 },
    TriggerNote { frequency: f32, gain: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct Track<const STEPS: usize> {
    pub pattern: EuclideanPattern<STEPS>,
    pub action: TrackAction,
    pub gain: f32,
    pub active: bool,
}

impl<const STEPS: usize> Track<STEPS> {
    pub const fn percussion(pattern: EuclideanPattern<STEPS>, patch_index: u8, gain: f32) -> Self {
        Self {
            pattern,
            action: TrackAction::Patch(patch_index),
            gain,
            active: true,
        }
    }

    pub fn melody(
        pattern: EuclideanPattern<STEPS>,
        chain: MarkovChain<MAX_NOTES>,
        frequencies: [f32; MAX_NOTES],
        note_count: usize,
        gain: f32,
    ) -> Self {
        Self {
            pattern,
            action: TrackAction::Melody {
                chain,
                frequencies,
                note_count: note_count.min(MAX_NOTES),
            },
            gain,
            active: true,
        }
    }

    pub fn query(&mut self, step_in_cycle: u32, random_seed: u16) -> Option<TrackEvent> {
        if !self.active {
            return None;
        }

        if !self.pattern.is_active(step_in_cycle as usize) {
            return None;
        }

        match &mut self.action {
            TrackAction::Patch(index) => Some(TrackEvent::TriggerPatch {
                patch_index: *index,
                gain: self.gain,
            }),
            TrackAction::Melody {
                chain,
                frequencies,
                note_count,
            } => {
                let state = chain.next(random_seed);
                let clamped = if *note_count == 0 { 0 } else { state % *note_count };
                Some(TrackEvent::TriggerNote {
                    frequency: frequencies[clamped],
                    gain: self.gain,
                })
            }
        }
    }
}
