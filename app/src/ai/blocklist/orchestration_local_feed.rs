//! Feeds local orchestration children (Warp-agent and CLI-harness children
//! launched by `run_agents` in this process) into `OrchestrationChildTracker`.
//!
//! A local child is a conversation with a parent link and a task id that is
//! not a remote placeholder. Everything is derived from
//! [`BlocklistAIHistoryEvent`]s: the first event that shows such a
//! conversation registers it as an in-band child (`ChildSpawned`), and every
//! status change after that is broadcast as `ChildStatusChanged`. Remote
//! children stay on the SSE path in `OrchestrationEventStreamer`.
//!
//! Trackers are keyed by parent conversation, because a BYOP parent may have
//! no task id. Such a parent gets a locally minted task id on its first child;
//! [`LocalChildFeed::parent_task_id`] exposes it so consumers can match the
//! broadcasts to the parent.

use std::collections::{HashMap, HashSet};

use warp_core::features::FeatureFlag;
use warpui::{ModelContext, SingletonEntity};

use super::history_model::{BlocklistAIHistoryEvent, BlocklistAIHistoryModel};
use super::orchestration_child_tracker::{ChildSignal, OrchestrationChildTracker};
use super::orchestration_event_streamer::OrchestrationEventStreamer;
use crate::ai::agent::conversation::{AIConversationId, ConversationStatus};
use crate::ai::ambient_agents::AmbientAgentTaskId;

/// Local-child trackers owned by `OrchestrationEventStreamer`.
#[derive(Default)]
pub struct LocalChildFeed {
    /// One tracker per parent conversation that has local children.
    trackers: HashMap<AIConversationId, OrchestrationChildTracker>,
    /// Last broadcast status per registered child conversation. Presence
    /// means the child is registered; the value suppresses repeat broadcasts,
    /// since the history model emits on every status write.
    last_status: HashMap<AIConversationId, ConversationStatus>,
}

/// What the feed needs from a local child's conversation.
struct LocalChild {
    parent_conversation_id: AIConversationId,
    parent_task_id: Option<AmbientAgentTaskId>,
    run_id: String,
    status: ConversationStatus,
}

impl LocalChildFeed {
    /// Routes a history event to the local trackers.
    pub fn on_history_event(
        &mut self,
        event: &BlocklistAIHistoryEvent,
        killed_run_ids: &HashSet<String>,
        ctx: &mut ModelContext<OrchestrationEventStreamer>,
    ) {
        if !FeatureFlag::MultiLevelOrchestration.is_enabled()
            || !FeatureFlag::OrchestrationUnifiedStack.is_enabled()
        {
            return;
        }
        match event {
            BlocklistAIHistoryEvent::StartedNewConversation {
                new_conversation_id,
                ..
            } => self.observe(*new_conversation_id, killed_run_ids, ctx),
            BlocklistAIHistoryEvent::UpdatedConversationStatus {
                conversation_id, ..
            }
            | BlocklistAIHistoryEvent::UpdatedConversationMetadata {
                conversation_id, ..
            } => self.observe(*conversation_id, killed_run_ids, ctx),
            BlocklistAIHistoryEvent::RemoveConversation {
                conversation_id, ..
            }
            | BlocklistAIHistoryEvent::DeletedConversation {
                conversation_id, ..
            } => {
                self.last_status.remove(conversation_id);
                self.trackers.remove(conversation_id);
            }
            _ => {}
        }
    }

    /// The task id that broadcasts for `parent_conversation_id`'s local
    /// children carry, once it has at least one.
    pub fn parent_task_id(
        &self,
        parent_conversation_id: AIConversationId,
    ) -> Option<AmbientAgentTaskId> {
        self.trackers
            .get(&parent_conversation_id)
            .map(OrchestrationChildTracker::parent_task_id)
    }

    /// Registers `conversation_id` on first sight and broadcasts its status
    /// when it changed. Conversations that are not local children are ignored.
    fn observe(
        &mut self,
        conversation_id: AIConversationId,
        killed_run_ids: &HashSet<String>,
        ctx: &mut ModelContext<OrchestrationEventStreamer>,
    ) {
        let Some(child) = local_child(BlocklistAIHistoryModel::as_ref(ctx), conversation_id) else {
            return;
        };
        if self.last_status.get(&conversation_id) == Some(&child.status) {
            return;
        }
        let tracker = self
            .trackers
            .entry(child.parent_conversation_id)
            .or_insert_with(|| {
                OrchestrationChildTracker::new(
                    child
                        .parent_task_id
                        .unwrap_or_else(AmbientAgentTaskId::new_local),
                )
            });
        if !self.last_status.contains_key(&conversation_id) {
            tracker.observe_child(&child.run_id, ChildSignal::Registered, killed_run_ids, ctx);
        }
        self.last_status
            .insert(conversation_id, child.status.clone());
        tracker.observe_child(
            &child.run_id,
            ChildSignal::LocalStatus(child.status),
            killed_run_ids,
            ctx,
        );
    }
}

/// Reads `conversation_id` as a local child: parent link and task id set,
/// and not a placeholder for a run hosted elsewhere.
fn local_child(
    history: &BlocklistAIHistoryModel,
    conversation_id: AIConversationId,
) -> Option<LocalChild> {
    let conversation = history.conversation(&conversation_id)?;
    if conversation.is_remote_child() || conversation.is_viewing_shared_session() {
        return None;
    }
    let parent_conversation_id = conversation.parent_conversation_id()?;
    let run_id = conversation.run_id()?;
    Some(LocalChild {
        parent_conversation_id,
        parent_task_id: history
            .conversation(&parent_conversation_id)
            .and_then(|parent| parent.task_id()),
        run_id,
        status: conversation.status().clone(),
    })
}

#[cfg(test)]
#[path = "orchestration_local_feed_tests.rs"]
mod tests;
