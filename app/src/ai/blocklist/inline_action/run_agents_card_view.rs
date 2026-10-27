//! Local subset of the upstream RunAgents card: installed harnesses and launch outcomes.
use ai::agent::action_result::{RunAgentsAgentOutcomeKind, RunAgentsResult};
use warp_core::features::FeatureFlag;

use crate::util::path::resolve_executable;

pub(crate) fn available_harnesses() -> Vec<(&'static str, &'static str)> {
    let mut harnesses = vec![("oz", "Warp")];
    if FeatureFlag::LocalClaudeCodexChildHarnesses.is_enabled() {
        harnesses.extend(
            [
                ("claude", "Claude Code"),
                ("codex", "Codex"),
                ("opencode", "OpenCode"),
            ]
            .into_iter()
            .filter(|(cli, _)| resolve_executable(cli).is_some()),
        );
    }
    harnesses
}

pub(crate) fn result_text(result: &RunAgentsResult) -> String {
    let mut text = result.to_string();
    if let RunAgentsResult::Launched { agents, .. } = result {
        for agent in agents {
            let outcome = match &agent.kind {
                RunAgentsAgentOutcomeKind::Launched { .. } => "Started".to_string(),
                RunAgentsAgentOutcomeKind::Failed { error } => format!("Failed: {error}"),
            };
            text.push_str(&format!("\n{}: {outcome}", agent.name));
        }
    }
    text
}

#[cfg(test)]
#[path = "run_agents_card_view_tests.rs"]
mod tests;
