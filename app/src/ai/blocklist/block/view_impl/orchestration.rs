//! Local run-agents confirmation and launch results, adapted from upstream orchestration UI.
use ai::agent::action::RunAgentsRequest;
use warpui::elements::{ChildView, Flex, ParentElement};
use warpui::{AppContext, Element};

use super::WithContentItemSpacing;
use super::output::{Props, blocked_action_header};
use crate::ai::agent::{AIAgentActionId, AIAgentActionResultType, AIAgentActionType};
use crate::ai::blocklist::action_model::AIActionStatus;
use crate::ai::blocklist::inline_action::inline_action_header::HeaderConfig;
use crate::ai::blocklist::inline_action::requested_action::RenderableAction;
use crate::ai::blocklist::inline_action::run_agents_card_view::result_text;

pub(super) fn render_run_agents(
    id: &AIAgentActionId,
    request: &RunAgentsRequest,
    props: Props,
    app: &AppContext,
) -> Box<dyn Element> {
    let status = props.action_model.as_ref(app).get_action_status(id);
    let blocked = status.as_ref().is_some_and(AIActionStatus::is_blocked);
    let title = if blocked {
        "Can I start additional agents for this task?"
    } else {
        "Additional agents"
    };
    let mut header = HeaderConfig::new(title, app);
    if blocked && !props.shared_session_status.is_viewer() {
        if let Some(buttons) = props.action_buttons.get(id) {
            header = blocked_action_header(
                id.clone(),
                title,
                buttons.run_button.clone(),
                buttons.cancel_button.clone(),
                props.action_model,
                props.model,
                app,
            );
        }
    }
    let request = props
        .action_model
        .as_ref(app)
        .get_pending_action_by_id(id)
        .and_then(|action| match &action.action {
            AIAgentActionType::RunAgents(request) => Some(request.as_ref()),
            _ => None,
        })
        .unwrap_or(request);
    let mut text = request.summary.clone();
    if !request.base_prompt.is_empty() {
        text.push_str(&format!("\n\n{}", request.base_prompt));
    }
    for agent in &request.agent_run_configs {
        let harness = if agent.harness_type.is_empty() {
            &request.harness_type
        } else {
            &agent.harness_type
        };
        let harness = if harness.is_empty() || harness == "oz" {
            "Warp"
        } else {
            harness
        };
        let harness_label = if blocked {
            format!(" ({harness})")
        } else {
            String::new()
        };
        text.push_str(&format!(
            "\n\n{}: {}{harness_label}\n{}",
            agent.name, agent.title, agent.prompt
        ));
    }
    if let Some(AIActionStatus::Finished(result)) = &status {
        if let AIAgentActionResultType::RunAgents(result) = &result.result {
            text.push_str(&format!("\n\n{}", result_text(result)));
        }
    } else if !blocked {
        text.push_str("\n\nStarting agents…");
    }
    let mut body = Flex::column();
    if blocked && !props.shared_session_status.is_viewer() {
        if let Some(dropdown) = props.run_agents_harnesses.get(id) {
            body.add_child(ChildView::new(dropdown).finish());
        }
    }
    body.add_child(RenderableAction::new(&text, app).render(app).finish());
    Flex::column()
        .with_child(header.render(app))
        .with_child(body.finish())
        .finish()
        .with_content_item_spacing()
        .finish()
}
