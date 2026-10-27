//! Launches `run_agents` children locally into hidden child panes (adapted from
//! upstream `terminal_pane.rs` `dispatch_start_agent_conversation`, local arms
//! only).
//!
//! Every child gets a hidden terminal pane and a child conversation whose task
//! id and run id are set before the launch outcome is reported back through
//! [`StartAgentExecutor::complete`]. Upstream mints Warp-agent task ids on the
//! server; here they come from the request's `local_task_id`.
use std::collections::HashMap;
use std::ffi::OsString;

use warp_cli::agent::Harness;
use warpui::{ModelHandle, SingletonEntity, ViewContext, ViewHandle};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::ai::blocklist::agent_view::AgentViewEntryOrigin;
use crate::ai::blocklist::{
    BlocklistAIHistoryModel, StartAgentExecutionMode, StartAgentExecutor, StartAgentOutcome,
    StartAgentRequest,
};
use crate::ai::execution_profiles::profiles::AIExecutionProfilesModel;
use crate::ai::llms::{LLMId, LLMPreferences};
use crate::pane_group::{PaneGroup, PaneId, TerminalPaneId};
use crate::terminal::TerminalView;

/// Launches one child and reports its outcome to `executor`.
pub(super) fn dispatch_start_agent_conversation(
    group: &mut PaneGroup,
    parent_pane_id: PaneId,
    terminal_pane_id: TerminalPaneId,
    request: StartAgentRequest,
    executor: ModelHandle<StartAgentExecutor>,
    ctx: &mut ViewContext<PaneGroup>,
) {
    match request.execution_mode.clone() {
        StartAgentExecutionMode::Local {
            harness_type: None, ..
        } => launch_local_warp_child(group, parent_pane_id, request, executor, ctx),
        #[cfg(not(target_family = "wasm"))]
        StartAgentExecutionMode::Local {
            harness_type: Some(harness),
            ..
        } => launch_local_harness_child(
            group,
            parent_pane_id,
            terminal_pane_id,
            request,
            harness,
            executor,
            ctx,
        ),
        #[cfg(target_family = "wasm")]
        StartAgentExecutionMode::Local {
            harness_type: Some(_),
            ..
        } => {
            let _ = terminal_pane_id;
            complete(
                &executor,
                &request,
                StartAgentOutcome::Error(
                    "Local child harnesses are not supported in WASM builds.".to_string(),
                ),
                ctx,
            );
        }
    }
}

fn complete(
    executor: &ModelHandle<StartAgentExecutor>,
    request: &StartAgentRequest,
    outcome: StartAgentOutcome,
    ctx: &mut ViewContext<PaneGroup>,
) {
    let request_id = request.id;
    executor.update(ctx, |executor, _| executor.complete(request_id, outcome));
}

/// Creates a hidden child pane and a child conversation in it, inheriting the
/// parent's execution profile, stamps `task_id` onto the conversation as its
/// task id and run id, and makes it the pane's active conversation.
fn create_hidden_child_conversation(
    group: &mut PaneGroup,
    parent_pane_id: PaneId,
    request: &StartAgentRequest,
    orchestration_harness: Harness,
    env_vars: HashMap<OsString, OsString>,
    task_id: AmbientAgentTaskId,
    ctx: &mut ViewContext<PaneGroup>,
) -> Option<(ViewHandle<TerminalView>, AIConversationId)> {
    let new_pane_id =
        group.insert_terminal_pane_hidden_for_child_agent(parent_pane_id, env_vars, ctx);
    let Some(terminal_view) = group.terminal_view_from_pane_id(new_pane_id, ctx) else {
        log::error!("Failed to get terminal view for new child agent pane");
        group.discard_pane(new_pane_id.into(), ctx);
        return None;
    };
    let terminal_view_id = terminal_view.id();

    match group.terminal_view_from_pane_id(parent_pane_id, ctx) {
        Some(parent_terminal_view) => {
            let parent_profile_id = AIExecutionProfilesModel::as_ref(ctx)
                .active_profile(Some(parent_terminal_view.id()), ctx)
                .id()
                .clone();
            AIExecutionProfilesModel::handle(ctx).update(ctx, |profiles, ctx| {
                profiles.set_active_profile(terminal_view_id, parent_profile_id, ctx);
            });
        }
        None => log::warn!(
            "Could not find parent terminal view for pane {parent_pane_id:?}; child will use the default AI profile"
        ),
    }

    let conversation_id = BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
        let conversation_id = history_model.start_new_child_conversation(
            terminal_view_id,
            request.name.clone(),
            request.parent_conversation_id,
            Some(orchestration_harness),
            ctx,
        );
        history_model.assign_run_id_for_conversation(
            conversation_id,
            task_id.to_string(),
            Some(task_id),
            terminal_view_id,
            ctx,
        );
        // CLI harness status reaches the child only through the pane's active
        // conversation (`TerminalView` maps `StatusChanged` onto it).
        history_model.mark_active_conversation_id(conversation_id, terminal_view_id, ctx);
        conversation_id
    });
    group
        .child_agent_panes
        .insert(conversation_id, new_pane_id.into());
    Some((terminal_view, conversation_id))
}

