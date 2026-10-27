use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use warp_cli::agent::Harness;
use warpui::platform::WindowStyle;
use warpui::{App, ModelHandle, SingletonEntity, ViewHandle};

use super::dispatch_start_agent_conversation;
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::blocklist::{
    BlocklistAIHistoryModel, StartAgentExecutionMode, StartAgentExecutor, StartAgentExecutorEvent,
    StartAgentOutcome, StartAgentRequest,
};
use crate::pane_group::{NewTerminalOptions, PaneGroup, PaneId, PanesLayout};
use crate::terminal::available_shells::AvailableShells;
use crate::test_util::terminal::initialize_app_for_terminal_view;
use crate::{GlobalResourceHandles, GlobalResourceHandlesProvider};

struct Fixture {
    pane_group: ViewHandle<PaneGroup>,
    parent_pane_id: PaneId,
    parent_conversation_id: AIConversationId,
    executor: ModelHandle<StartAgentExecutor>,
    requests: Arc<Mutex<Vec<StartAgentRequest>>>,
}

fn fixture(app: &mut App) -> Fixture {
    initialize_app_for_terminal_view(app);
    // Creating a hidden child pane resolves the shell to spawn.
    app.add_singleton_model(AvailableShells::new);
    app.add_singleton_model(remote_server::manager::RemoteServerManager::new);
    let global_resource_handles = GlobalResourceHandles::mock(app);
    let handles_for_provider = global_resource_handles.clone();
    app.add_singleton_model(|_| GlobalResourceHandlesProvider::new(handles_for_provider));
    let GlobalResourceHandles {
        model_event_sender,
        tips_completed,
        user_default_shell_unsupported_banner_model_handle,
        ..
    } = global_resource_handles;
    let (_, pane_group) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
        PaneGroup::new_with_panes_layout(
            tips_completed,
            user_default_shell_unsupported_banner_model_handle,
            PanesLayout::SingleTerminal(Box::<NewTerminalOptions>::default()),
            Arc::new(HashMap::new()),
            model_event_sender,
            ctx,
        )
    });
    let (parent_pane_id, parent_view_id) = pane_group.read(app, |group, ctx| {
        let pane_id = group.pane_id_from_index(0).expect("parent pane exists");
        let view = group
            .terminal_view_at_pane_index(0, ctx)
            .expect("parent terminal view exists");
        (pane_id, view.id())
    });
    let parent_conversation_id = BlocklistAIHistoryModel::handle(app).update(app, |model, ctx| {
        model.start_new_conversation(parent_view_id, false, false, ctx)
    });

    let executor = app.add_model(|_| StartAgentExecutor::new());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let requests_for_closure = requests.clone();
    app.update(|ctx| {
        ctx.subscribe_to_model(&executor, move |_, event, _| {
            let StartAgentExecutorEvent::CreateAgent(request) = event;
            requests_for_closure.lock().push(request.as_ref().clone());
        })
    });
    Fixture {
        pane_group,
        parent_pane_id,
        parent_conversation_id,
        executor,
        requests,
    }
}

/// Dispatches one child through the executor, then hands the emitted request to
/// the pane-group launcher, as the parent terminal view's re-emit does.
fn launch(
    app: &mut App,
    fixture: &Fixture,
    harness_type: Option<Harness>,
) -> async_channel::Receiver<StartAgentOutcome> {
    let parent_conversation_id = fixture.parent_conversation_id;
    let receiver = fixture.executor.update(app, |executor, ctx| {
        executor.dispatch(
            "child".to_string(),
            "Child".to_string(),
            "Survey the grid.".to_string(),
            StartAgentExecutionMode::Local {
                harness_type,
                model_id: None,
            },
            parent_conversation_id,
            None,
            ctx,
        )
    });
    let request = fixture.requests.lock().pop().expect("CreateAgent emitted");
    let executor = fixture.executor.clone();
    let parent_pane_id = fixture.parent_pane_id;
    fixture.pane_group.update(app, |group, ctx| {
        let terminal_pane_id = parent_pane_id
            .as_terminal_pane_id()
            .expect("parent is a terminal pane");
        dispatch_start_agent_conversation(
            group,
            parent_pane_id,
            terminal_pane_id,
            request,
            executor,
            ctx,
        );
    });
    receiver
}

#[test]
fn warp_child_gets_hidden_pane_with_local_task_id_and_starts() {
    App::test((), |mut app| async move {
        let fixture = fixture(&mut app);
        let receiver = launch(&mut app, &fixture, None);

        let (child_id, task_id) = BlocklistAIHistoryModel::handle(&app).read(&app, |model, _| {
            let children = model.child_conversations_of(fixture.parent_conversation_id);
            assert_eq!(children.len(), 1, "one child conversation");
            let child = children[0];
            assert_eq!(child.run_id(), child.task_id().map(|id| id.to_string()));
            (child.id(), child.task_id().expect("child task id is set"))
        });
        assert_eq!(
            receiver.try_recv(),
            Ok(StartAgentOutcome::Started {
                agent_id: task_id.to_string(),
            })
        );
        fixture.pane_group.read(&app, |group, _| {
            assert!(
                group.child_agent_panes.contains_key(&child_id),
                "child pane is tracked"
            );
            assert_eq!(
                group.visible_pane_ids(),
                vec![fixture.parent_pane_id],
                "child pane stays hidden"
            );
        });
    })
}

#[test]
fn cli_child_without_detected_shell_fails_without_a_pane() {
    App::test((), |mut app| async move {
        let fixture = fixture(&mut app);
        let receiver = launch(&mut app, &fixture, Some(Harness::Claude));

        let outcome = receiver.recv().await.expect("outcome reported");
        assert!(
            matches!(&outcome, StartAgentOutcome::Error(message) if message.contains("bash, zsh, or fish")),
            "unexpected outcome: {outcome:?}"
        );
        BlocklistAIHistoryModel::handle(&app).read(&app, |model, _| {
            assert!(
                model
                    .child_conversations_of(fixture.parent_conversation_id)
                    .is_empty()
            );
        });
        fixture.pane_group.read(&app, |group, _| {
            assert!(group.child_agent_panes.is_empty());
        });
    })
}
