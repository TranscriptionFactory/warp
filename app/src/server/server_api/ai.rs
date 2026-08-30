//! Minimal local stand-in for upstream's `server_api::ai` client surface.
//!
//! Ported for the orchestration unified stack (warpdotdev 8ba89e110,
//! QUALITY-928 M1). Vendors only the trait methods the orchestration event
//! streamer and child tracker actually call, plus the request/response
//! types they mention. `MockAIClient` (via `mockall::automock`) keeps the
//! upstream unit tests working unchanged; the runtime implementation is
//! [`DisabledAIClient`], which errors on every call because this fork has
//! no cloud backend.

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

pub(crate) use crate::ai::agent_events::AgentRunEvent;
use crate::ai::ambient_agents::{AmbientAgentTask, AmbientAgentTaskId, AmbientAgentTaskState};

/// Filter parameters for listing ambient agent tasks.
///
/// Vendored subset of upstream's struct: only the fields the orchestration
/// streamer sets are present.
#[derive(Clone, Debug, Default)]
pub struct TaskListFilter {
    pub states: Option<Vec<AmbientAgentTaskState>>,
    pub ancestor_run_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReadAgentMessageResponse {
    pub message_id: String,
    pub sender_run_id: String,
    pub subject: String,
    pub body: String,
    pub sent_at: String,
    pub delivered_at: Option<String>,
    pub read_at: Option<String>,
}

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AIClient: 'static + Send + Sync {
    async fn list_ambient_agent_tasks(
        &self,
        limit: i32,
        filter: TaskListFilter,
    ) -> anyhow::Result<Vec<AmbientAgentTask>, anyhow::Error>;

    async fn get_ambient_agent_task(
        &self,
        task_id: &AmbientAgentTaskId,
    ) -> anyhow::Result<AmbientAgentTask, anyhow::Error>;

    async fn update_event_sequence_on_server(
        &self,
        run_id: &str,
        sequence: i64,
    ) -> anyhow::Result<(), anyhow::Error>;

    async fn read_agent_message(
        &self,
        message_id: &str,
    ) -> anyhow::Result<ReadAgentMessageResponse, anyhow::Error>;
}

/// Runtime [`AIClient`] for this fork: every call errors because there is
/// no cloud backend to talk to.
pub(crate) struct DisabledAIClient;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AIClient for DisabledAIClient {
    async fn list_ambient_agent_tasks(
        &self,
        _limit: i32,
        _filter: TaskListFilter,
    ) -> anyhow::Result<Vec<AmbientAgentTask>, anyhow::Error> {
        Err(anyhow::anyhow!(
            "listing ambient agent tasks is disabled in this build - no cloud backend"
        ))
    }

    async fn get_ambient_agent_task(
        &self,
        _task_id: &AmbientAgentTaskId,
    ) -> anyhow::Result<AmbientAgentTask, anyhow::Error> {
        Err(anyhow::anyhow!(
            "fetching ambient agent tasks is disabled in this build - no cloud backend"
        ))
    }

    async fn update_event_sequence_on_server(
        &self,
        _run_id: &str,
        _sequence: i64,
    ) -> anyhow::Result<(), anyhow::Error> {
        Err(anyhow::anyhow!(
            "persisting event cursors to the server is disabled in this build - no cloud backend"
        ))
    }

    async fn read_agent_message(
        &self,
        _message_id: &str,
    ) -> anyhow::Result<ReadAgentMessageResponse, anyhow::Error> {
        Err(anyhow::anyhow!(
            "reading agent messages is disabled in this build - no cloud backend"
        ))
    }
}
