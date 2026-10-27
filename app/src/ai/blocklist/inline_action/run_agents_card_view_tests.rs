use super::*;
use ai::agent::action_result::RunAgentsAgentOutcome;

#[test]
fn launch_results_include_each_child_failure() {
    let result = RunAgentsResult::Launched {
        model_id: String::new(),
        harness_type: "oz".into(),
        agents: vec![
            RunAgentsAgentOutcome {
                name: "builder".into(),
                kind: RunAgentsAgentOutcomeKind::Launched {
                    agent_id: "local-child".into(),
                },
            },
            RunAgentsAgentOutcome {
                name: "reviewer".into(),
                kind: RunAgentsAgentOutcomeKind::Failed {
                    error: "CLI missing".into(),
                },
            },
        ],
    };
    let text = result_text(&result);
    assert!(text.contains("Launched 1 of 2"));
    assert!(text.contains("builder: Started"));
    assert!(text.contains("reviewer: Failed: CLI missing"));
}

#[test]
fn denied_and_cancelled_results_remain_visible() {
    assert!(
        result_text(&RunAgentsResult::Denied {
            reason: "disabled".into()
        })
        .contains("disabled")
    );
    assert!(result_text(&RunAgentsResult::Cancelled).contains("cancelled"));
}
