//! Per-child launch requests for local orchestration (adapted from upstream
//! `start_agent.rs`, local execution only).
//!
//! [`StartAgentExecutor::dispatch`] emits one [`StartAgentExecutorEvent::CreateAgent`]
//! per child and returns a receiver for its [`StartAgentOutcome`]. Whoever
//! subscribes to `CreateAgent` (the pane launcher) launches the child and reports
//! back through [`StartAgentExecutor::complete`]. Upstream instead derives the
//! outcome from server-backed history events, which this fork does not have.
use std::collections::HashMap;

use warp_cli::agent::Harness;
use warpui::{Entity, ModelContext};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::ambient_agents::AmbientAgentTaskId;

/// Per-request outcome of a child launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartAgentOutcome {
    Started {
        agent_id: String,
    },
    /// An error occurred while starting the agent.
    Error(String),
}

/// Opaque, monotonically increasing request identifier.
/// Disambiguates parallel in-flight launch requests.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Default)]
pub struct StartAgentRequestId(u64);

/// How a child runs. Upstream also has a `Remote` arm; this fork launches
/// children locally only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartAgentExecutionMode {
    Local {
        /// `None` selects a Warp agent child; `Some(_)` a local CLI harness.
        harness_type: Option<Harness>,
        /// `None` inherits the parent agent's model.
        model_id: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct StartAgentRequest {
    pub id: StartAgentRequestId,
    pub name: String,
    pub title: String,
    pub prompt: String,
    pub execution_mode: StartAgentExecutionMode,
    /// Task id minted locally for Warp agent children (never via server
    /// `create_agent_task`). `None` for CLI-harness children: the harness
    /// launcher mints its own in `prepare_local_harness_child_launch`.
    pub local_task_id: Option<AmbientAgentTaskId>,
    pub parent_conversation_id: AIConversationId,
    pub parent_run_id: Option<String>,
}

pub enum StartAgentExecutorEvent {
    /// Launch this child, then call [`StartAgentExecutor::complete`] with its id.
    CreateAgent(Box<StartAgentRequest>),
}

#[derive(Default)]
pub struct StartAgentExecutor {
    pending: HashMap<StartAgentRequestId, async_channel::Sender<StartAgentOutcome>>,
    next_request_id: u64,
}

impl Entity for StartAgentExecutor {
    type Event = StartAgentExecutorEvent;
}

impl StartAgentExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_request_id(&mut self) -> StartAgentRequestId {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        StartAgentRequestId(id)
    }

    /// Emits a `CreateAgent` event for one child. Returns a receiver for the
    /// resulting [`StartAgentOutcome`].
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &mut self,
        name: String,
        title: String,
        prompt: String,
        execution_mode: StartAgentExecutionMode,
        parent_conversation_id: AIConversationId,
        parent_run_id: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) -> async_channel::Receiver<StartAgentOutcome> {
        let (sender, receiver) = async_channel::bounded(1);
        let request_id = self.next_request_id();
        self.pending.insert(request_id, sender);
        let local_task_id = match &execution_mode {
            StartAgentExecutionMode::Local {
                harness_type: None, ..
            } => Some(AmbientAgentTaskId::new_local()),
            StartAgentExecutionMode::Local {
                harness_type: Some(_),
                ..
            } => None,
        };
        ctx.emit(StartAgentExecutorEvent::CreateAgent(Box::new(
            StartAgentRequest {
                id: request_id,
                name,
                title,
                prompt,
                execution_mode,
                local_task_id,
                parent_conversation_id,
                parent_run_id,
            },
        )));
        receiver
    }

    /// Reports the launch outcome for a dispatched request. Unknown or
    /// already-completed ids are ignored.
    pub fn complete(&mut self, request_id: StartAgentRequestId, outcome: StartAgentOutcome) {
        if let Some(sender) = self.pending.remove(&request_id) {
            let _ = sender.try_send(outcome);
        }
    }
}
