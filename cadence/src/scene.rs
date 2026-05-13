use crate::euclid::EuclideanPattern;
use crate::markov::MarkovChain;
use crate::sequencer::Sequencer;
use crate::track::{MAX_NOTES, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Drums,
    Melodic,
    Full,
}

impl Scene {
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Drums),
            1 => Some(Self::Melodic),
            2 => Some(Self::Full),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Drums => "Drums",
            Self::Melodic => "Melodic",
            Self::Full => "Full",
        }
    }
}

const PENTATONIC_A4: [f32; MAX_NOTES] = [
    220.0,  // A3
    261.63, // C4
    293.66, // D4
    329.63, // E4
    392.00, // G4
    440.0,  // A4
    523.25, // C5
    587.33, // D5
];

const BASS_NOTES: [f32; MAX_NOTES] = [
    55.0,   // A1
    65.41,  // C2
    73.42,  // D2
    82.41,  // E2
    98.00,  // G2
    110.0,  // A2
    130.81, // C3
    146.83, // D3
];

const MELODY_MATRIX: [[u8; MAX_NOTES]; MAX_NOTES] = [
    [0, 40, 10, 5, 0, 0, 0, 5],  // A3 → mostly C4
    [20, 0, 35, 10, 5, 0, 0, 0], // C4 → mostly D4
    [5, 20, 0, 35, 10, 0, 0, 0], // D4 → mostly E4
    [5, 5, 20, 0, 30, 10, 0, 0], // E4 → mostly G4
    [0, 5, 5, 20, 0, 35, 10, 0], // G4 → mostly A4
    [0, 0, 5, 5, 20, 0, 35, 10], // A4 → mostly C5
    [5, 0, 0, 5, 5, 25, 0, 30],  // C5 → mostly D5
    [10, 5, 0, 0, 5, 10, 30, 0], // D5 → wraps back down
];

const BASS_MATRIX: [[u8; MAX_NOTES]; MAX_NOTES] = [
    [0, 30, 10, 5, 20, 5, 0, 0],  // A1 → mostly C2, some G2
    [25, 0, 30, 10, 5, 0, 0, 0],  // C2 → A1 or D2
    [5, 20, 0, 35, 10, 0, 0, 0],  // D2 → mostly E2
    [10, 5, 20, 0, 30, 5, 0, 0],  // E2 → mostly G2
    [15, 5, 5, 15, 0, 25, 5, 0],  // G2 → A2 or back to A1
    [20, 0, 5, 5, 20, 0, 20, 0],  // A2 → root motion
    [5, 15, 0, 0, 10, 25, 0, 15], // C3 → mostly A2
    [10, 5, 15, 0, 5, 10, 25, 0], // D3 → mostly C3
];

pub fn drums<const S: usize>(sample_rate: u32, bpm: f32, seed: u16) -> Sequencer<S> {
    assert!(S >= 16, "Drums scene requires at least 16 steps");

    let mut seq = Sequencer::<S>::new(sample_rate, bpm, 16, seed);

    seq.add_track(Track::percussion(EuclideanPattern::new(4, 16, 0), 0, 0.95));

    seq.add_track(Track::percussion(EuclideanPattern::new(2, 16, 4), 1, 0.80));

    seq.add_track(Track::percussion(EuclideanPattern::new(8, 16, 0), 2, 0.55));

    seq.add_track(Track::percussion(EuclideanPattern::new(3, 16, 2), 2, 0.40));

    seq
}

pub fn melodic<const S: usize>(sample_rate: u32, bpm: f32, seed: u16) -> Sequencer<S> {
    assert!(S >= 16, "Melodic scene requires at least 16 steps");

    let mut seq = Sequencer::<S>::new(sample_rate, bpm, 16, seed);

    seq.add_track(Track::melody(
        EuclideanPattern::new(5, 16, 0),
        MarkovChain::new(MELODY_MATRIX, 0),
        PENTATONIC_A4,
        8,
        0.70,
    ));

    seq.add_track(Track::percussion(EuclideanPattern::new(2, 16, 0), 0, 0.60));

    seq.add_track(Track::percussion(EuclideanPattern::new(4, 16, 0), 2, 0.35));

    seq
}

pub fn full<const S: usize>(sample_rate: u32, bpm: f32, seed: u16) -> Sequencer<S> {
    assert!(S >= 16, "Full scene requires at least 16 steps");

    let mut seq = Sequencer::<S>::new(sample_rate, bpm, 16, seed);

    seq.add_track(Track::percussion(EuclideanPattern::new(4, 16, 0), 0, 0.90));

    seq.add_track(Track::percussion(EuclideanPattern::new(2, 16, 4), 1, 0.75));

    seq.add_track(Track::percussion(EuclideanPattern::new(8, 16, 0), 2, 0.45));

    seq.add_track(Track::melody(
        EuclideanPattern::new(3, 8, 0),
        MarkovChain::new(BASS_MATRIX, 0),
        BASS_NOTES,
        8,
        0.80,
    ));

    seq.add_track(Track::melody(
        EuclideanPattern::new(5, 16, 3),
        MarkovChain::new(MELODY_MATRIX, 0),
        PENTATONIC_A4,
        8,
        0.65,
    ));

    seq
}

pub fn build<const S: usize>(scene: Scene, sample_rate: u32, bpm: f32, seed: u16) -> Sequencer<S> {
    match scene {
        Scene::Drums => drums(sample_rate, bpm, seed),
        Scene::Melodic => melodic(sample_rate, bpm, seed),
        Scene::Full => full(sample_rate, bpm, seed),
    }
}
