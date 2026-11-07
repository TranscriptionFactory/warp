use std::sync::Arc;

use async_trait::async_trait;
use warpui::{Entity, SingletonEntity};

#[derive(thiserror::Error, Debug)]
pub enum TranscribeError {
    #[error("Request failed due to lack of Voice quota.")]
    QuotaLimit,

    #[error("OpenWarp is currently overloaded. Please try again later.")]
    ServerOverloaded,

    #[error("Internal error occurred at transport layer.")]
    Transport,

    #[error("Failed to deserialize JSON.")]
    Deserialization,

    /// OpenWarp 已禁用语音转写(BYOP genai 协议无法承载音频)。
    #[error("Voice transcription is unavailable in OpenWarp.")]
    Disabled,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Interface for transcribing voice input.
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait Transcriber: Send + Sync {
    /// Transcribe the given base64 encoded wav file into text.
    /// This is expected to be async and called off the main thread.
    async fn transcribe(&self, wav_base64: String) -> Result<String, TranscribeError>;

    /// Gets ready to transcribe. Called when recording starts, so that setup overlaps with the
    /// user speaking.
    async fn prepare(&self) -> Result<(), TranscribeError> {
        Ok(())
    }

    /// The number of bytes that must be downloaded before transcription can run, if any.
    fn pending_download_bytes(&self) -> Option<u64> {
        None
    }
}

/// Starts preparing the transcriber for a recording that just began.
#[cfg(feature = "voice_input")]
pub fn prepare_transcriber<V: warpui::View>(ctx: &mut warpui::ViewContext<V>) {
    let Some(transcriber) = VoiceTranscriber::as_ref(ctx).transcriber().cloned() else {
        return;
    };
    if let Some(bytes) = transcriber.pending_download_bytes() {
        let message = crate::t!(
            "editor-voice-model-download-toast",
            size_mb = (bytes / 1_000_000).to_string()
        );
        let window_id = ctx.window_id();
        crate::workspace::ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
            let toast = crate::view_components::DismissibleToast::default(message);
            toast_stack.add_ephemeral_toast(toast, window_id, ctx);
        });
    }
    // `transcribe` retries, and reports to the user, if this fails.
    ctx.spawn(async move { transcriber.prepare().await }, |_, result, _| {
        if let Err(e) = result {
            log::warn!("Failed to prepare voice transcription: {e:?}");
        }
    });
}

/// A voice transcriber that is enabled or disabled.
///
/// This is a singleton model that the app can decide to enable or disable.
/// The editor does expect that it will exist as a singleton fetchable from app context
/// either way though, and depending on whether the optional transcriber is set,
/// the editor considers transcriber to be enabled or disabled.
///
/// We set it up this way to avoid the editor having a direct dependency on any server api.
pub struct VoiceTranscriber {
    /// The transcriber to use. If `None`, the transcriber is disabled.
    #[cfg_attr(not(feature = "voice_input"), allow(dead_code))]
    transcriber: Option<Arc<dyn Transcriber>>,
}

impl VoiceTranscriber {
    pub fn new(transcriber: Arc<dyn Transcriber>) -> Self {
        Self {
            transcriber: Some(transcriber),
        }
    }

    /// OpenWarp(本地化,Phase 4):创建一个禁用的 transcriber。原语义上 `Some(...)`
    /// 代表云端 STT 后端可用,`None` 代表 "transcriber disabled";本地化后云端
    /// `ServerVoiceTranscriber`(调 server_api.transcribe 发 Wispr STT)不可用,改走该构造子。
    pub fn disabled() -> Self {
        Self { transcriber: None }
    }

    /// Returns the transcriber if one is set.
    pub fn transcriber(&self) -> Option<&Arc<dyn Transcriber>> {
        self.transcriber.as_ref()
    }
}

impl Entity for VoiceTranscriber {
    type Event = ();
}

impl SingletonEntity for VoiceTranscriber {}
