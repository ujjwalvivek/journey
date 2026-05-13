use cadence::{EuclideanPattern, Lfsr, MarkovChain, Transport};
use cadence::{Sequencer, Track, TrackEvent};

#[test]
fn transport_triggers_sixteenth_notes_at_expected_samples() {
    let mut transport = Transport::new(44_100, 120.0);
    let mut triggers = [0u64; 5];
    let mut found = 0;

    while found < triggers.len() {
        let sample = transport.current_sample();
        if transport.tick() {
            triggers[found] = sample;
            found += 1;
        }
    }

    assert_eq!(triggers, [0, 5513, 11025, 16538, 22050]);
}

#[test]
fn transport_does_not_accumulate_step_drift() {
    let mut transport = Transport::new(48_000, 123.0);
    let samples_per_step = transport.samples_per_step();
    let mut last_trigger = 0;
    let mut trigger_count = 0;

    while trigger_count <= 4096 {
        let sample = transport.current_sample();
        if transport.tick() {
            last_trigger = sample;
            trigger_count += 1;
        }
    }

    let expected = (4096.0 * samples_per_step).round() as u64;
    assert_eq!(last_trigger, expected);
}

#[test]
fn euclidean_pattern_distributes_pulses() {
    let pattern = EuclideanPattern::<16>::new(4, 16, 0);
    let active: [bool; 16] = core::array::from_fn(|index| pattern.is_active(index));

    assert_eq!(
        active,
        [
            true, false, false, false, true, false, false, false, true, false, false, false, true,
            false, false, false,
        ]
    );
    assert_eq!(pattern.active_count(), 4);
}

#[test]
fn euclidean_pattern_wraps_and_rotates() {
    let pattern = EuclideanPattern::<8>::new(3, 8, 1);

    assert_eq!(pattern.active_count(), 3);
    assert_eq!(pattern.is_active(7), pattern.is_active(15));
    assert_eq!(pattern.is_active(0), pattern.raw_step(1));
}

#[test]
fn lfsr_is_deterministic() {
    let mut a = Lfsr::new(0x1234);
    let mut b = Lfsr::new(0x1234);

    for _ in 0..512 {
        assert_eq!(a.next_u16(), b.next_u16());
    }
}

#[test]
fn markov_chain_uses_weighted_rows() {
    let matrix = [[0, 10, 0], [5, 0, 5], [0, 10, 0]];
    let mut chain = MarkovChain::<3>::new(matrix, 0);

    assert_eq!(chain.next(0), 1);

    let next = chain.next(7);
    assert!(next == 0 || next == 2);
    assert_eq!(chain.current_state(), next);
}

#[test]
fn markov_chain_stays_put_on_empty_row() {
    let matrix = [[0, 0], [10, 0]];
    let mut chain = MarkovChain::<2>::new(matrix, 0);

    assert_eq!(chain.next(42), 0);
    assert_eq!(chain.current_state(), 0);
}

#[test]
fn sequencer_fires_kick_on_four_on_the_floor() {
    let mut seq = Sequencer::<16>::new(44_100, 120.0, 16, 0xBEEF);

    seq.add_track(Track::percussion(EuclideanPattern::new(4, 16, 0), 0, 1.0));

    let mut kick_events = 0u32;
    let mut steps_seen = 0u32;
    let max_samples = 44_100 * 2;

    for _ in 0..max_samples {
        let output = seq.tick();
        if output.step_triggered {
            steps_seen += 1;
            if let Some(TrackEvent::TriggerPatch { patch_index: 0, .. }) = output.events[0] {
                kick_events += 1;
            }
            if steps_seen >= 16 {
                break;
            }
        }
    }

    assert_eq!(
        kick_events, 4,
        "E(4,16) should fire exactly 4 kicks per cycle"
    );
    assert_eq!(steps_seen, 16, "Should have seen all 16 steps");
}

#[test]
fn sequencer_melodic_track_stays_in_scale() {
    let scale: [f32; 8] = [220.0, 261.63, 293.66, 329.63, 392.00, 440.0, 523.25, 587.33];

    let matrix = [
        [0, 40, 10, 5, 0, 0, 0, 5],
        [20, 0, 35, 10, 5, 0, 0, 0],
        [5, 20, 0, 35, 10, 0, 0, 0],
        [5, 5, 20, 0, 30, 10, 0, 0],
        [0, 5, 5, 20, 0, 35, 10, 0],
        [0, 0, 5, 5, 20, 0, 35, 10],
        [5, 0, 0, 5, 5, 25, 0, 30],
        [10, 5, 0, 0, 5, 10, 30, 0],
    ];

    let mut seq = Sequencer::<16>::new(44_100, 120.0, 16, 0x1234);

    seq.add_track(Track::melody(
        EuclideanPattern::new(5, 16, 0),
        MarkovChain::new(matrix, 0),
        scale,
        8,
        0.8,
    ));

    let mut note_events = 0u32;
    let max_samples = 44_100 * 30;

    for _ in 0..max_samples {
        let output = seq.tick();
        if let (true, Some(TrackEvent::TriggerNote { frequency, .. })) =
            (output.step_triggered, output.events[0])
        {
            assert!(
                scale.contains(&frequency),
                "Frequency {} is not in the scale",
                frequency
            );
            note_events += 1;
        }
    }

    assert!(
        note_events > 50,
        "Should have fired many melodic events, got {}",
        note_events
    );
}

