use ai::agent::action_result::RunAgentsAgentOutcome;

use super::*;

const SAMPLE_ARGS: &str = r#"{
    "summary": "Investigate two crates",
    "base_prompt": "Report findings as file:line bullets.",
    "harness_type": "",
    "agents": [
        { "name": "grid", "prompt": "Survey crates/warp_terminal." },
        { "name": "cli", "prompt": "Survey crates/warp_cli.", "harness_type": "claude", "model_id": "sonnet" }
    ]
}"#;

#[test]
fn schema_requires_agents_with_name_and_prompt() {
    let schema = parameters();
    assert_eq!(schema["required"], json!(["agents"]));
    assert_eq!(
        schema["properties"]["agents"]["items"]["required"],
        json!(["name", "prompt"])
    );
    assert_eq!(RUN_AGENTS.name, "run_agents");
    assert!(super::super::lookup(TOOL_NAME).is_some());
}

#[test]
fn parse_args_round_trips_sample_call() {
    let request = parse_args(SAMPLE_ARGS).expect("sample args parse");
    assert_eq!(request.summary, "Investigate two crates");
    assert_eq!(request.base_prompt, "Report findings as file:line bullets.");
    assert_eq!(request.harness_type, "");
    assert_eq!(
        request.agent_run_configs,
        vec![
            RunAgentsAgentRunConfig {
                name: "grid".to_string(),
                prompt: "Survey crates/warp_terminal.".to_string(),
                ..Default::default()
            },
            RunAgentsAgentRunConfig {
                name: "cli".to_string(),
                prompt: "Survey crates/warp_cli.".to_string(),
                model_id: "sonnet".to_string(),
                harness_type: "claude".to_string(),
                ..Default::default()
            },
        ]
    );
}

#[test]
fn parse_args_rejects_empty_or_malformed_agents() {
    assert!(parse_args(r#"{"agents": []}"#).is_err());
    assert!(parse_args(r#"{"base_prompt": "x"}"#).is_err());
    assert!(parse_args("not json").is_err());
}

#[test]
fn from_args_is_never_used() {
    assert!((RUN_AGENTS.from_args)(SAMPLE_ARGS).is_err());
}

#[test]
fn parse_carrier_matches_only_run_agents() {
    let carrier = format!("{TOOL_NAME}\n{SAMPLE_ARGS}");
    let request = parse_carrier(&carrier)
        .expect("run_agents carrier recognized")
        .expect("carrier args parse");
    assert_eq!(request.agent_run_configs.len(), 2);

    assert!(parse_carrier("todowrite\n{\"todos\":[]}").is_none());
    assert!(parse_carrier("run_agents").is_none());
    assert!(parse_carrier("").is_none());
}

#[test]
fn result_to_json_reports_each_agent() {
    let result = RunAgentsResult::Launched {
        model_id: String::new(),
        harness_type: String::new(),
        agents: vec![
            RunAgentsAgentOutcome {
                name: "grid".to_string(),
                kind: RunAgentsAgentOutcomeKind::Launched {
                    agent_id: "abc".to_string(),
                },
            },
            RunAgentsAgentOutcome {
                name: "cli".to_string(),
                kind: RunAgentsAgentOutcomeKind::Failed {
                    error: "claude not installed".to_string(),
                },
            },
        ],
    };
    let json = result_to_json(&result);
    assert_eq!(json["status"], "launched");
    assert_eq!(json["agents"][0]["agent_id"], "abc");
    assert_eq!(json["agents"][1]["status"], "failed");
    assert_eq!(
        result_to_json(&RunAgentsResult::Cancelled)["status"],
        "cancelled"
    );
}
