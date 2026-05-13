use crate::clock::Transport;
use crate::random::Lfsr;
use crate::track::{Track, TrackEvent};

pub const MAX_TRACKS: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct SequencerOutput {
    pub events: [Option<TrackEvent>; MAX_TRACKS],
    pub step_triggered: bool,
    pub step_in_cycle: u32,
}

impl SequencerOutput {
    const fn empty() -> Self {
        Self {
            events: [None; MAX_TRACKS],
            step_triggered: false,
            step_in_cycle: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sequencer<const STEPS: usize> {
    pub transport: Transport,
    pub lfsr: Lfsr,
    tracks: [Option<Track<STEPS>>; MAX_TRACKS],
    track_count: usize,
    cycle_steps: u32,
}

impl<const STEPS: usize> Sequencer<STEPS> {
    pub fn new(sample_rate: u32, bpm: f32, cycle_steps: u32, seed: u16) -> Self {
        Self {
            transport: Transport::new(sample_rate, bpm),
            lfsr: Lfsr::new(seed),
            tracks: [const { None }; MAX_TRACKS],
            track_count: 0,
            cycle_steps: cycle_steps.max(1),
        }
    }

    pub fn add_track(&mut self, track: Track<STEPS>) -> Option<usize> {
        if self.track_count >= MAX_TRACKS {
            return None;
        }
        let index = self.track_count;
        self.tracks[index] = Some(track);
        self.track_count += 1;
        Some(index)
    }

    pub fn tick(&mut self) -> SequencerOutput {
        if !self.transport.tick() {
            return SequencerOutput::empty();
        }

        let step = self.transport.step_in_cycle(self.cycle_steps);
        let mut output = SequencerOutput {
            events: [None; MAX_TRACKS],
            step_triggered: true,
            step_in_cycle: step,
        };

        for i in 0..self.track_count {
            if let Some(track) = &mut self.tracks[i] {
                let seed = self.lfsr.next_u16();
                output.events[i] = track.query(step, seed);
            }
        }

        output
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.transport.set_bpm(bpm);
    }

    pub fn bpm(&self) -> f32 {
        self.transport.bpm()
    }

    pub const fn track_count(&self) -> usize {
        self.track_count
    }

    pub const fn cycle_steps(&self) -> u32 {
        self.cycle_steps
    }

    pub fn mute(&mut self, index: usize) {
        if let Some(Some(track)) = self.tracks.get_mut(index) {
            track.active = false;
        }
    }

    pub fn unmute(&mut self, index: usize) {
        if let Some(Some(track)) = self.tracks.get_mut(index) {
            track.active = true;
        }
    }

    pub fn is_track_active(&self, index: usize) -> bool {
        matches!(self.tracks.get(index), Some(Some(track)) if track.active)
    }

    pub fn current_sample(&self) -> u64 {
        self.transport.current_sample()
    }

    pub fn current_step_in_cycle(&self) -> u32 {
        self.transport.step_in_cycle(self.cycle_steps)
    }
}
