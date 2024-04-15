use anyhow::{Context, Result};
use midly::{live::LiveEvent, MidiMessage};
use rusty_link::{AblLink, SessionState};
use tracing::{info, trace};

pub struct LinkAdapter {
    link: AblLink,
    state: SessionState,
}

impl LinkAdapter {
    /// Creates a new adapter that automatically connects to Ableton Link.
    pub fn new() -> Self {
        let link = AblLink::new(120.0);
        let state = SessionState::new();

        info!("Enabling Link");
        link.enable(true);

        link.set_num_peers_callback(move |peers| {
            info!(peers, "Link updated");
        });

        Self { link, state }
    }

    /// Handles an incoming MIDI event in raw form.
    pub fn handle_raw_event(&mut self, stamp: u64, raw: &[u8]) -> Result<()> {
        let event = LiveEvent::parse(raw).context("Could not parse MIDI event")?;
        self.handle_event(stamp, event)
    }

    /// Handles an incoming MIDI event.
    pub fn handle_event(&mut self, stamp: u64, event: LiveEvent) -> Result<()> {
        match event {
            LiveEvent::Midi { channel: _, message: MidiMessage::NoteOn { key, vel } } => {
                // See https://github.com/mixxxdj/mixxx/wiki/MIDI%20clock%20output
                match key.as_int() {
                    // Beat
                    50 if vel == 100 => {
                        info!(stamp, "Setting beat");
                        let beat = 0.0;
                        let quantum = 1.0;
                        self.update_state(|state| state.force_beat_at_time(beat, stamp, quantum));
                    },
                    // BPM
                    52 => {
                        let bpm = vel.as_int() + 50;
                        info!(bpm, "Setting BPM");
                        self.update_state(|state| {
                            if state.tempo() != bpm as f64 {
                                state.set_tempo(bpm as f64, 0);
                            }
                        });
                    },
                    _ => trace!(?event, "Ignoring MIDI note event"),
                }
            },
            _ => trace!(?event, "Ignoring MIDI event"),
        }

        Ok(())
    }

    fn update_state(&mut self, action: impl FnOnce(&mut SessionState)) {
        self.link.capture_audio_session_state(&mut self.state);
        action(&mut self.state);
        self.link.commit_audio_session_state(&mut self.state);
    }
}

impl Drop for LinkAdapter {
    fn drop(&mut self) {
        info!("Disabling Link");
        self.link.enable(false);
    }
}
