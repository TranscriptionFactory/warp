use warpui::{App, EntityId};

use super::*;

#[test]
fn local_pills_include_root_siblings_and_descendants_but_not_remote_or_shared_children() {
    App::test((), |mut app| async move {
        let history = app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], &[]));
        let owner = EntityId::new();
        let (root, child, sibling, grandchild) = history.update(&mut app, |history, ctx| {
            let root = history.start_new_conversation(owner, false, false, ctx);
            let child =
                history.start_new_child_conversation(owner, "first".into(), root, None, ctx);
            let sibling =
                history.start_new_child_conversation(owner, "second".into(), root, None, ctx);
            let grandchild =
                history.start_new_child_conversation(owner, "nested".into(), child, None, ctx);
            let remote =
                history.start_new_child_conversation(owner, "remote".into(), root, None, ctx);
            history
                .conversation_mut(&remote)
                .unwrap()
                .mark_as_remote_child();
            let shared =
                history.start_new_child_conversation(owner, "shared".into(), root, None, ctx);
            history
                .conversation_mut(&shared)
                .unwrap()
                .set_is_viewing_shared_session(true);
            (root, child, sibling, grandchild)
        });
        history.read(&app, |history, _| {
            assert_eq!(root_conversation_id(grandchild, history), root);
            assert_eq!(
                conversation_ids(root, history),
                vec![root, child, sibling, grandchild]
            );
            assert_eq!(
                conversation_ids(child, history),
                conversation_ids(root, history)
            );
            assert_eq!(
                conversation_ids(grandchild, history),
                conversation_ids(root, history)
            );
        });
    });
}

#[test]
fn remote_only_conversation_has_no_local_pills() {
    App::test((), |mut app| async move {
        let history = app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], &[]));
        let root = history.update(&mut app, |history, ctx| {
            let owner = EntityId::new();
            let root = history.start_new_conversation(owner, false, false, ctx);
            let child =
                history.start_new_child_conversation(owner, "remote".into(), root, None, ctx);
            history
                .conversation_mut(&child)
                .unwrap()
                .mark_as_remote_child();
            root
        });
        history.read(&app, |history, _| {
            assert_eq!(conversation_ids(root, history), vec![root])
        });
    });
}
