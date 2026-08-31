use std::sync::Arc;

use crate::ai::agent::ReceivedMessageInput;
use crate::ai::agent_events::AgentRunEvent;
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::server::server_api::ai::AIClient;
use crate::server::server_api::ServerApi;

/// Zap 本地构建不再从云端 mailbox 拉取消息正文或发送 delivered 回执。
/// 该类型保留本地 harness 桥接调用面的无副作用兼容语义。
///
/// The constructor surface mirrors upstream (warpdotdev 8ba89e110) so the
/// orchestration event streamer can be ported faithfully: the supplied
/// clients are ignored because hydration is a no-op in this fork.
#[derive(Clone)]
pub(crate) struct MessageHydrator;

impl MessageHydrator {
    /// Local no-op hydrator with no client. Used by harness bridges.
    pub(crate) fn disabled() -> Self {
        Self
    }

    /// Upstream shape: hydrator backed by an [`AIClient`]. The client is
    /// unused in this fork.
    pub(crate) fn new(_ai_client: Arc<dyn AIClient>) -> Self {
        Self
    }

    /// Upstream shape: hydrator scoped to one task via [`ServerApi`]. Both
    /// arguments are unused in this fork.
    pub(crate) fn for_task(_server_api: Arc<ServerApi>, _task_id: AmbientAgentTaskId) -> Self {
        Self
    }

    pub(crate) async fn hydrate_event_for_recipient(
        &self,
        event: &AgentRunEvent,
        recipient_run_id: &str,
    ) -> Option<ReceivedMessageInput> {
        if event.event_type != "new_message" || event.run_id != recipient_run_id {
            return None;
        }

        None
    }

    pub(crate) async fn mark_messages_delivered_best_effort<'a, I>(
        &self,
        _message_ids: I,
    ) -> Vec<(String, anyhow::Error)>
    where
        I: IntoIterator<Item = &'a str>,
    {
        Vec::new()
    }
}
