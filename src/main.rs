#[cfg(not(unix))]
compile_error!("MIDILink currently requires a Unix-like platform (e.g. macOS or Linux) since virtual MIDI ports are not supported on Windows");

use std::thread;

use anyhow::{anyhow, Result};
use clap::Parser;
use midir::{os::unix::{VirtualInput, VirtualOutput}, MidiInput, MidiOutput};
use midly::live::LiveEvent;
use tracing::{info, warn};

#[derive(Parser)]
#[command(version)]
struct Args {
    /// The name of the virtual MIDI input and output ports.
    #[arg(short, long, default_value = "Link")]
    name: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    _ = dotenvy::dotenv();

    let args = Args::parse();

    info!(name = %args.name, "Creating virtual MIDI ports");

    let midi_in = MidiInput::new("MIDILink input")?;
    let _conn_in = midi_in.create_virtual(
        &args.name,
        move |_stamp, raw, _| {
            // TODO: Factor this whole callback into a Result-returning function
            match LiveEvent::parse(raw) {
                Ok(event) => info!(?event, "Received"),
                Err(err) => warn!(?err, "Could not parse MIDI event"),
            }
        },
        ()
    ).map_err(|e| anyhow!("Could not create virtual input: {}", e))?;

    let midi_out = MidiOutput::new("MIDILink output")?;
    let _conn_out = midi_out.create_virtual(&args.name)
        .map_err(|e| anyhow!("Could not create virtual input: {}", e))?;

    info!("Waiting for input...");

    loop {
        thread::park();
    }
}
