//! Minimal local stand-in for upstream's `server_api` module.
//!
//! Ported for the orchestration unified stack (warpdotdev 8ba89e110,
//! QUALITY-928 M1). Upstream's `ServerApi` is a full authenticated HTTP
//! client stack against Warp's cloud backend; this fork has no cloud
//! backend, so this module vendors only the surface the orchestration
//! event streamer consumes, with every transport disabled. Local children
//! are driven in-band (registration + lifecycle signals from local
//! executors), so nothing here needs to reach a server for local-only
//! orchestration semantics.

use std::sync::Arc;

use anyhow::anyhow;

use crate::ai::agent_events::AgentEventFilter;

pub mod ai;

/// Minimal stand-in for upstream's `ServerApi`. Holds the (disabled) AI
/// client and rejects every streaming request.
pub struct ServerApi {
    ai_client: Arc<dyn ai::AIClient>,
}

impl ServerApi {
    fn new_disabled() -> Self {
        Self {
            ai_client: Arc::new(ai::DisabledAIClient),
        }
    }

    /// Upstream opens a cloud SSE stream here; this fork has no cloud RTC
    /// endpoint, so every open attempt errors and the shared driver backs
    /// off. See [`crate::ai::agent_events::ServerApiAgentEventSource`].
    pub(crate) async fn stream_agent_events(
        &self,
        filter: &AgentEventFilter,
        _since_sequence: i64,
    ) -> anyhow::Result<http_client::EventSourceStream> {
        Err(anyhow!(
            "agent event streaming ({}) is disabled in this build - no cloud RTC endpoint",
            filter.log_label()
        ))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn ai_client(&self) -> Arc<dyn ai::AIClient> {
        self.ai_client.clone()
    }
}

/// A singleton entity that provides access to the global [`ServerApi`]
/// instance, or any of its implemented trait objects.
pub struct ServerApiProvider {
    server_api: Arc<ServerApi>,
}

impl ServerApiProvider {
    /// Constructs a provider around the disabled local `ServerApi`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new_disabled() -> Self {
        Self {
            server_api: Arc::new(ServerApi::new_disabled()),
        }
    }

    /// Constructs a new ServerApiProvider for tests.
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self::new_disabled()
    }

    pub fn get(&self) -> Arc<ServerApi> {
        self.server_api.clone()
    }

    pub fn get_ai_client(&self) -> Arc<dyn ai::AIClient> {
        self.server_api.ai_client.clone()
    }
}

impl warpui::Entity for ServerApiProvider {
    type Event = ();
}

impl warpui::SingletonEntity for ServerApiProvider {}
