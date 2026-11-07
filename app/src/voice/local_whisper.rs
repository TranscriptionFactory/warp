//! Transcribes voice input on this device with Whisper. Audio never leaves the machine; the only
//! network use is a one-time download of the model.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result, anyhow, ensure};
use async_trait::async_trait;
use base64::Engine as _;
use futures::{StreamExt as _, channel::oneshot, lock::Mutex};
use futures_lite::io::AsyncWriteExt as _;
use sha2::{Digest as _, Sha256};
use voice_input::whisper::WhisperModel;

use super::transcriber::{TranscribeError, Transcriber};

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60 * 60);

struct ModelFile {
    name: &'static str,
    sha256: &'static str,
    size: u64,
}

struct ModelSpec {
    repo: &'static str,
    /// Pinned so that the hashes below stay valid.
    revision: &'static str,
    files: [ModelFile; 3],
}

const TOKENIZER: ModelFile = ModelFile {
    name: "tokenizer.json",
    sha256: "5eb60cec1e77aeeb6869a2bb5a8e01a84c3fe5d072d75369343021fe6f5310d0",
    size: 2_405_679,
};

const BASE_EN: ModelSpec = ModelSpec {
    repo: "openai/whisper-base.en",
    revision: "911407f4214e0e1d82085af863093ec0b66f9cd6",
    files: [
        ModelFile {
            name: "config.json",
            sha256: "160c1df40a60d4ef5a6014d536fd845c33aabfd1f318458ac7f87cfe91ebbe76",
            size: 1_937,
        },
        TOKENIZER,
        ModelFile {
            name: "model.safetensors",
            sha256: "d4dd5542fd6a1d35639e21384238f3bfe6c557c849d392b5905d33ee29e71db5",
            size: 290_401_888,
        },
    ],
};

const SMALL_EN: ModelSpec = ModelSpec {
    repo: "openai/whisper-small.en",
    revision: "e8727524f962ee844a7319d92be39ac1bd25655a",
    files: [
        ModelFile {
            name: "config.json",
            sha256: "3441094addfb2a31ef62ea117e95bd6e437bbad8e93d3697fe420a4a6d3603a5",
            size: 1_943,
        },
        TOKENIZER,
        ModelFile {
            name: "model.safetensors",
            sha256: "6014ac49b506df900f66f4aca6b0801eed7245594ace97bcaf73e0ae5b863066",
            size: 966_992_008,
        },
    ],
};

/// Macs run on Metal, which is fast enough for the more accurate model. Other platforms run on the
/// CPU, where `small.en` takes several seconds per clip.
const MODEL: ModelSpec = if cfg!(target_os = "macos") {
    SMALL_EN
} else {
    BASE_EN
};

pub struct LocalWhisperTranscriber {
    client: Arc<http_client::Client>,
    model_dir: PathBuf,
    /// Held for the whole download and load, so concurrent callers wait for one attempt.
    model: Mutex<Option<WhisperModel>>,
}

impl LocalWhisperTranscriber {
    pub fn new(client: Arc<http_client::Client>) -> Self {
        let model_dir = warp_core::paths::cache_dir()
            .join("whisper")
            .join(MODEL.revision);
        Self {
            client,
            model_dir,
            model: Mutex::new(None),
        }
    }

    async fn load_model(&self) -> Result<WhisperModel> {
        async_fs::create_dir_all(&self.model_dir).await?;
        for file in &MODEL.files {
            let path = self.model_dir.join(file.name);
            if !path.exists() {
                let url = format!(
                    "https://huggingface.co/{}/resolve/{}/{}",
                    MODEL.repo, MODEL.revision, file.name
                );
                download_file(&self.client, &url, &path, file.sha256)
                    .await
                    .with_context(|| format!("failed to download {}", file.name))?;
            }
        }
        let model_dir = self.model_dir.clone();
        run_off_executor(move || WhisperModel::load(&model_dir)).await?
    }
}

#[async_trait]
impl Transcriber for LocalWhisperTranscriber {
    async fn transcribe(&self, wav_base64: String) -> Result<String, TranscribeError> {
        let wav = base64::engine::general_purpose::STANDARD
            .decode(wav_base64)
            .map_err(anyhow::Error::from)?;

        let mut slot = self.model.lock().await;
        let mut model = match slot.take() {
            Some(model) => model,
            None => self.load_model().await?,
        };
        let (model, text) = run_off_executor(move || {
            let text = model.transcribe_wav(&wav);
            (model, text)
        })
        .await?;
        *slot = Some(model);
        Ok(text?)
    }

    async fn prepare(&self) -> Result<(), TranscribeError> {
        let mut slot = self.model.lock().await;
        if slot.is_none() {
            *slot = Some(self.load_model().await?);
        }
        Ok(())
    }

    fn pending_download_bytes(&self) -> Option<u64> {
        let pending = MODEL
            .files
            .iter()
            .filter(|file| !self.model_dir.join(file.name).exists())
            .map(|file| file.size)
            .sum();
        (pending > 0).then_some(pending)
    }
}

/// Runs CPU-heavy model work on its own thread so that it can't stall the async executor.
async fn run_off_executor<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T> {
    let (sender, receiver) = oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .await
        .map_err(|_| anyhow!("Whisper thread panicked"))
}

/// Downloads `url` to `path`, which only appears once the contents match `sha256`.
async fn download_file(
    client: &http_client::Client,
    url: &str,
    path: &Path,
    sha256: &str,
) -> Result<()> {
    let response = client
        .get(url)
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .await?
        .error_for_status()?;

    let partial = path.with_extension("part");
    let mut file = async_fs::File::create(&partial).await?;
    let mut hasher = Sha256::new();
    let mut stream = std::pin::pin!(response.bytes_stream());
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);

    let actual = hex::encode(hasher.finalize());
    if actual != sha256 {
        async_fs::remove_file(&partial).await?;
    }
    ensure!(actual == sha256, "expected SHA-256 {sha256}, got {actual}");
    async_fs::rename(&partial, path).await?;
    Ok(())
}

#[cfg(test)]
#[path = "local_whisper_tests.rs"]
mod tests;
