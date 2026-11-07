#[cfg(all(feature = "voice_input", not(target_family = "wasm")))]
pub mod local_whisper;
pub mod transcriber;
