#![no_std]

pub mod buffer;
pub mod envelope;
pub mod lut;
pub mod oscillator;
pub mod patch;
pub mod sound;

#[cfg(feature = "convert")]
extern crate std;

#[cfg(feature = "convert")]
pub mod convert;
