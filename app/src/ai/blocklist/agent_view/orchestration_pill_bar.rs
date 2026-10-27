//! Local orchestration navigation, hosted by the existing child-status surface.
use std::collections::{HashMap, HashSet};

use warpui::elements::new_scrollable::{NewScrollable, ScrollableAppearance, SingleAxisConfig};
use warpui::elements::{
    Border, ClippedScrollStateHandle, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
    Empty, Flex, Hoverable, MouseStateHandle, ParentElement, Radius, ScrollbarWidth, Shrinkable,
    Text,
};
use warpui::platform::Cursor;
use warpui::text_layout::ClipConfig;
use warpui::{AppContext, Element, SingletonEntity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::blocklist::BlocklistAIHistoryModel;
use crate::appearance::Appearance;
use crate::features::FeatureFlag;
use crate::terminal::view::TerminalAction;
use crate::view_components::action_button::{ActionButtonTheme, SecondaryTheme};
use crate::workspace::WorkspaceAction;

pub(super) fn enabled() -> bool {
    FeatureFlag::MultiLevelOrchestration.is_enabled()
        && FeatureFlag::OrchestrationUnifiedStack.is_enabled()
}

pub(super) fn root_conversation_id(
    active: AIConversationId,
    history: &BlocklistAIHistoryModel,
) -> AIConversationId {
    let mut root = active;
    let mut visited = HashSet::from([root]);
    while let Some(parent) = history
        .conversation(&root)
        .and_then(|conversation| conversation.parent_conversation_id())
    {
        if history.conversation(&parent).is_none() || !visited.insert(parent) {
            break;
        }
        root = parent;
    }
    root
}

pub(super) fn conversation_ids(
    active: AIConversationId,
    history: &BlocklistAIHistoryModel,
) -> Vec<AIConversationId> {
    let root = root_conversation_id(active, history);
    let mut ids = vec![root];
    let mut visited = HashSet::from([root]);
    let mut index = 0;
    while index < ids.len() {
        let mut children = history.child_conversations_of(ids[index]);
        children.sort_by_key(|child| child.first_exchange().map(|exchange| exchange.start_time));
        for child in children {
            if !child.is_remote_child()
                && !child.is_viewing_shared_session()
                && visited.insert(child.id())
            {
                ids.push(child.id());
            }
        }
        index += 1;
    }
    ids
}

pub(super) fn render(
    active: AIConversationId,
    mouse_states: &HashMap<AIConversationId, MouseStateHandle>,
    scroll_state: ClippedScrollStateHandle,
    app: &AppContext,
) -> Box<dyn Element> {
    let history = BlocklistAIHistoryModel::as_ref(app);
    let ids = conversation_ids(active, history);
    if ids.len() < 2 {
        return Empty::new().finish();
    }
    let root = ids[0];
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let mut row = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);
    for id in ids {
        let Some(conversation) = history.conversation(&id) else {
            continue;
        };
        let Some(mouse_state) = mouse_states.get(&id) else {
            continue;
        };
        let label = conversation
            .agent_name()
            .map(str::to_owned)
            .or_else(|| conversation.title())
            .unwrap_or_else(|| crate::t!("child-agent-default-name"));
        let (icon, color) = conversation.status().status_icon_and_color(theme);
        let owner = history.terminal_view_id_for_conversation(&id);
        let pill = Hoverable::new(mouse_state.clone(), move |hover| {
            let button_theme = SecondaryTheme;
            let background =
                button_theme.background(hover.is_hovered() || id == active, appearance);
            let content = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    ConstrainedBox::new(icon.to_warpui_icon(color.into()).finish())
                        .with_width(14.)
                        .with_height(14.)
                        .finish(),
                )
                .with_child(
                    Shrinkable::new(
                        1.,
                        Container::new(
                            Text::new(
                                label.clone(),
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .soft_wrap(false)
                            .with_clip(ClipConfig::ellipsis())
                            .with_color(button_theme.text_color(
                                hover.is_hovered(),
                                background,
                                appearance,
                            ))
                            .finish(),
                        )
                        .with_margin_left(4.)
                        .finish(),
                    )
                    .finish(),
                )
                .finish();
            let mut pill =
                Container::new(ConstrainedBox::new(content).with_max_width(150.).finish())
                    .with_horizontal_padding(8.)
                    .with_vertical_padding(4.)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(12.)));
            if let Some(background) = background {
                pill = pill.with_background(background);
            }
            if let Some(border) = button_theme.border(appearance) {
                pill = pill.with_border(Border::all(1.).with_border_color(border));
            }
            pill.finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            if id == root {
                if let Some(terminal_view_id) = owner {
                    ctx.dispatch_typed_action(WorkspaceAction::FocusTerminalViewInWorkspace {
                        terminal_view_id,
                    });
                }
            } else {
                ctx.dispatch_typed_action(TerminalAction::RevealChildAgent {
                    conversation_id: id,
                });
            }
        })
        .finish();
        row.add_child(Container::new(pill).with_margin_right(4.).finish());
    }
    NewScrollable::horizontal(
        SingleAxisConfig::Clipped {
            handle: scroll_state,
            child: row.finish(),
        },
        theme.nonactive_ui_detail().into(),
        theme.active_ui_detail().into(),
        warpui::elements::Fill::None,
    )
    .with_horizontal_scrollbar(ScrollableAppearance::new(ScrollbarWidth::Custom(4.), false))
    .with_propagate_mousewheel_if_not_handled(true)
    .finish()
}

#[cfg(test)]
#[path = "orchestration_pill_bar_tests.rs"]
mod tests;
