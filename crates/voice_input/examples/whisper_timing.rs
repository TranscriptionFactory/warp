//! Times local Whisper transcription with Candle.
//!
//! cargo run --release -p voice_input --features whisper --example whisper_timing -- \
//!     <model_dir> <clip.wav>
//!
//! `model_dir` holds `config.json`, `tokenizer.json` and `model.safetensors`.
//! The clip must be 16 kHz mono 16-bit PCM, which is what `voice_input` records.

use std::path::PathBuf;

use anyhow::{Context, Result};
use instant::Instant;
use voice_input::whisper::WhisperModel;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: whisper_timing <model_dir> <clip.wav>";
    let model_dir = PathBuf::from(args.next().context(usage)?);
    let wav = std::fs::read(args.next().context(usage)?)?;

    let start = Instant::now();
    let mut model = WhisperModel::load(&model_dir)?;
    let load = start.elapsed();

    let start = Instant::now();
    let text = model.transcribe_wav(&wav)?;
    let transcribe = start.elapsed();

    println!("text: {text}");
    println!("load {load:.2?} | transcribe {transcribe:.2?}");
    Ok(())
}
