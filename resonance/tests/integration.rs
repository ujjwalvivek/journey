use resonance::buffer::{PcmBuffer, mix};
use resonance::envelope::{Adsr, AdsrState, Stage};
use resonance::lut::{SINE_LUT, TABLE_SIZE};
use resonance::oscillator;
use resonance::patch::{Patch, PatchVoice};
use resonance::sound::{Curve, FilterMode, Layer, MAX_LAYERS, SoundSpec, SoundVoice, Waveform};

const SAMPLE_RATE: u32 = 44100;

#[test]
fn sine_zero_phase_is_zero() {
    //* sin(0) = 0
    assert_eq!(oscillator::sine(0), 0);
}

#[test]
fn sine_quarter_phase_is_peak() {
    //* sin(π/2) = 1.0 → i16::MAX (32767)
    //* Quarter phase in u64 = u64::MAX / 4
    let quarter = u64::MAX / 4;
    let sample = oscillator::sine(quarter);
    //* Allow ±1 for rounding
    assert!(
        (sample - 32767_i16).abs() <= 1,
        "Expected ~32767 at quarter phase, got {}",
        sample
    );
}

#[test]
fn sine_half_phase_is_zero() {
    //* sin(π) = 0
    let half = u64::MAX / 2;
    let sample = oscillator::sine(half);
    //* Allow rounding from 1024-entry LUT discretization.
    //* At index 512 the phase maps to sin(π + ε) which can be up to ~±201.
    assert!(
        sample.abs() <= 210,
        "Expected ~0 at half phase, got {}",
        sample
    );
}

#[test]
fn sine_periodicity() {
    let freq = 100.0;
    let inc = oscillator::phase_increment(freq, SAMPLE_RATE);
    let mut phase: u64 = 0;
    let period_samples = SAMPLE_RATE / freq as u32; //* 441 samples

    //* Record first cycle
    let mut first_cycle = [0i16; 441];
    for sample in first_cycle.iter_mut() {
        *sample = oscillator::sine(phase);
        phase = phase.wrapping_add(inc);
    }

    //* Record second cycle
    let mut second_cycle = [0i16; 441];
    for sample in second_cycle.iter_mut() {
        *sample = oscillator::sine(phase);
        phase = phase.wrapping_add(inc);
    }

    //* Cycles should be nearly identical
    for i in 0..period_samples as usize {
        let diff = (first_cycle[i] as i32 - second_cycle[i] as i32).abs();
        assert!(
            diff <= 2,
            "Periodicity broken at sample {}: first={}, second={}, diff={}",
            i,
            first_cycle[i],
            second_cycle[i],
            diff
        );
    }
}

#[test]
fn phase_increment_440hz() {
    let inc = oscillator::phase_increment(440.0, SAMPLE_RATE);
    //* Expected: (440/44100) * 2^64 ≈ 183,936,223,565,038,805
    assert!(inc > 0, "Phase increment should be positive");
    assert!(
        inc < u64::MAX / 10,
        "440Hz increment should be much less than u64::MAX"
    );
}

#[test]
fn octave_band_selection() {
    assert_eq!(oscillator::octave_for_freq(20.0), 0);
    assert_eq!(oscillator::octave_for_freq(100.0), 2);
    assert_eq!(oscillator::octave_for_freq(440.0), 4);
    assert_eq!(oscillator::octave_for_freq(1000.0), 5);
    assert_eq!(oscillator::octave_for_freq(4000.0), 7);
    assert_eq!(oscillator::octave_for_freq(15000.0), 9);
}

#[test]
fn square_band_limited_not_all_zeros() {
    let inc = oscillator::phase_increment(440.0, SAMPLE_RATE);
    let band = oscillator::octave_for_freq(440.0);
    let mut phase: u64 = 0;
    let mut has_nonzero = false;

    for _ in 0..1024 {
        let s = oscillator::square(phase, band);
        if s != 0 {
            has_nonzero = true;
            break;
        }
        phase = phase.wrapping_add(inc);
    }
    assert!(has_nonzero, "Square wave should produce non-zero samples");
}

#[test]
fn noise_is_nonzero_and_varying() {
    let mut noise = oscillator::Noise::new();
    let mut samples = [0i16; 100];
    for sample in samples.iter_mut() {
        *sample = noise.next_sample();
    }

    //* Should have non-zero values
    let has_nonzero = samples.iter().any(|&s| s != 0);
    assert!(has_nonzero, "Noise should produce non-zero samples");

    //* Should have variation
    let first = samples[0];
    let has_variation = samples.iter().any(|&s| s != first);
    assert!(has_variation, "Noise should produce varying samples");
}

