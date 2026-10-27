//! Tests for [`LocalChildFeed`]: conversations created through the history
//! model drive the streamer's local trackers and pill-bar broadcasts.

use std::sync::Arc;

use parking_lot::Mutex;
use warp_core::features::FeatureFlag;
use warpui::{App, EntityId, ModelHandle};

use super::*;
use crate::ai::blocklist::orchestration_event_streamer::OrchestrationEventStreamerEvent;
use crate::server::server_api::ServerApiProvider;
use crate::server::server_api::ai::{AIClient, MockAIClient};

const PARENT_RUN_ID: &str = "11111111-1111-1111-1111-111111111111";
const CHILD_RUN_ID: &str = "22222222-2222-2222-2222-222222222222";

fn task_id(run_id: &str) -> AmbientAgentTaskId {
    run_id.parse().expect("hardcoded task id parses")
}

/// A pill-bar broadcast, reduced to what the assertions compare.
#[derive(Debug, PartialEq)]
enum Broadcast {
    Spawned(AmbientAgentTaskId, String),
    Status(AmbientAgentTaskId, String, ConversationStatus),
}

struct Harness {
    history: ModelHandle<BlocklistAIHistoryModel>,
    streamer: ModelHandle<OrchestrationEventStreamer>,
    broadcasts: Arc<Mutex<Vec<Broadcast>>>,
    view_id: EntityId,
}

fn setup(app: &mut App) -> Harness {
    let history = app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], &[]));
    let ai_client: Arc<dyn AIClient> = Arc::new(MockAIClient::new());
    let server_api = ServerApiProvider::new_for_test().get();
    let streamer = app.add_singleton_model(|ctx| {
        OrchestrationEventStreamer::new_with_clients_for_test(ai_client, server_api, ctx)
    });
    let broadcasts = Arc::new(Mutex::new(Vec::new()));
    let sink = broadcasts.clone();
    app.update(|ctx| {
        ctx.subscribe_to_model(&streamer, move |_, event, _| match event {
            OrchestrationEventStreamerEvent::ChildSpawned {
                parent_task_id,
                run_id,
            } => sink
                .lock()
                .push(Broadcast::Spawned(*parent_task_id, run_id.clone())),
            OrchestrationEventStreamerEvent::ChildStatusChanged {
                parent_task_id,
                run_id,
                status,
            } => sink.lock().push(Broadcast::Status(
                *parent_task_id,
                run_id.clone(),
                status.clone(),
            )),
            _ => {}
        })
    });
    Harness {
        history,
        streamer,
        broadcasts,
        view_id: EntityId::new(),
    }
}

impl Harness {
    fn start_conversation(&self, app: &mut App, run_id: Option<&str>) -> AIConversationId {
        let view_id = self.view_id;
        self.history.update(app, |history, ctx| {
            let id = history.start_new_conversation(view_id, false, false, ctx);
            if let Some(run_id) = run_id {
                history
                    .conversation_mut(&id)
                    .expect("just created")
                    .set_task_id(task_id(run_id));
            }
            id
        })
    }

    /// Mirrors orch3's launch: `start_new_child_conversation`, then the task
    /// id, in one update. `remote` mirrors the SSE placeholder path instead.
    fn start_child(
        &self,
        app: &mut App,
        parent: AIConversationId,
        run_id: Option<&str>,
        remote: bool,
    ) -> AIConversationId {
        let view_id = self.view_id;
        self.history.update(app, |history, ctx| {
            let id =
                history.start_new_child_conversation(view_id, "child".into(), parent, None, ctx);
            if remote {
                // The flag only; the history wrapper also persists, which
                // needs singletons this test does not install.
                history
                    .conversation_mut(&id)
                    .expect("just created")
                    .mark_as_remote_child();
            }
            if let Some(run_id) = run_id {
                history
                    .conversation_mut(&id)
                    .expect("just created")
                    .set_task_id(task_id(run_id));
            }
            id
        })
    }

