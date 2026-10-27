use super::*;
use crate::ai::agent::{AIAgentActionId, AIAgentOutputMessageType};

fn carrier_message(server_message_data: &str) -> api::Message {
    api::Message {
        id: "msg-1".to_string(),
        task_id: "task-1".to_string(),
        server_message_data: server_message_data.to_string(),
        citations: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: "call-1".to_string(),
            tool: None,
        })),
        request_id: "req-1".to_string(),
        timestamp: None,
    }
}

fn convert(message: api::Message) -> MaybeAIAgentOutputMessage {
    let task_id = TaskId::new("task-1".to_string());
    message
        .to_client_output_message(ConversionParams {
            task_id: &task_id,
            current_todo_list: None,
            active_code_review: None,
        })
        .expect("conversion succeeds")
}

#[test]
fn run_agents_carrier_converts_to_run_agents_action() {
    let converted = convert(carrier_message(
        "run_agents\n{\"base_prompt\":\"b\",\"agents\":[{\"name\":\"a\",\"prompt\":\"p\"}]}",
    ));
    let MaybeAIAgentOutputMessage::Message(message) = converted else {
        panic!("expected an output message");
    };
    let AIAgentOutputMessageType::Action(action) = &message.message else {
        panic!("expected an action message");
    };
    assert_eq!(action.id, AIAgentActionId::from("call-1".to_string()));
    let AIAgentActionType::RunAgents(request) = &action.action else {
        panic!("expected RunAgents, got {:?}", action.action);
    };
    assert_eq!(request.base_prompt, "b");
    assert_eq!(request.agent_run_configs.len(), 1);
    assert_eq!(request.agent_run_configs[0].name, "a");
}

#[test]
fn other_carriers_have_no_client_representation() {
    for data in [
        "todowrite\n{\"todos\":[]}",
        "run_agents\n{\"agents\":[]}",
        "",
    ] {
        assert!(
            matches!(
                convert(carrier_message(data)),
                MaybeAIAgentOutputMessage::NoClientRepresentation
            ),
            "carrier {data:?} should have no client representation"
        );
    }
}
