//! Local speech-to-text using Whisper on Candle.

use std::{io::Cursor, path::Path};

use anyhow::{Context, Result, anyhow, ensure};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::{VarBuilder, ops::softmax};
use candle_transformers::models::whisper::{self as m, Config, audio, model::Whisper};
use tokenizers::Tokenizer;

/// 80-bin mel filter bank used by the `base.en` and `small.en` models.
const MEL_FILTERS: &[u8] = include_bytes!("../assets/melfilters.bytes");
const MAX_TOKENS_PER_SEGMENT: usize = 224;

/// A loaded Whisper model. `model_dir` must hold `config.json`, `tokenizer.json` and
/// `model.safetensors`.
pub struct WhisperModel {
    model: Whisper,
    config: Config,
    tokenizer: Tokenizer,
    device: Device,
    mel_filters: Vec<f32>,
    suppress: Tensor,
    sot: u32,
    no_timestamps: u32,
    eot: u32,
    no_speech: u32,
}

fn token_id(tokenizer: &Tokenizer, token: &str) -> Result<u32> {
    tokenizer
        .token_to_id(token)
        .ok_or_else(|| anyhow!("tokenizer has no {token}"))
}

fn device() -> Device {
    if cfg!(feature = "whisper_metal") {
        match Device::new_metal(0) {
            Ok(device) => return device,
            Err(e) => log::warn!("Metal is unavailable, using the CPU for Whisper: {e}"),
        }
    }
    Device::Cpu
}

impl WhisperModel {
    pub fn load(model_dir: &Path) -> Result<Self> {
        let device = device();
        let config: Config = serde_json::from_slice(&std::fs::read(model_dir.join("config.json"))?)
            .context("invalid config.json")?;
        let tokenizer =
            Tokenizer::from_file(model_dir.join("tokenizer.json")).map_err(|e| anyhow!("{e}"))?;
        let weights = model_dir.join("model.safetensors");
        // SAFETY: the weights file is only replaced by an atomic rename, never written in place.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], m::DTYPE, &device)? };
        let model = Whisper::load(&vb, config.clone())?;

        let mel_filters = MEL_FILTERS
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let suppress: Vec<f32> = (0..config.vocab_size as u32)
            .map(|i| {
                if config.suppress_tokens.contains(&i) {
                    f32::NEG_INFINITY
                } else {
                    0.0
                }
            })
            .collect();
        let suppress = Tensor::new(suppress.as_slice(), &device)?;

        let mut whisper = Self {
            sot: token_id(&tokenizer, m::SOT_TOKEN)?,
            no_timestamps: token_id(&tokenizer, m::NO_TIMESTAMPS_TOKEN)?,
            eot: token_id(&tokenizer, m::EOT_TOKEN)?,
            no_speech: m::NO_SPEECH_TOKENS
                .iter()
                .find_map(|token| tokenizer.token_to_id(token))
                .context("tokenizer has no no-speech token")?,
            model,
            config,
            tokenizer,
            device,
            mel_filters,
            suppress,
        };
        if whisper.device.is_metal() {
            // The first Metal run compiles shaders, which takes several seconds.
            whisper.transcribe_pcm(&[0.0; m::SAMPLE_RATE])?;
        }
        Ok(whisper)
    }

    /// Transcribes a 16 kHz mono 16-bit WAV file, which is what `VoiceInput` records.
    pub fn transcribe_wav(&mut self, wav: &[u8]) -> Result<String> {
        let reader = hound::WavReader::new(Cursor::new(wav))?;
        let spec = reader.spec();
        ensure!(
            spec.sample_rate == m::SAMPLE_RATE as u32
                && spec.channels == 1
                && spec.bits_per_sample == 16
                && spec.sample_format == hound::SampleFormat::Int,
            "expected 16 kHz mono 16-bit audio, got {spec:?}"
        );
        let pcm = reader
            .into_samples::<i16>()
            .map(|sample| sample.map(|sample| sample as f32 / 32768.0))
            .collect::<Result<Vec<_>, _>>()?;
        self.transcribe_pcm(&pcm)
    }

    fn transcribe_pcm(&mut self, pcm: &[f32]) -> Result<String> {
        if pcm.is_empty() {
            return Ok(String::new());
        }
        let mel = audio::pcm_to_mel(&self.config, pcm, &self.mel_filters);
        let bins = self.config.num_mel_bins;
        let frames = mel.len() / bins;
        let mel = Tensor::from_vec(mel, (1, bins, frames), &self.device)?;

        let mut text = String::new();
        let mut seek = 0;
        while seek < frames {
            let size = usize::min(frames - seek, m::N_FRAMES);
            text.push_str(&self.decode_segment(&mel.narrow(2, seek, size)?)?);
            seek += size;
        }
        Ok(text.trim().to_string())
    }

    fn decode_segment(&mut self, segment: &Tensor) -> Result<String> {
        let features = self.model.encoder.forward(segment, true)?;
        let mut tokens = vec![self.sot, self.no_timestamps];
        let prompt_len = tokens.len();
        for i in 0..MAX_TOKENS_PER_SEGMENT {
            let input = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
            let hidden = self.model.decoder.forward(&input, &features, i == 0)?;
            if i == 0 {
                // Whisper invents words for silence, so trust its own no-speech estimate.
                let logits = self
                    .model
                    .decoder
                    .final_linear(&hidden.i((..1, ..1))?)?
                    .i(0)?
                    .i(0)?;
                let no_speech_prob = softmax(&logits, 0)?
                    .i(self.no_speech as usize)?
                    .to_scalar::<f32>()?;
                if f64::from(no_speech_prob) > m::NO_SPEECH_THRESHOLD {
                    return Ok(String::new());
                }
            }
            let (_, seq_len, _) = hidden.dims3()?;
            let logits = self
                .model
                .decoder
                .final_linear(&hidden.i((..1, seq_len - 1..))?)?
                .i(0)?
                .i(0)?;
            let next = logits
                .broadcast_add(&self.suppress)?
                .argmax(0)?
                .to_scalar::<u32>()?;
            if next == self.eot {
                break;
            }
            tokens.push(next);
        }
        self.tokenizer
            .decode(&tokens[prompt_len..], true)
            .map_err(|e| anyhow!("{e}"))
    }
}
