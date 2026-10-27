//! Async executor for the fork-local `AIAgentActionType::RunAgents`.
//!
//! Adapted from upstream `run_agents.rs`, local execution only (no plan
//! publication, remote children, approved plan config or auth secrets). Fans
//! out per child via [`StartAgentExecutor::dispatch`] and aggregates the
//! outcomes into a single `RunAgentsResult`.
use std::time::Duration;

use ai::agent::action::{RunAgentsAgentRunConfig, RunAgentsRequest};
use ai::agent::action_result::{RunAgentsAgentOutcome, RunAgentsAgentOutcomeKind, RunAgentsResult};
use futures::FutureExt;
use futures::future::BoxFuture;
use warp_cli::agent::Harness;
use warp_core::execution_mode::AppExecutionMode;
use warp_core::features::FeatureFlag;
use warpui::{ModelContext, ModelHandle, SingletonEntity};

use super::start_agent::{StartAgentExecutionMode, StartAgentExecutor, StartAgentOutcome};
use super::{ActionExecution, AnyActionExecution, ExecuteActionInput, PreprocessActionInput};
use crate::ai::agent::{AIAgentAction, AIAgentActionResultType, AIAgentActionType};
use crate::ai::blocklist::BlocklistAIHistoryModel;

/// Per-child spawn timeout. If a child doesn't report back within this window
/// (e.g. binary not found), its slot is failed rather than hanging the run.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RunAgentsExecutor {
    start_agent_executor: ModelHandle<StartAgentExecutor>,
}

impl warpui::Entity for RunAgentsExecutor {
    type Event = ();
}

enum ChildSlot {
    Failed(String),
    Pending(async_channel::Receiver<StartAgentOutcome>),
}

impl RunAgentsExecutor {
    pub fn new(start_agent_executor: ModelHandle<StartAgentExecutor>) -> Self {
        Self {
            start_agent_executor,
        }
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> + use<> {
        let AIAgentAction { action, .. } = input.action;
        let AIAgentActionType::RunAgents(request) = action else {
            return ActionExecution::InvalidAction;
        };
        let request = request.as_ref().clone();
        let parent_conversation_id = input.conversation_id;
        let parent = BlocklistAIHistoryModel::as_ref(ctx).conversation(&parent_conversation_id);
        let is_child_conversation = parent.is_some_and(|c| c.is_child_agent_conversation());
        let parent_run_id = parent.and_then(|c| c.run_id());

        if let Err(reason) = validate_request(&request, is_child_conversation) {
            return ActionExecution::Sync(AIAgentActionResultType::RunAgents(
                RunAgentsResult::Denied { reason },
            ));
        }

        let RunAgentsRequest {
            base_prompt,
            model_id,
            harness_type,
            agent_run_configs,
            ..
        } = request;

        let slots: Vec<ChildSlot> = agent_run_configs
            .iter()
            .map(
                |cfg| match run_agents_to_start_agent_mode(&harness_type, &model_id, cfg) {
                    Ok(mode) => {
                        let prompt = compose_run_agents_child_prompt(&base_prompt, &cfg.prompt);
                        ChildSlot::Pending(self.start_agent_executor.update(
                            ctx,
                            |executor, ctx| {
                                executor.dispatch(
                                    cfg.name.clone(),
                                    cfg.title.clone(),
                                    prompt,
                                    mode,
                                    parent_conversation_id,
                                    parent_run_id.clone(),
                                    ctx,
                                )
                            },
                        ))
                    }
                    Err(error) => ChildSlot::Failed(error),
                },
            )
            .collect();

        ActionExecution::new_async(
            async move {
                let mut outcomes = Vec::with_capacity(slots.len());
                for slot in slots {
                    outcomes.push(match slot {
                        ChildSlot::Failed(error) => RunAgentsAgentOutcomeKind::Failed { error },
                        ChildSlot::Pending(recv) => await_child_outcome(recv).await,
                    });
                }
                outcomes
            },
            move |outcomes, _ctx| {
                let agents = agent_run_configs
                    .iter()
                    .zip(outcomes)
                    .map(|(cfg, kind)| RunAgentsAgentOutcome {
                        name: cfg.name.clone(),
                        kind,
                    })
                    .collect();
                AIAgentActionResultType::RunAgents(RunAgentsResult::Launched {
                    model_id,
                    harness_type,
                    agents,
                })
            },
        )
    }

    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        if !matches!(input.action.action, AIAgentActionType::RunAgents(_)) {
            return false;
        }
        // Child conversations live in hidden panes where a confirmation card
        // would be invisible and hang the run; `validate_request` denies them
        // gracefully when multi-level orchestration is off.
        AppExecutionMode::as_ref(ctx).is_autonomous()
            || BlocklistAIHistoryModel::as_ref(ctx)
                .conversation(&input.conversation_id)
                .is_some_and(|c| c.is_child_agent_conversation())
    }

