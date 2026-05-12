use crate::lut::{
    NUM_BANDS, PHASE_INDEX_SHIFT, SAWTOOTH_LUTS, SINE_LUT, SQUARE_LUTS, TABLE_SIZE, TRIANGLE_LUTS,
};

const PHASE_SCALE: f64 = 18_446_744_073_709_551_616.0; //* 2^64

pub fn phase_increment(frequency: f32, sample_rate: u32) -> u64 {
    let ratio = frequency as f64 / sample_rate as f64;
    (ratio * PHASE_SCALE) as u64
}

pub fn octave_for_freq(freq_hz: f32) -> usize {
    if freq_hz < 40.0 {
        0
    } else if freq_hz < 80.0 {
        1
    } else if freq_hz < 160.0 {
        2
    } else if freq_hz < 320.0 {
        3
    } else if freq_hz < 640.0 {
        4
    } else if freq_hz < 1280.0 {
        5
    } else if freq_hz < 2560.0 {
        6
    } else if freq_hz < 5120.0 {
        7
    } else if freq_hz < 10240.0 {
        8
    } else {
        9
    }
}

#[inline(always)]
fn lut_index(phase: u64) -> usize {
    (phase >> PHASE_INDEX_SHIFT) as usize % TABLE_SIZE
}

#[inline]
pub fn sine(phase: u64) -> i16 {
    SINE_LUT[lut_index(phase)]
}

#[inline]
pub fn square(phase: u64, band: usize) -> i16 {
    SQUARE_LUTS[band.min(NUM_BANDS - 1)][lut_index(phase)]
}

#[inline]
pub fn sawtooth(phase: u64, band: usize) -> i16 {
    SAWTOOTH_LUTS[band.min(NUM_BANDS - 1)][lut_index(phase)]
}

#[inline]
pub fn triangle(phase: u64, band: usize) -> i16 {
    TRIANGLE_LUTS[band.min(NUM_BANDS - 1)][lut_index(phase)]
}

pub struct Noise {
    lfsr: u16,
}

impl Noise {
    pub const fn new() -> Self {
        Self { lfsr: 0xACE1 }
    }

    pub const fn with_seed(seed: u16) -> Self {
        Self {
            lfsr: if seed == 0 { 0xACE1 } else { seed },
        }
    }

    #[inline]
    pub fn next_sample(&mut self) -> i16 {
        let bit = self.lfsr & 1;
        self.lfsr >>= 1;
        if bit == 1 {
            self.lfsr ^= 0xB400; //* taps at 16, 14, 13, 11
        }
        self.lfsr as i16
    }
}

impl Default for Noise {
    fn default() -> Self {
        Self::new()
    }
}
