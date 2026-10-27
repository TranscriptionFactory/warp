//! `run_agents` BYOP 工具 descriptor(fork-local orchestration M2)。
//!
//! 我们锁定的 `warp_multi_agent_api` 没有 `run_agents` proto,所以跟 todowrite /
//! webfetch 一样**不**映射到 protobuf `tool_call::Tool` variant。区别是它要走
//! executor(`RunAgentsExecutor`),不能在 chat_stream 里直接合成 result:
//!
//! 1. `chat_stream.rs` 按 name 拦截,只 emit 一条 carrier `ToolCall`(tool=None,
//!    `server_message_data = "run_agents\n<args json>"`),不带 ToolCallResult。
//! 2. `convert_from.rs` 遇到 tool=None 的 carrier 时调 [`parse_carrier`],
//!    命中则转成 `AIAgentActionType::RunAgents`,走正常 action 流程。
//! 3. executor 产出 `RunAgentsResult` 后,`serialize_action_result` 用
//!    [`result_to_json`] 序列化给上游模型(无 proto result variant)。
//!
//! 下一轮 history replay 沿用 carrier 通路:server_message_data 还原 fn_name + args。

use ai::agent::action::{RunAgentsAgentRunConfig, RunAgentsRequest};
use ai::agent::action_result::{RunAgentsAgentOutcomeKind, RunAgentsResult};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use serde_json::{Value, json};
use warp_multi_agent_api as api;

use super::OpenAiTool;
use warp_core::features::FeatureFlag;

pub const TOOL_NAME: &str = "run_agents";

#[derive(Debug, Deserialize)]
struct Args {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    base_prompt: String,
    #[serde(default)]
    model_id: String,
    #[serde(default)]
    harness_type: String,
    agents: Vec<AgentArg>,
}

#[derive(Debug, Deserialize)]
struct AgentArg {
    name: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    model_id: String,
    #[serde(default)]
    harness_type: String,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "description": "One-line summary of what the child agents will do, shown to the user."
            },
            "base_prompt": {
                "type": "string",
                "description": "Shared instructions prepended to every child's prompt."
            },
            "model_id": {
                "type": "string",
                "description": "Optional run-wide model override. Empty inherits the current model."
            },
            "harness_type": {
                "type": "string",
                "description": "Run-wide harness: empty or \"oz\" for a built-in agent, or \"claude\", \"opencode\", \"codex\" for a local CLI agent."
            },
            "agents": {
                "type": "array",
                "minItems": 1,
                "description": "One entry per child agent.",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Short unique handle for this child."
                        },
                        "prompt": {
                            "type": "string",
                            "description": "This child's own task, appended to base_prompt."
                        },
                        "title": {
                            "type": "string",
                            "description": "Optional display title."
                        },
                        "model_id": {
                            "type": "string",
                            "description": "Optional per-child model override."
                        },
                        "harness_type": {
                            "type": "string",
                            "description": "Optional per-child harness override (same values as the run-wide field)."
                        }
                    },
                    "required": ["name", "prompt"]
                }
            }
        },
        "required": ["agents"],
        "additionalProperties": false
    })
}

fn from_args(_args: &str) -> Result<api::message::tool_call::Tool> {
    Err(anyhow!(
        "run_agents is intercepted by chat_stream and converted to a fork-local action; \
         from_args should never be called"
    ))
}

fn msg_result_to_json(_result: &api::message::tool_call_result::Result) -> Option<Value> {
    None
}

pub static RUN_AGENTS: OpenAiTool = OpenAiTool {
    name: TOOL_NAME,
    description: include_str!("../prompts/tool_descriptions/run_agents.md"),
    parameters,
    from_args,
    result_to_json: msg_result_to_json,
};

/// Whether `run_agents` is offered to the model this session.
pub fn is_enabled() -> bool {
    FeatureFlag::MultiLevelOrchestration.is_enabled()
}

/// Parses the model's `run_agents` arguments into a [`RunAgentsRequest`].
pub fn parse_args(args: &str) -> Result<RunAgentsRequest> {
    let parsed: Args =
        serde_json::from_str(args).map_err(|e| anyhow!("run_agents args parse error: {e}"))?;
    if parsed.agents.is_empty() {
        return Err(anyhow!(
            "run_agents requires at least one entry in `agents`"
        ));
    }
    Ok(RunAgentsRequest {
        summary: parsed.summary,
        base_prompt: parsed.base_prompt,
        model_id: parsed.model_id,
        harness_type: parsed.harness_type,
        agent_run_configs: parsed
            .agents
            .into_iter()
            .map(|agent| RunAgentsAgentRunConfig {
                name: agent.name,
                prompt: agent.prompt,
                title: agent.title,
                model_id: agent.model_id,
                harness_type: agent.harness_type,
            })
            .collect(),
    })
}

/// Recognizes a `run_agents` carrier's `server_message_data`
/// (`"run_agents\n<args json>"`). Returns `None` for any other carrier.
pub fn parse_carrier(server_message_data: &str) -> Option<Result<RunAgentsRequest>> {
    let (fn_name, args) = server_message_data.split_once('\n')?;
    (fn_name == TOOL_NAME).then(|| parse_args(args))
}

/// Serializes a [`RunAgentsResult`] for the upstream model (role=tool content).
pub fn result_to_json(result: &RunAgentsResult) -> Value {
    match result {
        RunAgentsResult::Launched {
            model_id,
            harness_type,
            agents,
        } => json!({
            "status": "launched",
            "model_id": model_id,
            "harness_type": harness_type,
            "agents": agents
                .iter()
                .map(|agent| match &agent.kind {
                    RunAgentsAgentOutcomeKind::Launched { agent_id } => json!({
                        "name": agent.name,
                        "status": "launched",
                        "agent_id": agent_id,
                    }),
                    RunAgentsAgentOutcomeKind::Failed { error } => json!({
                        "name": agent.name,
                        "status": "failed",
                        "error": error,
                    }),
                })
                .collect::<Vec<_>>(),
        }),
        RunAgentsResult::Denied { reason } => json!({ "status": "denied", "reason": reason }),
        RunAgentsResult::Failure { error } => json!({ "status": "failed", "error": error }),
        RunAgentsResult::Cancelled => json!({ "status": "cancelled" }),
    }
}

pub fn invalid_arguments_result_to_json(detail: String, received_args: &str) -> Value {
    json!({
        "error": "invalid_arguments",
        "detail": detail,
        "tool": TOOL_NAME,
        "received_args": received_args,
        "hint": "Expected { base_prompt?: string, harness_type?: string, agents: [{ name: string, prompt: string, harness_type?: string }] }.",
    })
}

#[cfg(test)]
#[path = "run_agents_tests.rs"]
mod tests;