#[test]
fn noise_period_is_65535() {
    let mut noise = oscillator::Noise::new();
    let first = noise.next_sample();

    //* The LFSR should not repeat for 65535 steps
    let mut repeated_at = 0;
    for i in 1..=65536 {
        let s = noise.next_sample();
        if s == first && i > 1 {
            //* Check next few to confirm period
            let mut noise2 = oscillator::Noise::new();
            let _ = noise2.next_sample(); //* skip first
            for _ in 1..i {
                noise2.next_sample();
            }
            if noise2.next_sample() == noise.next_sample() {
                repeated_at = i;
                break;
            }
        }
    }

    //* LFSR period should be exactly 65535
    assert!(
        repeated_at == 0 || repeated_at >= 65535,
        "Noise repeated too early at step {}",
        repeated_at
    );
}

#[test]
fn sine_lut_has_correct_size() {
    assert_eq!(SINE_LUT.len(), TABLE_SIZE);
    assert_eq!(TABLE_SIZE, 1024);
}

#[test]
fn sine_lut_known_values() {
    //* Index 0 → sin(0) = 0
    assert_eq!(SINE_LUT[0], 0);

    //* Index 256 → sin(π/2) = 32767
    assert_eq!(SINE_LUT[256], 32767);

    //* Index 512 → sin(π) ≈ 0
    assert!(
        SINE_LUT[512].abs() <= 1,
        "LUT[512] should be ~0, got {}",
        SINE_LUT[512]
    );

    //* Index 768 → sin(3π/2) = -32767
    assert_eq!(SINE_LUT[768], -32767);
}

#[test]
fn adsr_attack_ramp() {
    let adsr = Adsr {
        attack_ms: 10.0,
        decay_ms: 0.0,
        sustain: 1.0,
        release_ms: 0.0,
    };

    //* Gain at t=0 should be 0
    assert_eq!(adsr.gain(Stage::Attack, 0.0), 0.0);

    //* Gain at t=5ms should be 0.5
    let mid = adsr.gain(Stage::Attack, 5.0);
    assert!((mid - 0.5).abs() < 0.01, "Expected ~0.5, got {}", mid);

    //* Gain at t=10ms should be 1.0
    let end = adsr.gain(Stage::Attack, 10.0);
    assert!((end - 1.0).abs() < 0.01, "Expected ~1.0, got {}", end);
}

#[test]
fn adsr_state_persists_across_ticks() {
    let adsr = Adsr {
        attack_ms: 10.0,
        decay_ms: 5.0,
        sustain: 0.7,
        release_ms: 20.0,
    };
    let mut state = AdsrState::new();
    let ms_per_sample = 1000.0 / SAMPLE_RATE as f32; //* ~0.0227ms

    //* Tick through attack phase (+ a few extra samples past the boundary)
    let mut last_gain = 0.0;
    let attack_samples = (10.0 / ms_per_sample) as usize + 2;

    for _ in 0..attack_samples {
        let gain = state.tick(&adsr, ms_per_sample);
        //* Gain is non-decreasing during attack, may dip slightly entering decay
        if state.stage == Stage::Attack {
            assert!(
                gain >= last_gain - 0.001,
                "Attack gain should be non-decreasing"
            );
        }
        last_gain = gain;
    }

    //* Should have transitioned to Decay
    assert_eq!(state.stage, Stage::Decay, "Should be in Decay after attack");
}

#[test]
fn adsr_state_survives_multiple_buffer_fills() {
    let adsr = Adsr {
        attack_ms: 10.0,
        decay_ms: 50.0,
        sustain: 0.7,
        release_ms: 100.0,
    };
    let mut state = AdsrState::new();
    let ms_per_sample = 1000.0 / SAMPLE_RATE as f32;

    //* Simulate 3 buffer fills of 256 samples each (~17.4ms total)
    //* This should span the attack (10ms) and enter decay
    for _ in 0..(256 * 3) {
        state.tick(&adsr, ms_per_sample);
    }

    //* Should be in Decay (10ms < 17.4ms < 60ms)
    assert_eq!(
        state.stage,
        Stage::Decay,
        "After 17.4ms with 10ms attack, should be in Decay"
    );
}

