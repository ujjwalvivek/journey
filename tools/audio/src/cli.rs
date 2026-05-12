use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};
use resonance::patch::Patch;

const WAVE_SINE: u8 = 0;
const WAVE_SQUARE: u8 = 1;
const WAVE_SAW: u8 = 2;
const WAVE_TRI: u8 = 3;
const WAVE_NOISE: u8 = 4;
const PATCH_NONE: u8 = 255;

fn waveform_name(w: u8) -> &'static str {
    match w {
        WAVE_SINE => "Sine",
        WAVE_SQUARE => "Square",
        WAVE_SAW => "Sawtooth",
        WAVE_TRI => "Triangle",
        WAVE_NOISE => "Noise",
        _ => "Unknown",
    }
}

fn patch_for_key(ch: char) -> Option<Patch> {
    match ch.to_ascii_lowercase() {
        'z' => Some(Patch::Kick),
        'x' => Some(Patch::Snare),
        'c' => Some(Patch::HiHat),
        'v' => Some(Patch::Laser),
        'b' => Some(Patch::Coin),
        'n' => Some(Patch::Explosion),
        _ => None,
    }
}

struct SharedParams {
    frequency: AtomicU32,
    waveform: AtomicU8,
    attack_ms: AtomicU32,
    decay_ms: AtomicU32,
    sustain: AtomicU32,
    release_ms: AtomicU32,
    note_on: AtomicBool,
    patch_trigger: AtomicU8,
    retrigger: AtomicBool,
    release: AtomicBool,
    running: AtomicBool,
}

impl SharedParams {
    fn new() -> Self {
        Self {
            frequency: AtomicU32::new(440.0_f32.to_bits()),
            waveform: AtomicU8::new(WAVE_SINE),
            attack_ms: AtomicU32::new(10.0_f32.to_bits()),
            decay_ms: AtomicU32::new(50.0_f32.to_bits()),
            sustain: AtomicU32::new(0.7_f32.to_bits()),
            release_ms: AtomicU32::new(200.0_f32.to_bits()),
            note_on: AtomicBool::new(true),
            patch_trigger: AtomicU8::new(PATCH_NONE),
            retrigger: AtomicBool::new(true),
            release: AtomicBool::new(false),
            running: AtomicBool::new(true),
        }
    }
    fn get_freq(&self) -> f32 {
        f32::from_bits(self.frequency.load(Ordering::Relaxed))
    }
    fn set_freq(&self, v: f32) {
        self.frequency.store(v.to_bits(), Ordering::Relaxed);
    }
    fn get_attack(&self) -> f32 {
        f32::from_bits(self.attack_ms.load(Ordering::Relaxed))
    }
    fn set_attack(&self, v: f32) {
        self.attack_ms
            .store(v.clamp(0.0, 5000.0).to_bits(), Ordering::Relaxed);
    }
    fn get_decay(&self) -> f32 {
        f32::from_bits(self.decay_ms.load(Ordering::Relaxed))
    }
    fn set_decay(&self, v: f32) {
        self.decay_ms
            .store(v.clamp(0.0, 5000.0).to_bits(), Ordering::Relaxed);
    }
    fn get_sustain(&self) -> f32 {
        f32::from_bits(self.sustain.load(Ordering::Relaxed))
    }
    fn set_sustain(&self, v: f32) {
        self.sustain
            .store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    fn get_release(&self) -> f32 {
        f32::from_bits(self.release_ms.load(Ordering::Relaxed))
    }
    fn set_release(&self, v: f32) {
        self.release_ms
            .store(v.clamp(0.0, 5000.0).to_bits(), Ordering::Relaxed);
    }
    fn trigger_patch(&self, patch: Patch) {
        self.patch_trigger.store(patch.index(), Ordering::Relaxed);
    }
}

fn start_audio(params: Arc<SharedParams>) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No audio output device found");

    let supported_config = device
        .default_output_config()
        .expect("No default output config");
    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels() as usize;
    let config = supported_config.into();

    let mut phase: u64 = 0;
    let mut env_state = resonance::envelope::AdsrState::new();
    let mut noise = resonance::oscillator::Noise::new();
    let mut patch_voice = resonance::patch::PatchVoice::new(sample_rate);
    let mut current_freq = params.get_freq();
    let mut current_inc = resonance::oscillator::phase_increment(current_freq, sample_rate);
    let mut current_band = resonance::oscillator::octave_for_freq(current_freq);

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if !params.running.load(Ordering::Relaxed) {
                    for s in data.iter_mut() {
                        *s = 0.0;
                    }
                    return;
                }
                if params.retrigger.swap(false, Ordering::Relaxed) {
                    env_state = resonance::envelope::AdsrState::new();
                    phase = 0;
                }
                if params.release.swap(false, Ordering::Relaxed) {
                    let adsr = resonance::envelope::Adsr {
                        attack_ms: params.get_attack(),
                        decay_ms: params.get_decay(),
                        sustain: params.get_sustain(),
                        release_ms: params.get_release(),
                    };
                    env_state.note_off(&adsr);
                }
                let patch_index = params.patch_trigger.swap(PATCH_NONE, Ordering::Relaxed);
                if let Some(patch) = Patch::from_index(patch_index) {
                    patch_voice.trigger(patch);
                }
                let freq = params.get_freq();
                if (freq - current_freq).abs() > 0.01 {
                    current_freq = freq;
                    current_inc = resonance::oscillator::phase_increment(freq, sample_rate);
                    current_band = resonance::oscillator::octave_for_freq(freq);
                }
                let waveform = params.waveform.load(Ordering::Relaxed);
                let adsr = resonance::envelope::Adsr {
                    attack_ms: params.get_attack(),
                    decay_ms: params.get_decay(),
                    sustain: params.get_sustain(),
                    release_ms: params.get_release(),
                };
                let ms_per_sample = 1000.0 / sample_rate as f32;