/// Starts a Warp-agent child: a hidden child conversation that receives the
/// child prompt as an agent-initiated query.
fn launch_local_warp_child(
    group: &mut PaneGroup,
    parent_pane_id: PaneId,
    request: StartAgentRequest,
    executor: ModelHandle<StartAgentExecutor>,
    ctx: &mut ViewContext<PaneGroup>,
) {
    let Some(task_id) = request.local_task_id else {
        complete(
            &executor,
            &request,
            StartAgentOutcome::Error("Local child agent is missing its task id.".to_string()),
            ctx,
        );
        return;
    };
    let StartAgentExecutionMode::Local { model_id, .. } = &request.execution_mode;
    let preferences = LLMPreferences::as_ref(ctx);
    let model = match model_id {
        Some(model_id) => {
            let id = LLMId::from(model_id.clone());
            let Some(model) = preferences.get_llm_info(&id) else {
                complete(
                    &executor,
                    &request,
                    StartAgentOutcome::Error(format!("Unknown child agent model: {model_id}")),
                    ctx,
                );
                return;
            };
            model.id.clone()
        }
        None => {
            let parent_view_id = group
                .terminal_view_from_pane_id(parent_pane_id, ctx)
                .map(|view| view.id());
            preferences
                .get_active_base_model(ctx, parent_view_id)
                .id
                .clone()
        }
    };
    let Some((terminal_view, conversation_id)) = create_hidden_child_conversation(
        group,
        parent_pane_id,
        &request,
        Harness::Oz,
        HashMap::new(),
        task_id,
        ctx,
    ) else {
        complete(
            &executor,
            &request,
            StartAgentOutcome::Error(
                "Failed to create a hidden pane for the local child agent.".to_string(),
            ),
            ctx,
        );
        return;
    };

    LLMPreferences::handle(ctx).update(ctx, |preferences, ctx| {
        preferences.set_child_agent_model(model, terminal_view.id(), ctx);
    });

    let prompt = request.prompt.clone();
    terminal_view.update(ctx, |terminal_view, ctx| {
        terminal_view
            .ai_controller()
            .update(ctx, |controller, ctx| {
                controller.send_agent_query_in_conversation(prompt, conversation_id, ctx);
            });
        terminal_view.enter_agent_view(
            None,
            Some(conversation_id),
            AgentViewEntryOrigin::ChildAgent,
            ctx,
        );
    });
    complete(
        &executor,
        &request,
        StartAgentOutcome::Started {
            agent_id: task_id.to_string(),
        },
        ctx,
    );
}

/// Prepares a CLI-harness launch (validates the shell and CLI, builds the
/// command), then creates the hidden child pane and runs the command in it.
#[cfg(not(target_family = "wasm"))]
fn launch_local_harness_child(
    group: &mut PaneGroup,
    parent_pane_id: PaneId,
    terminal_pane_id: TerminalPaneId,
    request: StartAgentRequest,
    harness: Harness,
    executor: ModelHandle<StartAgentExecutor>,
    ctx: &mut ViewContext<PaneGroup>,
) {
    use super::local_harness_launch::{
        PreparedLocalHarnessLaunch, prepare_local_harness_child_launch,
    };

    let startup_directory = group.startup_path_for_new_session(Some(terminal_pane_id), ctx);
    let shell_type = group
        .terminal_view_from_pane_id(parent_pane_id, ctx)
        .and_then(|terminal_view| terminal_view.as_ref(ctx).active_session_shell_type(ctx));
    let launch = prepare_local_harness_child_launch(
        request.prompt.clone(),
        harness.to_string(),
        request.parent_run_id.clone(),
        shell_type,
        startup_directory,
    );
    let _ = ctx.spawn(launch, move |group, result, ctx| {
        let PreparedLocalHarnessLaunch {
            command,
            env_vars,
            run_id,
            task_id,
        } = match result {
            Ok(launch) => launch,
            Err(error) => {
                complete(&executor, &request, StartAgentOutcome::Error(error), ctx);
                return;
            }
        };
        let Some((terminal_view, conversation_id)) = create_hidden_child_conversation(
            group,
            parent_pane_id,
            &request,
            harness,
            env_vars,
            task_id,
            ctx,
        ) else {
            complete(
                &executor,
                &request,
                StartAgentOutcome::Error(
                    "Failed to create a hidden pane for the local child harness.".to_string(),
                ),
                ctx,
            );
            return;
        };
        terminal_view.update(ctx, |terminal_view, ctx| {
            terminal_view.execute_command_or_set_pending(&command, ctx);
            terminal_view.enter_agent_view(
                None,
                Some(conversation_id),
                AgentViewEntryOrigin::ChildAgent,
                ctx,
            );
        });
        complete(
            &executor,
            &request,
            StartAgentOutcome::Started { agent_id: run_id },
            ctx,
        );
    });
}

#[cfg(test)]
#[path = "local_child_launch_tests.rs"]
mod tests;