    fn set_status(&self, app: &mut App, id: AIConversationId, status: ConversationStatus) {
        let view_id = self.view_id;
        self.history.update(app, |history, ctx| {
            history.update_conversation_status(view_id, id, status, ctx);
        });
    }

    fn take_broadcasts(&self) -> Vec<Broadcast> {
        std::mem::take(&mut *self.broadcasts.lock())
    }
}

#[test]
fn local_child_registers_in_band_and_broadcasts_status_changes() {
    App::test((), |mut app| async move {
        let _multi = FeatureFlag::MultiLevelOrchestration.override_enabled(true);
        let _unified = FeatureFlag::OrchestrationUnifiedStack.override_enabled(true);
        let h = setup(&mut app);
        // BYOP parent: no task id of its own.
        let parent = h.start_conversation(&mut app, None);
        // `StartedNewConversation` is delivered after the launch update, so
        // the child is already linked and registers on creation.
        let child = h.start_child(&mut app, parent, Some(CHILD_RUN_ID), false);
        let initial = h
            .history
            .read(&app, |history, _| {
                history.conversation(&child).map(|c| c.status().clone())
            })
            .expect("child exists");
        let parent_task_id = h
            .streamer
            .read(&app, |s, _| s.local_child_parent_task_id(parent))
            .expect("parent gets a minted task id on its first child");
        assert_eq!(
            h.take_broadcasts(),
            vec![
                Broadcast::Spawned(parent_task_id, CHILD_RUN_ID.into()),
                Broadcast::Status(parent_task_id, CHILD_RUN_ID.into(), initial.clone()),
            ]
        );
        h.streamer.read(&app, |s, _| {
            let tracker = s.local_children.trackers.get(&parent).expect("tracker");
            assert!(tracker.is_in_band_child(task_id(CHILD_RUN_ID)));
        });

        // A repeated status is not rebroadcast; a new one is.
        h.set_status(&mut app, child, initial);
        h.set_status(&mut app, child, ConversationStatus::Success);
        h.set_status(&mut app, child, ConversationStatus::Success);
        assert_eq!(
            h.take_broadcasts(),
            vec![Broadcast::Status(
                parent_task_id,
                CHILD_RUN_ID.into(),
                ConversationStatus::Success
            )]
        );
    });
}

#[test]
fn local_child_broadcasts_carry_the_parents_own_task_id() {
    App::test((), |mut app| async move {
        let _multi = FeatureFlag::MultiLevelOrchestration.override_enabled(true);
        let _unified = FeatureFlag::OrchestrationUnifiedStack.override_enabled(true);
        let h = setup(&mut app);
        let parent = h.start_conversation(&mut app, Some(PARENT_RUN_ID));
        h.start_child(&mut app, parent, Some(CHILD_RUN_ID), false);
        assert_eq!(
            h.take_broadcasts().first(),
            Some(&Broadcast::Spawned(
                task_id(PARENT_RUN_ID),
                CHILD_RUN_ID.into()
            ))
        );
    });
}

#[test]
fn conversations_without_parent_or_task_id_or_that_are_remote_are_ignored() {
    App::test((), |mut app| async move {
        let _multi = FeatureFlag::MultiLevelOrchestration.override_enabled(true);
        let _unified = FeatureFlag::OrchestrationUnifiedStack.override_enabled(true);
        let h = setup(&mut app);
        let parent = h.start_conversation(&mut app, None);
        // No parent link.
        let orphan = h.start_conversation(&mut app, Some(CHILD_RUN_ID));
        // Parent link but no task id.
        let unlaunched = h.start_child(&mut app, parent, None, false);
        // Remote placeholder: the SSE tracker owns it.
        let remote = h.start_child(&mut app, parent, Some(PARENT_RUN_ID), true);
        for id in [orphan, unlaunched, remote] {
            h.set_status(&mut app, id, ConversationStatus::InProgress);
        }
        assert!(h.take_broadcasts().is_empty());
        h.streamer
            .read(&app, |s, _| assert!(s.local_children.trackers.is_empty()));
    });
}