#[test]
fn sequencer_is_deterministic() {
    let mut seq_a = Sequencer::<16>::new(44_100, 120.0, 16, 0xCAFE);
    let mut seq_b = Sequencer::<16>::new(44_100, 120.0, 16, 0xCAFE);

    let scale: [f32; 8] = [220.0, 261.63, 293.66, 329.63, 392.00, 440.0, 523.25, 587.33];
    let matrix = [[10; 8]; 8];

    let track = Track::melody(
        EuclideanPattern::new(5, 16, 0),
        MarkovChain::new(matrix, 0),
        scale,
        8,
        0.7,
    );

    seq_a.add_track(track);
    seq_b.add_track(track);

    for _ in 0..44_100 {
        let out_a = seq_a.tick();
        let out_b = seq_b.tick();
        assert_eq!(out_a.step_triggered, out_b.step_triggered);
        assert_eq!(out_a.events[0], out_b.events[0]);
    }
}

#[test]
fn sequencer_mute_suppresses_events() {
    let mut seq = Sequencer::<16>::new(44_100, 120.0, 16, 0xDEAD);

    seq.add_track(Track::percussion(EuclideanPattern::new(4, 16, 0), 0, 1.0));

    seq.mute(0);
    assert!(!seq.is_track_active(0));

    let mut events_while_muted = 0u32;
    for _ in 0..44_100 {
        let output = seq.tick();
        if output.step_triggered && output.events[0].is_some() {
            events_while_muted += 1;
        }
    }

    assert_eq!(
        events_while_muted, 0,
        "Muted track should produce no events"
    );

    seq.unmute(0);
    assert!(seq.is_track_active(0));

    let mut events_after_unmute = 0u32;
    let mut steps_seen = 0u32;
    for _ in 0..44_100 {
        let output = seq.tick();
        if output.step_triggered {
            steps_seen += 1;
            if output.events[0].is_some() {
                events_after_unmute += 1;
            }
            if steps_seen >= 16 {
                break;
            }
        }
    }

    assert!(
        events_after_unmute > 0,
        "Unmuted track should produce events"
    );
}

#[test]
fn sequencer_bpm_change_maintains_continuity() {
    let mut seq = Sequencer::<16>::new(44_100, 120.0, 16, 0xFACE);

    seq.add_track(Track::percussion(EuclideanPattern::new(4, 16, 0), 0, 1.0));

    let mut steps_before = 0u32;
    for _ in 0..44_100 {
        let output = seq.tick();
        if output.step_triggered {
            steps_before += 1;
            if steps_before >= 8 {
                break;
            }
        }
    }
    assert_eq!(steps_before, 8);

    seq.set_bpm(180.0);

    let mut steps_after = 0u32;
    for _ in 0..44_100 {
        let output = seq.tick();
        if output.step_triggered {
            steps_after += 1;
            if steps_after >= 8 {
                break;
            }
        }
    }
    assert_eq!(steps_after, 8, "Should get 8 steps after BPM change");
}

#[test]
fn scene_drums_builds_and_runs() {
    let mut seq = cadence::scene::drums::<16>(44_100, 120.0, 0xBEAD);

    assert_eq!(seq.track_count(), 4);

    let mut total_events = 0u32;
    for _ in 0..44_100 {
        let output = seq.tick();
        if output.step_triggered {
            for event in &output.events {
                if event.is_some() {
                    total_events += 1;
                }
            }
        }
    }

    assert!(total_events > 0, "Drums scene should produce events");
}

#[test]
fn scene_full_builds_and_runs() {
    let mut seq = cadence::scene::full::<16>(44_100, 120.0, 0xF00D);

    assert_eq!(seq.track_count(), 5);

    let mut patch_events = 0u32;
    let mut note_events = 0u32;
    for _ in 0..44_100 * 2 {
        let output = seq.tick();
        if output.step_triggered {
            for event in &output.events {
                match event {
                    Some(TrackEvent::TriggerPatch { .. }) => patch_events += 1,
                    Some(TrackEvent::TriggerNote { .. }) => note_events += 1,
                    None => {}
                }
            }
        }
    }

    assert!(patch_events > 0, "Should have percussion events");
    assert!(note_events > 0, "Should have melodic events");
}