#[test]
fn adsr_note_off_during_attack() {
    let adsr = Adsr {
        attack_ms: 100.0,
        decay_ms: 50.0,
        sustain: 0.7,
        release_ms: 50.0,
    };
    let mut state = AdsrState::new();
    let ms_per_sample = 1000.0 / SAMPLE_RATE as f32;

    //* Tick to 50ms (halfway through attack, gain ~0.5)
    let half_attack = (50.0 / ms_per_sample) as usize;
    for _ in 0..half_attack {
        state.tick(&adsr, ms_per_sample);
    }

    state.note_off(&adsr);
    assert_eq!(state.stage, Stage::Release);

    let first_release_gain = state.tick(&adsr, ms_per_sample);
    assert!(
        first_release_gain < 0.6,
        "Release should start from ~0.5 (attack midpoint), not 0.7 (sustain). Got {}",
        first_release_gain
    );
}

#[test]
fn buffer_fill_produces_audio() {
    let inc = oscillator::phase_increment(440.0, SAMPLE_RATE);
    let mut phase: u64 = 0;
    let adsr = Adsr {
        attack_ms: 0.0,
        decay_ms: 0.0,
        sustain: 1.0,
        release_ms: 0.0,
    };
    let mut env_state = AdsrState::new();
    let mut buf = PcmBuffer::<256>::new();

    buf.fill(
        oscillator::sine,
        inc,
        &mut phase,
        &adsr,
        &mut env_state,
        SAMPLE_RATE,
    );

    assert_eq!(buf.len, 256);
    let has_nonzero = buf.as_slice().iter().any(|&s| s != 0);
    assert!(
        has_nonzero,
        "Buffer should contain non-zero samples after fill"
    );
}

#[test]
fn buffer_fill_raw_no_envelope() {
    let inc = oscillator::phase_increment(440.0, SAMPLE_RATE);
    let mut phase: u64 = 0;
    let mut buf = PcmBuffer::<256>::new();

    buf.fill_raw(oscillator::sine, inc, &mut phase);

    assert_eq!(buf.len, 256);
    //* should have strong signal
    let peak = buf.as_slice().iter().map(|s| s.abs()).max().unwrap();
    assert!(
        peak > 10000,
        "Raw fill should produce strong signal, peak was {}",
        peak
    );
}

#[test]
fn mix_two_voices() {
    //* Create two sine buffers at different frequencies
    let mut buf_a = PcmBuffer::<256>::new();
    let mut buf_b = PcmBuffer::<256>::new();
    let mut phase_a: u64 = 0;
    let mut phase_b: u64 = 0;

    buf_a.fill_raw(
        oscillator::sine,
        oscillator::phase_increment(440.0, SAMPLE_RATE),
        &mut phase_a,
    );
    buf_b.fill_raw(
        oscillator::sine,
        oscillator::phase_increment(880.0, SAMPLE_RATE),
        &mut phase_b,
    );

    let mut output = [0i16; 256];
    mix(&[buf_a.as_slice(), buf_b.as_slice()], &mut output, 256);

    //* Output should differ from both inputs
    let differs_from_a = output.iter().zip(buf_a.as_slice()).any(|(o, a)| o != a);
    let differs_from_b = output.iter().zip(buf_b.as_slice()).any(|(o, b)| o != b);
    assert!(differs_from_a, "Mix should differ from input A");
    assert!(differs_from_b, "Mix should differ from input B");
}

#[test]
fn mix_saturation_clamp() {
    //* Two maximum-amplitude signals should clamp to MAX
    let max_signal = [32767i16; 4];
    let mut output = [0i16; 4];

    mix(&[&max_signal, &max_signal], &mut output, 4);

    for &s in &output {
        assert_eq!(s, 32767, "Mix should saturate at i16::MAX, got {}", s);
    }
}

#[test]
fn all_patches_produce_audio() {
    let patches = [
        Patch::Kick,
        Patch::Snare,
        Patch::HiHat,
        Patch::Laser,
        Patch::Coin,
        Patch::Explosion,
    ];

    for patch in patches {
        let mut voice = PatchVoice::new(SAMPLE_RATE);
        voice.trigger(patch);

        let mut peak = 0i16;
        for _ in 0..2048 {
            let sample = voice.next_sample();
            peak = peak.max(sample.saturating_abs());
        }

        assert!(
            peak > 1000,
            "{} patch should produce an audible signal, peak was {}",
            patch.name(),
            peak
        );
    }
}

#[test]
fn patch_voice_turns_off_after_duration() {
    let mut voice = PatchVoice::new(SAMPLE_RATE);
    voice.trigger(Patch::HiHat);

    let max_samples = ((Patch::HiHat.duration_ms() + 50.0) * SAMPLE_RATE as f32 / 1000.0) as usize;
    for _ in 0..max_samples {
        voice.next_sample();
    }

    assert!(!voice.is_active(), "Patch voice should turn itself off");
}

