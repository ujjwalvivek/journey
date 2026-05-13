#![no_std]

//! `cadence` does not render audio. It decides when musical or sound events
//! should happen, then a sink such as `resonance` turns those events into PCM.

pub mod clock;
pub mod euclid;
pub mod markov;
pub mod random;
pub mod scene;
pub mod sequencer;
pub mod track;

pub use clock::Transport;
pub use euclid::EuclideanPattern;
pub use markov::MarkovChain;
pub use random::Lfsr;
pub use scene::Scene;
pub use sequencer::{Sequencer, SequencerOutput, MAX_TRACKS};
pub use track::{Track, TrackAction, TrackEvent};