    pub(super) fn preprocess_action(
        &mut self,
        _action: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
}

async fn await_child_outcome(
    recv: async_channel::Receiver<StartAgentOutcome>,
) -> RunAgentsAgentOutcomeKind {
    let timeout = warpui::r#async::Timer::after(SPAWN_TIMEOUT);
    match futures::future::select(Box::pin(recv.recv()), Box::pin(timeout)).await {
        futures::future::Either::Left((Ok(StartAgentOutcome::Started { agent_id }), _)) => {
            RunAgentsAgentOutcomeKind::Launched { agent_id }
        }
        futures::future::Either::Left((Ok(StartAgentOutcome::Error(error)), _)) => {
            RunAgentsAgentOutcomeKind::Failed { error }
        }
        futures::future::Either::Left((Err(_), _)) => RunAgentsAgentOutcomeKind::Failed {
            error: "Cancelled before launch".to_string(),
        },
        futures::future::Either::Right(_) => RunAgentsAgentOutcomeKind::Failed {
            error: format!(
                "Agent failed to start within {} seconds.",
                SPAWN_TIMEOUT.as_secs()
            ),
        },
    }
}

fn validate_request(request: &RunAgentsRequest, is_child_conversation: bool) -> Result<(), String> {
    if request.agent_run_configs.is_empty() {
        return Err("run_agents: empty agent_run_configs".to_string());
    }
    if is_child_conversation && !FeatureFlag::MultiLevelOrchestration.is_enabled() {
        return Err("Multi-level orchestration is not enabled on this client.".to_string());
    }
    Ok(())
}

/// Joins `base_prompt` and a per-agent prompt with `"\n\n"`,
/// falling back to whichever is non-empty.
pub fn compose_run_agents_child_prompt(base_prompt: &str, per_agent_prompt: &str) -> String {
    let base_trimmed = base_prompt.trim();
    let per_agent_trimmed = per_agent_prompt.trim();
    match (base_trimmed.is_empty(), per_agent_trimmed.is_empty()) {
        (false, false) => format!("{base_prompt}\n\n{per_agent_prompt}"),
        (false, true) => base_prompt.to_string(),
        (true, false) => per_agent_prompt.to_string(),
        (true, true) => String::new(),
    }
}

/// Translates run-wide + per-agent config into a child's
/// [`StartAgentExecutionMode`]. The per-agent `harness_type` / `model_id`
/// override the run-wide values when non-empty. Returns `Err` for an unknown
/// harness or a CLI harness while `LocalClaudeCodexChildHarnesses` is off.
pub fn run_agents_to_start_agent_mode(
    run_harness_type: &str,
    run_model_id: &str,
    cfg: &RunAgentsAgentRunConfig,
) -> Result<StartAgentExecutionMode, String> {
    let pick = |per_agent: &str, run_wide: &str| {
        let per_agent = per_agent.trim();
        if per_agent.is_empty() {
            run_wide.trim().to_string()
        } else {
            per_agent.to_string()
        }
    };
    let harness = pick(&cfg.harness_type, run_harness_type);
    let model_id = pick(&cfg.model_id, run_model_id);
    let model_id = (!model_id.is_empty()).then_some(model_id);
    if harness.is_empty() || harness.eq_ignore_ascii_case("oz") {
        return Ok(StartAgentExecutionMode::Local {
            harness_type: None,
            model_id,
        });
    }
    let Some(parsed) = Harness::parse_local_child_harness(&harness) else {
        return Err(format!("Unsupported local child harness '{harness}'."));
    };
    if !FeatureFlag::LocalClaudeCodexChildHarnesses.is_enabled() {
        return Err(format!(
            "Local {} child agents are disabled on this client.",
            parsed.display_name()
        ));
    }
    Ok(StartAgentExecutionMode::Local {
        harness_type: Some(parsed),
        model_id,
    })
}

#[cfg(test)]
#[path = "run_agents_tests.rs"]
mod tests;
