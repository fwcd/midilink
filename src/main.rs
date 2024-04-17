#[cfg(not(unix))]
compile_error!("MIDILink currently requires a Unix-like platform (e.g. macOS or Linux) since virtual MIDI ports are not supported on Windows");

mod adapter;

use std::{sync::{atomic::{AtomicBool, Ordering}, Arc}, thread};

use anyhow::{anyhow, Result};
use clap::Parser;
use midir::{os::unix::{VirtualInput, VirtualOutput}, MidiInput, MidiOutput};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;

use crate::adapter::LinkAdapter;

#[derive(Parser)]
#[command(version)]
struct Args {
    /// The name of the virtual MIDI input and output ports.
    #[arg(short, long, default_value = "Link")]
    name: String,
}

fn main() -> Result<()> {
    _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env()?)
        .init();

    let args = Args::parse();
    let mut adapter = LinkAdapter::new();

    info!(name = %args.name, "Creating virtual MIDI ports");
    let midi_in = MidiInput::new("MIDILink input")?;
    let _conn_in = midi_in.create_virtual(
        &args.name,
        move |stamp, raw, _| {
            let result = adapter.handle_raw_event(stamp, raw);
            if let Err(e) = result {
                warn!(%e, "Error while handling event");
            }
        },
        ()
    ).map_err(|e| anyhow!("Could not create virtual input: {}", e))?;

    let midi_out = MidiOutput::new("MIDILink output")?;
    let _conn_out = midi_out.create_virtual(&args.name)
        .map_err(|e| anyhow!("Could not create virtual input: {}", e))?;

    info!("Waiting for input...");

    let running = Arc::new(AtomicBool::new(true));

    {
        let running = running.clone();
        let main_thread = thread::current();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
            main_thread.unpark();
        })?;
    }

    while running.load(Ordering::SeqCst) {
        thread::park();
    }

    info!("Exiting...");
    Ok(())
}
