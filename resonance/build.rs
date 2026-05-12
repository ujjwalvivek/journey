//! build.rs: Runs on the host machine at compile time.
//! Generates lookup tables for oscillator waveforms.

use std::env;
use std::f64::consts::{PI, TAU};
use std::fs;
use std::path::Path;

const TABLE_SIZE: usize = 1024;
const MAX_AMP: f64 = 32767.0; //* i16::MAX

const STANDARD_HARMONICS_PER_BAND: [usize; 10] = [512, 256, 128, 64, 32, 16, 8, 4, 2, 1];

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("lut.rs");
    let harmonics_per_band: &[usize] = &STANDARD_HARMONICS_PER_BAND;

    let mut output = String::with_capacity(1024 * 1024); //* ~1MB pre-alloc

    output.push_str(&format!("pub const TABLE_SIZE: usize = {};\n", TABLE_SIZE));
    output.push_str(&format!(
        "pub const PHASE_INDEX_SHIFT: u32 = {};\n",
        64 - TABLE_SIZE.trailing_zeros()
    ));
    output.push_str(&format!(
        "pub const NUM_BANDS: usize = {};\n\n",
        harmonics_per_band.len()
    ));

    generate_sine_lut(&mut output);
    generate_banded_luts(&mut output, "SQUARE", harmonics_per_band, square_sample);
    generate_banded_luts(&mut output, "SAWTOOTH", harmonics_per_band, sawtooth_sample);
    generate_banded_luts(&mut output, "TRIANGLE", harmonics_per_band, triangle_sample);
    fs::write(&dest_path, output).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}

fn generate_sine_lut(output: &mut String) {
    output.push_str(&format!(
        "pub const SINE_LUT: [i16; {}] = [\n    ",
        TABLE_SIZE
    ));
    for i in 0..TABLE_SIZE {
        let phase = (i as f64 / TABLE_SIZE as f64) * TAU;
        let sample = (phase.sin() * MAX_AMP).round() as i16;
        output.push_str(&format!("{}, ", sample));
        if (i + 1) % 16 == 0 {
            output.push_str("\n    ");
        }
    }
    output.push_str("];\n\n");
}

fn generate_banded_luts(
    output: &mut String,
    name: &str,
    harmonics_per_band: &[usize],
    sample_fn: fn(f64, usize) -> f64,
) {
    let num_bands = harmonics_per_band.len();
    output.push_str(&format!(
        "pub static {}_LUTS: [[i16; {}]; {}] = [\n",
        name, TABLE_SIZE, num_bands
    ));

    for &max_harmonics in harmonics_per_band {
        //* compute raw f64 values and find peak for normalization
        let mut raw = [0.0_f64; TABLE_SIZE];
        let mut peak = 0.0_f64;

        for (i, sample) in raw.iter_mut().enumerate() {
            let t = i as f64 / TABLE_SIZE as f64;
            *sample = sample_fn(t, max_harmonics);
            let abs = sample.abs();
            if abs > peak {
                peak = abs;
            }
        }

        //* normalize to i16 range
        let scale = if peak > 0.0 { MAX_AMP / peak } else { 0.0 };

        output.push_str("    [\n        ");
        for (i, sample) in raw.iter().enumerate() {
            let sample = (sample * scale).round() as i16;
            output.push_str(&format!("{}, ", sample));
            if (i + 1) % 16 == 0 {
                output.push_str("\n        ");
            }
        }
        output.push_str("],\n");
    }

    output.push_str("];\n\n");
}

//? Square wave: (4/π) Σ sin((2k+1)θ) / (2k+1)  for k=0..N
fn square_sample(t: f64, max_harmonics: usize) -> f64 {
    let theta = t * TAU;
    let mut sum = 0.0;
    let mut k = 0_usize;
    loop {
        let n = 2 * k + 1; //* odd harmonics only
        if n > max_harmonics {
            break;
        }
        sum += (n as f64 * theta).sin() / n as f64;
        k += 1;
    }
    sum * 4.0 / PI
}

//? Sawtooth wave: -(2/π) Σ (-1)^n sin(nθ) / n  for n=1..N
fn sawtooth_sample(t: f64, max_harmonics: usize) -> f64 {
    let theta = t * TAU;
    let mut sum = 0.0;
    for n in 1..=max_harmonics {
        let sign = if n.is_multiple_of(2) { 1.0 } else { -1.0 }; //* (-1)^n
        sum += sign * (n as f64 * theta).sin() / n as f64;
    }
    sum * -2.0 / PI
}

//? Triangle wave: (8/π²) Σ (-1)^k sin((2k+1)θ) / (2k+1)²  for k=0..N
fn triangle_sample(t: f64, max_harmonics: usize) -> f64 {
    let theta = t * TAU;
    let mut sum = 0.0;
    let mut k = 0_usize;
    loop {
        let n = 2 * k + 1; //* odd harmonics only
        if n > max_harmonics {
            break;
        }
        let sign = if k.is_multiple_of(2) { 1.0 } else { -1.0 }; //* (-1)^k
        sum += sign * (n as f64 * theta).sin() / (n as f64 * n as f64);
        k += 1;
    }
    sum * 8.0 / (PI * PI)
}
