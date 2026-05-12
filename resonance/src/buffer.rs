use crate::envelope::{Adsr, AdsrState};

pub struct PcmBuffer<const N: usize> {
    pub samples: [i16; N],
    pub len: usize,
}

impl<const N: usize> PcmBuffer<N> {
    pub const fn new() -> Self {
        Self {
            samples: [0i16; N],
            len: 0,
        }
    }
    pub fn fill(
        &mut self,
        osc_fn: fn(u64) -> i16,
        phase_inc: u64,
        phase: &mut u64,
        envelope: &Adsr,
        env_state: &mut AdsrState,
        sample_rate: u32,
    ) {
        let ms_per_sample = 1000.0 / sample_rate as f32;

        for i in 0..N {
            let gain = env_state.tick(envelope, ms_per_sample);
            let raw = osc_fn(*phase);
            self.samples[i] = (raw as f32 * gain) as i16;
            *phase = phase.wrapping_add(phase_inc);
        }
        self.len = N;
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The banded variant mirrors fill while adding the wavetable band selector."
    )]
    pub fn fill_banded(
        &mut self,
        osc_fn: fn(u64, usize) -> i16,
        band: usize,
        phase_inc: u64,
        phase: &mut u64,
        envelope: &Adsr,
        env_state: &mut AdsrState,
        sample_rate: u32,
    ) {
        let ms_per_sample = 1000.0 / sample_rate as f32;

        for i in 0..N {
            let gain = env_state.tick(envelope, ms_per_sample);
            let raw = osc_fn(*phase, band);
            self.samples[i] = (raw as f32 * gain) as i16;
            *phase = phase.wrapping_add(phase_inc);
        }
        self.len = N;
    }
    pub fn fill_raw(&mut self, osc_fn: fn(u64) -> i16, phase_inc: u64, phase: &mut u64) {
        for i in 0..N {
            self.samples[i] = osc_fn(*phase);
            *phase = phase.wrapping_add(phase_inc);
        }
        self.len = N;
    }
    pub fn as_slice(&self) -> &[i16] {
        &self.samples[..self.len]
    }
}

impl<const N: usize> Default for PcmBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn mix(inputs: &[&[i16]], output: &mut [i16], len: usize) {
    for i in 0..len {
        let mut sum: i32 = 0;
        for input in inputs {
            sum += input[i] as i32;
        }
        output[i] = if sum > 32767 {
            32767
        } else if sum < -32768 {
            -32768
        } else {
            sum as i16
        };
    }
}
