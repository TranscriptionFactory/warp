use std::sync::Arc;

use parking_lot::Mutex;
use warpui::App;

use super::*;
use crate::ai::agent::AIAgentActionId;
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::task::TaskId;
use crate::ai::blocklist::action_model::execute::start_agent::{
    StartAgentExecutorEvent, StartAgentRequest,
};

fn run_agents_action(request: RunAgentsRequest) -> AIAgentAction {
    AIAgentAction {
        id: AIAgentActionId::from("run-agents-1".to_string()),
        task_id: TaskId::new("task-1".to_string()),
        action: AIAgentActionType::RunAgents(Box::new(request)),
        requires_result: true,
    }
}

fn capture_create_agent(
    app: &mut App,
    start: &ModelHandle<StartAgentExecutor>,
) -> Arc<Mutex<Vec<StartAgentRequest>>> {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let captured_for_closure = captured.clone();
    app.update(|ctx| {
        ctx.subscribe_to_model(start, move |_, event, _| {
            let StartAgentExecutorEvent::CreateAgent(request) = event;
            captured_for_closure.lock().push(request.as_ref().clone());
        })
    });
    captured
}

#[test]
fn dispatches_one_create_agent_per_child_and_aggregates_outcomes() {
    App::test((), |mut app| async move {
        let _flag = FeatureFlag::LocalClaudeCodexChildHarnesses.override_enabled(true);
        app.add_singleton_model(|_| BlocklistAIHistoryModel::new_for_test());
        let start = app.add_model(|_| StartAgentExecutor::new());
        let captured = capture_create_agent(&mut app, &start);
        let executor = app.add_model(|_| RunAgentsExecutor::new(start.clone()));

        let action = run_agents_action(RunAgentsRequest {
            base_prompt: "Report back.".to_string(),
            agent_run_configs: vec![
                RunAgentsAgentRunConfig {
                    name: "warp-child".to_string(),
                    prompt: "Survey the grid.".to_string(),
                    ..Default::default()
                },
                RunAgentsAgentRunConfig {
                    name: "cli-child".to_string(),
                    prompt: "Survey the CLI.".to_string(),
                    harness_type: "claude".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });

        let execution = executor.update(&mut app, |executor, ctx| {
            let input = ExecuteActionInput {
                action: &action,
                conversation_id: AIConversationId::new(),
            };
            let execution: AnyActionExecution = executor.execute(input, ctx).into();
            execution
        });

        let requests = captured.lock().clone();
        assert_eq!(requests.len(), 2, "one CreateAgent per child");
        assert_eq!(requests[0].name, "warp-child");
        assert_eq!(requests[0].prompt, "Report back.\n\nSurvey the grid.");
        assert_eq!(
            requests[0].execution_mode,
            StartAgentExecutionMode::Local {
                harness_type: None,
                model_id: None,
            }
        );
        assert!(requests[0].local_task_id.is_some());
        assert_eq!(requests[1].name, "cli-child");
        assert_eq!(
            requests[1].execution_mode,
            StartAgentExecutionMode::Local {
                harness_type: Some(Harness::Claude),
                model_id: None,
            }
        );
        assert!(requests[1].local_task_id.is_none());

        let AnyActionExecution::Async {
            execute_future,
            on_complete,
        } = execution
        else {
            panic!("expected async execution");
        };

        start.update(&mut app, |start, _| {
            start.complete(
                requests[1].id,
                StartAgentOutcome::Error("claude is not installed".to_string()),
            );
            start.complete(
                requests[0].id,
                StartAgentOutcome::Started {
                    agent_id: "child-a".to_string(),
                },
            );
        });

        let outcomes = execute_future.await;
        let result = app.update(|ctx| on_complete(outcomes, ctx));
        assert_eq!(
            result,
            AIAgentActionResultType::RunAgents(RunAgentsResult::Launched {
                model_id: String::new(),
                harness_type: String::new(),
                agents: vec![
                    RunAgentsAgentOutcome {
                        name: "warp-child".to_string(),
                        kind: RunAgentsAgentOutcomeKind::Launched {
                            agent_id: "child-a".to_string(),
                        },
                    },
                    RunAgentsAgentOutcome {
                        name: "cli-child".to_string(),
                        kind: RunAgentsAgentOutcomeKind::Failed {
                            error: "claude is not installed".to_string(),
                        },
                    },
                ],
            })
        );
    });
}

#[test]
fn start_agent_mode_resolves_overrides_and_rejects_unknown_harness() {
    let _flag = FeatureFlag::LocalClaudeCodexChildHarnesses.override_enabled(true);
    let cfg = RunAgentsAgentRunConfig {
        model_id: "child-model".to_string(),
        ..Default::default()
    };
    assert_eq!(
        run_agents_to_start_agent_mode("codex", "run-model", &cfg),
        Ok(StartAgentExecutionMode::Local {
            harness_type: Some(Harness::Codex),
            model_id: Some("child-model".to_string()),
        })
    );
    assert_eq!(
        run_agents_to_start_agent_mode("oz", "run-model", &RunAgentsAgentRunConfig::default()),
        Ok(StartAgentExecutionMode::Local {
            harness_type: None,
            model_id: Some("run-model".to_string()),
        })
    );
    assert!(run_agents_to_start_agent_mode("gemini", "", &cfg).is_err());
}