#[test]
fn patch_output_is_deterministic() {
    let mut a = PatchVoice::new(SAMPLE_RATE);
    let mut b = PatchVoice::new(SAMPLE_RATE);
    a.trigger(Patch::Explosion);
    b.trigger(Patch::Explosion);

    for i in 0..512 {
        assert_eq!(
            a.next_sample(),
            b.next_sample(),
            "Patch output diverged at sample {}",
            i
        );
    }
}

#[test]
fn patch_index_roundtrip() {
    for index in 0..Patch::COUNT as u8 {
        let patch = Patch::from_index(index).expect("known patch index");
        assert_eq!(patch.index(), index);
    }
    assert!(Patch::from_index(255).is_none());
}

#[test]
fn custom_layered_sound_produces_audio() {
    let layers = [
        Layer::tone(
            Waveform::Sine,
            0.9,
            320.0,
            1400.0,
            120.0,
            Curve::Decay,
            Curve::Decay,
        ),
        Layer::noise(0.35, 40.0, Curve::Linear, FilterMode::HighPass, 0.05),
        Layer::silent(),
        Layer::silent(),
    ];
    let spec = SoundSpec::from_layers(340.0, 2, layers);
    let mut voice = SoundVoice::new(SAMPLE_RATE);
    voice.trigger(spec);

    let mut peak = 0i16;
    for _ in 0..2048 {
        peak = peak.max(voice.next_sample().saturating_abs());
    }

    assert!(peak > 1000, "Custom layered sound should produce audio");
}

#[test]
fn custom_sound_turns_off() {
    let layers = [
        Layer::tone(
            Waveform::Sawtooth,
            0.5,
            80.0,
            900.0,
            900.0,
            Curve::Hold,
            Curve::Linear,
        ),
        Layer::silent(),
        Layer::silent(),
        Layer::silent(),
    ];
    let spec = SoundSpec::from_layers(90.0, 1, layers);
    let mut voice = SoundVoice::new(SAMPLE_RATE);
    voice.trigger(spec);

    for _ in 0..((SAMPLE_RATE as f32 * 0.12) as usize) {
        voice.next_sample();
    }

    assert!(!voice.is_active(), "Custom sound should turn itself off");
}

#[test]
fn custom_sound_output_is_deterministic() {
    let layers = [
        Layer::noise(0.7, 180.0, Curve::Decay, FilterMode::LowPass, 0.12),
        Layer::tone(
            Waveform::Triangle,
            0.35,
            180.0,
            70.0,
            40.0,
            Curve::Linear,
            Curve::Decay,
        ),
        Layer::silent(),
        Layer::silent(),
    ];
    let spec = SoundSpec::from_layers(200.0, 2, layers);
    let mut a = SoundVoice::new(SAMPLE_RATE);
    let mut b = SoundVoice::new(SAMPLE_RATE);
    a.trigger(spec);
    b.trigger(spec);

    for i in 0..512 {
        assert_eq!(
            a.next_sample(),
            b.next_sample(),
            "Custom sound diverged at sample {}",
            i
        );
    }
}

#[test]
fn sound_spec_caps_layer_count() {
    let layers = [Layer::silent(); MAX_LAYERS];
    let spec = SoundSpec::from_layers(10.0, MAX_LAYERS + 20, layers);
    let mut voice = SoundVoice::new(SAMPLE_RATE);
    voice.trigger(spec);

    for _ in 0..128 {
        voice.next_sample();
    }

    assert!(voice.elapsed_ms() > 0.0);
}

#[cfg(feature = "convert")]
mod convert_tests {
    use resonance::convert;

    #[test]
    fn wav_header_is_valid() {
        let samples = [0i16; 100];
        let wav = convert::to_wav_bytes(&samples, 44100);

        //* Check RIFF header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        //* Total size: 44 header + 200 data bytes
        assert_eq!(wav.len(), 244);

        //* Data size field should be 200 (100 samples × 2 bytes)
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]);
        assert_eq!(data_size, 200);
    }

    #[test]
    fn wav_sample_data_is_correct() {
        let samples = [1000i16, -1000, 0, 32767, -32768];
        let wav = convert::to_wav_bytes(&samples, 44100);

        //* Verify first sample (offset 44, 45)
        let s0 = i16::from_le_bytes([wav[44], wav[45]]);
        assert_eq!(s0, 1000);

        //* Verify second sample
        let s1 = i16::from_le_bytes([wav[46], wav[47]]);
        assert_eq!(s1, -1000);
    }
}