                //* cpal gives us interleaved samples: [L, R, L, R, ...]
                let num_frames = data.len() / channels;
                for frame in 0..num_frames {
                    let gain = env_state.tick(&adsr, ms_per_sample);

                    let raw: i16 = match waveform {
                        WAVE_SINE => resonance::oscillator::sine(phase),
                        WAVE_SQUARE => resonance::oscillator::square(phase, current_band),
                        WAVE_SAW => resonance::oscillator::sawtooth(phase, current_band),
                        WAVE_TRI => resonance::oscillator::triangle(phase, current_band),
                        WAVE_NOISE => noise.next_sample(),
                        _ => 0,
                    };

                    let held_tone = (raw as f32 / 32768.0) * gain * 0.42;
                    let patch = (patch_voice.next_sample() as f32 / 32768.0) * 0.85;
                    let sample_f32 = (held_tone + patch).clamp(-1.0, 1.0);

                    for ch in 0..channels {
                        data[frame * channels + ch] = sample_f32;
                    }

                    if waveform != WAVE_NOISE {
                        phase = phase.wrapping_add(current_inc);
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )
        .expect("Failed to build output stream");

    stream.play().expect("Failed to start audio stream");
    stream
}

#[derive(Clone, Copy, PartialEq)]
enum EnvParam {
    Attack,
    Decay,
    Sustain,
    Release,
}

struct App {
    params: Arc<SharedParams>,
    selected_env: EnvParam,
    freq_step: f32,
}

impl App {
    fn new(params: Arc<SharedParams>) -> Self {
        Self {
            params,
            selected_env: EnvParam::Attack,
            freq_step: 10.0,
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        let freq_mult = if shift { 10.0 } else { 1.0 };
        let env_mult = if shift { 10.0 } else { 1.0 };

        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Up => {
                let f = self.params.get_freq();
                self.params
                    .set_freq((f + self.freq_step * freq_mult).min(20000.0));
            }
            KeyCode::Down => {
                let f = self.params.get_freq();
                self.params
                    .set_freq((f - self.freq_step * freq_mult).max(20.0));
            }
            KeyCode::Char('1') => self.params.waveform.store(WAVE_SINE, Ordering::Relaxed),
            KeyCode::Char('2') => self.params.waveform.store(WAVE_SQUARE, Ordering::Relaxed),
            KeyCode::Char('3') => self.params.waveform.store(WAVE_SAW, Ordering::Relaxed),
            KeyCode::Char('4') => self.params.waveform.store(WAVE_TRI, Ordering::Relaxed),
            KeyCode::Char('5') => self.params.waveform.store(WAVE_NOISE, Ordering::Relaxed),
            KeyCode::Char('a') => self.selected_env = EnvParam::Attack,
            KeyCode::Char('d') => self.selected_env = EnvParam::Decay,
            KeyCode::Char('s') => self.selected_env = EnvParam::Sustain,
            KeyCode::Char('r') => self.selected_env = EnvParam::Release,
            KeyCode::Right => {
                let step = env_mult;
                match self.selected_env {
                    EnvParam::Attack => {
                        let v = self.params.get_attack();
                        self.params.set_attack(v + step * 5.0);
                    }
                    EnvParam::Decay => {
                        let v = self.params.get_decay();
                        self.params.set_decay(v + step * 5.0);
                    }
                    EnvParam::Sustain => {
                        let v = self.params.get_sustain();
                        self.params.set_sustain(v + 0.05 * env_mult);
                    }
                    EnvParam::Release => {
                        let v = self.params.get_release();
                        self.params.set_release(v + step * 5.0);
                    }
                }
            }
            KeyCode::Left => {
                let step = env_mult;
                match self.selected_env {
                    EnvParam::Attack => {
                        let v = self.params.get_attack();
                        self.params.set_attack(v - step * 5.0);
                    }
                    EnvParam::Decay => {
                        let v = self.params.get_decay();
                        self.params.set_decay(v - step * 5.0);
                    }
                    EnvParam::Sustain => {
                        let v = self.params.get_sustain();
                        self.params.set_sustain(v - 0.05 * env_mult);
                    }
                    EnvParam::Release => {
                        let v = self.params.get_release();
                        self.params.set_release(v - step * 5.0);
                    }
                }
            }
            KeyCode::Char(' ') => {
                let was_on = self.params.note_on.load(Ordering::Relaxed);
                if was_on {
                    self.params.note_on.store(false, Ordering::Relaxed);
                    self.params.release.store(true, Ordering::Relaxed);
                } else {
                    self.params.note_on.store(true, Ordering::Relaxed);
                    self.params.retrigger.store(true, Ordering::Relaxed);
                }
            }
            KeyCode::Char(ch) if patch_for_key(ch).is_some() => {
                self.params.trigger_patch(patch_for_key(ch).unwrap());
            }
            _ => {}
        }
        true
    }
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(3),
        ])
        .split(area);

    let note_status = if app.params.note_on.load(Ordering::Relaxed) {
        Span::styled(
            "PLAYING",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("SILENT", Style::default().fg(Color::DarkGray))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " RESONANCE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("DSP CLI Synthesizer", Style::default().fg(Color::Gray)),
        Span::raw("    "),
        note_status,
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, chunks[0]);

    let freq_wave_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let freq = app.params.get_freq();
    let freq_note = freq_to_note(freq);
    let freq_text = vec![
        Line::from(vec![
            Span::styled("Frequency: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1} Hz", freq),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Note: ", Style::default().fg(Color::Gray)),
            Span::styled(freq_note, Style::default().fg(Color::Cyan)),
        ]),
    ];
    let freq_widget = Paragraph::new(freq_text).block(
        Block::default()
            .title(" Frequency ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(freq_widget, freq_wave_chunks[0]);

    let waveform = app.params.waveform.load(Ordering::Relaxed);
    let wave_lines: Vec<Line> = (0..5)
        .map(|i| {
            let name = waveform_name(i);
            let key = format!("[{}] ", i + 1);
            if i == waveform {
                Line::from(vec![
                    Span::styled(key, Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("> {}", name),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(key, Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("  {}", name), Style::default().fg(Color::DarkGray)),
                ])
            }
        })
        .collect();
    let wave_widget = Paragraph::new(wave_lines).block(
        Block::default()
            .title(" Waveform ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(wave_widget, freq_wave_chunks[1]);

    render_adsr(frame, chunks[2], app);

    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Up/Down",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Freq  "),
            Span::styled(
                "1-5",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Wave  "),
            Span::styled(
                "A/D/S/R",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Select Env  "),
            Span::styled(
                "Left/Right",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Adjust  "),
            Span::styled(
                "Space",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Note  "),
            Span::styled("Shift", Style::default().fg(Color::DarkGray)),
            Span::raw(" x10  "),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ]),
        Line::from(vec![
            Span::styled(
                "Z/X/C/V/B/N",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Kick / Snare / HiHat / Laser / Coin / Explosion"),
        ]),
    ])
    .block(
        Block::default()
            .title(" Controls ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(help, chunks[3]);
}

fn render_adsr(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" ADSR Envelope ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let params = [
        (
            "A  Attack ",
            app.params.get_attack(),
            5000.0,
            "ms",
            EnvParam::Attack,
        ),
        (
            "D  Decay  ",
            app.params.get_decay(),
            5000.0,
            "ms",
            EnvParam::Decay,
        ),
        (
            "S  Sustain",
            app.params.get_sustain(),
            1.0,
            "",
            EnvParam::Sustain,
        ),
        (
            "R  Release",
            app.params.get_release(),
            5000.0,
            "ms",
            EnvParam::Release,
        ),
    ];

    for (i, (label, value, max, unit, param)) in params.iter().enumerate() {
        let is_selected = app.selected_env == *param;
        let ratio = (*value / *max).clamp(0.0, 1.0);

        let label_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let bar_color = if is_selected {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let value_str = if unit.is_empty() {
            format!("{:.2}", value)
        } else {
            format!("{:.0} {}", value, unit)
        };

        let gauge = Gauge::default()
            .label(Span::styled(
                format!("{} | {}", label, value_str),
                label_style,
            ))
            .gauge_style(Style::default().fg(bar_color))
            .ratio(ratio as f64);
        frame.render_widget(gauge, rows[i]);
    }
}

fn freq_to_note(freq: f32) -> String {
    if freq <= 0.0 {
        return "-".to_string();
    }
    let notes = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let midi_rounded = midi.round() as i32;
    let note_idx = ((midi_rounded % 12) + 12) % 12;
    let octave = (midi_rounded / 12) - 1;
    let cents = ((midi - midi_rounded as f32) * 100.0).round() as i32;
    let cents_str = if cents == 0 {
        String::new()
    } else if cents > 0 {
        format!(" +{} cents", cents)
    } else {
        format!(" {} cents", cents)
    };
    format!("{}{}{}", notes[note_idx as usize], octave, cents_str)
}

pub fn run() -> io::Result<()> {
    let params = Arc::new(SharedParams::new());
    let _stream = start_audio(Arc::clone(&params));
    let mut terminal = ratatui::init();
    let mut app = App::new(Arc::clone(&params));
    let result = run_tui(&mut terminal, &mut app);
    params.running.store(false, Ordering::Relaxed);
    ratatui::restore();
    result
}

fn run_tui(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;
        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && !app.handle_key(key.code, key.modifiers)
        {
            break;
        }
    }
    Ok(())
}
