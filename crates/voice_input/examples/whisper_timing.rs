//! Times local Whisper transcription with Candle.
//!
//! cargo run --release -p voice_input --features whisper --example whisper_timing -- \
//!     <model_dir> <clip.wav> <melfilters.bytes>
//!
//! `model_dir` holds `config.json`, `tokenizer.json` and `model.safetensors`.
//! The clip must be 16 kHz mono 16-bit PCM, which is what `voice_input` records.

use std::{path::PathBuf, time::Instant};

use anyhow::{Context, Result, anyhow};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config, audio, model::Whisper};
use tokenizers::Tokenizer;

const MAX_TOKENS_PER_SEGMENT: usize = 224;

fn token_id(tokenizer: &Tokenizer, token: &str) -> Result<u32> {
    tokenizer
        .token_to_id(token)
        .ok_or_else(|| anyhow!("tokenizer has no {token}"))
}

fn device() -> Result<Device> {
    if cfg!(feature = "whisper_metal") {
        Ok(Device::new_metal(0)?)
    } else {
        Ok(Device::Cpu)
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: whisper_timing <model_dir> <clip.wav> <melfilters.bytes>";
    let model_dir = PathBuf::from(args.next().context(usage)?);
    let clip = args.next().context(usage)?;
    let mel_path = args.next().context(usage)?;

    let device = device()?;
    let start = Instant::now();
    let config: Config =
        serde_json::from_slice(&std::fs::read(model_dir.join("config.json"))?)?;
    let tokenizer =
        Tokenizer::from_file(model_dir.join("tokenizer.json")).map_err(|e| anyhow!("{e}"))?;
    let weights = model_dir.join("model.safetensors");
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], m::DTYPE, &device)? };
    let mut model = Whisper::load(&vb, config.clone())?;
    let load = start.elapsed();

    let mel_bytes = std::fs::read(mel_path)?;
    let mel_filters: Vec<f32> = mel_bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();

    let reader = hound::WavReader::open(clip)?;
    let spec = reader.spec();
    anyhow::ensure!(
        spec.sample_rate == m::SAMPLE_RATE as u32 && spec.channels == 1,
        "expected 16 kHz mono audio, got {spec:?}"
    );
    let pcm: Vec<f32> = reader
        .into_samples::<i16>()
        .map(|s| s.map(|s| s as f32 / 32768.0))
        .collect::<Result<_, _>>()?;
    let clip_seconds = pcm.len() as f32 / m::SAMPLE_RATE as f32;

    let start = Instant::now();
    let mel = audio::pcm_to_mel(&config, &pcm, &mel_filters);
    let bins = config.num_mel_bins;
    let frames = mel.len() / bins;
    let mel = Tensor::from_vec(mel, (1, bins, frames), &device)?;
    let mel_time = start.elapsed();

    let sot = token_id(&tokenizer, m::SOT_TOKEN)?;
    let no_timestamps = token_id(&tokenizer, m::NO_TIMESTAMPS_TOKEN)?;
    let eot = token_id(&tokenizer, m::EOT_TOKEN)?;

    let vocab = config.vocab_size;
    let suppress: Vec<f32> = (0..vocab as u32)
        .map(|i| {
            if config.suppress_tokens.contains(&i) {
                f32::NEG_INFINITY
            } else {
                0.0
            }
        })
        .collect();
    let suppress = Tensor::new(suppress.as_slice(), &device)?;

    let mut encode_time = std::time::Duration::ZERO;
    let mut decode_time = std::time::Duration::ZERO;
    let mut text = String::new();
    let mut decoded = 0usize;
    let mut seek = 0;
    while seek < frames {
        let size = usize::min(frames - seek, m::N_FRAMES);
        let segment = mel.narrow(2, seek, size)?;
        seek += size;

        let start = Instant::now();
        let features = model.encoder.forward(&segment, true)?;
        encode_time += start.elapsed();

        let start = Instant::now();
        let mut tokens = vec![sot, no_timestamps];
        let prompt_len = tokens.len();
        for i in 0..MAX_TOKENS_PER_SEGMENT {
            let input = Tensor::new(tokens.as_slice(), &device)?.unsqueeze(0)?;
            let hidden = model.decoder.forward(&input, &features, i == 0)?;
            let (_, seq_len, _) = hidden.dims3()?;
            let logits = model
                .decoder
                .final_linear(&hidden.i((..1, seq_len - 1..))?)?
                .i(0)?
                .i(0)?;
            let next = logits.broadcast_add(&suppress)?.argmax(0)?.to_scalar::<u32>()?;
            if next == eot {
                break;
            }
            tokens.push(next);
        }
        decoded += tokens.len() - prompt_len;
        text.push_str(
            &tokenizer
                .decode(&tokens[prompt_len..], true)
                .map_err(|e| anyhow!("{e}"))?,
        );
        decode_time += start.elapsed();
    }

    let total = mel_time + encode_time + decode_time;
    println!("text: {}", text.trim());
    println!(
        "clip {clip_seconds:.1}s | load {load:.2?} | mel {mel_time:.2?} | encode {encode_time:.2?} | \
         decode {decode_time:.2?} ({decoded} tokens) | transcribe {total:.2?} | {:.2}x realtime",
        clip_seconds / total.as_secs_f32()
    );
    Ok(())
}
