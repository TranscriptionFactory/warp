//! Rendering helpers and styles shared by AI settings pages.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use settings::Setting;
use warp_core::ui::theme::color::internal_colors;
use warpui::elements::{
    ChildView, Container, Element, Fill, Flex, MouseStateHandle, ParentElement,
};
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{Action, AppContext, SingletonEntity, ViewHandle};

use super::SettingsAction;
use super::settings_page::{
    HEADER_PADDING, LocalOnlyIconState, ToggleState, build_toggle_element, render_body_item_label,
};
use crate::ai::blocklist::agent_view::agent_input_footer::editor::AgentToolbarInlineEditor;
use crate::appearance::Appearance;

pub(super) fn render_ai_setting_toggle<S: Setting>(
    label: impl Into<String>,
    action: impl Action + Clone,
    is_setting_enabled: bool,
    is_setting_toggleable: bool,
    switch_state: SwitchStateHandle,
    tooltip_states: &RefCell<HashMap<String, MouseStateHandle>>,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    build_toggle_element(
        setting_label_element::<S>(label, is_setting_toggleable, tooltip_states, app),
        render_ai_feature_switch(
            switch_state,
            is_setting_enabled,
            is_setting_toggleable,
            action,
            app,
        ),
        appearance,
        None,
    )
}

pub(super) fn render_ai_setting_label<S: Setting>(
    label: impl Into<String>,
    is_setting_toggleable: bool,
    tooltip_states: &RefCell<HashMap<String, MouseStateHandle>>,
    app: &AppContext,
) -> Box<dyn Element> {
    Container::new(setting_label_element::<S>(
        label,
        is_setting_toggleable,
        tooltip_states,
        app,
    ))
    .with_margin_bottom(HEADER_PADDING)
    .finish()
}

fn setting_label_element<S: Setting>(
    label: impl Into<String>,
    is_setting_toggleable: bool,
    tooltip_states: &RefCell<HashMap<String, MouseStateHandle>>,
    app: &AppContext,
) -> Box<dyn Element> {
    render_body_item_label::<SettingsAction>(
        label.into(),
        Some(styles::header_font_color(is_setting_toggleable, app)),
        None,
        LocalOnlyIconState::for_setting(
            S::storage_key(),
            S::sync_to_cloud(),
            &mut tooltip_states.borrow_mut(),
            app,
        ),
        ToggleState::Enabled,
        Appearance::as_ref(app),
    )
}

pub(super) fn render_ai_setting_description(
    description: impl Into<Cow<'static, str>>,
    is_setting_toggleable: bool,
    app: &AppContext,
) -> Box<dyn Element> {
    let default_font_size = Appearance::as_ref(app).ui_font_size();
    render_ai_setting_description_with_font_size(
        description,
        default_font_size,
        is_setting_toggleable,
        app,
    )
}

pub(super) fn render_ai_setting_description_with_font_size(
    description: impl Into<Cow<'static, str>>,
    font_size: f32,
    is_setting_toggleable: bool,
    app: &AppContext,
) -> Box<dyn Element> {
    let ui_builder = Appearance::as_ref(app).ui_builder();
    ui_builder
        .paragraph(description)
        .with_style(UiComponentStyles {
            font_size: Some(font_size),
            font_color: Some(styles::description_font_color(is_setting_toggleable, app).into()),
            margin: Some(
                Coords::default()
                    .top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
                    .bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
                    .right(styles::TOGGLE_WIDTH_MARGIN),
            ),
            ..Default::default()
        })
        .build()
        .finish()
}

pub(super) fn render_toolbar_layout_editor(
    editor: &ViewHandle<AgentToolbarInlineEditor>,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let label = Container::new(
        appearance
            .ui_builder()
            .span(crate::t!("settings-ai-toolbar-layout"))
            .with_style(UiComponentStyles {
                font_size: Some(appearance.ui_font_body()),
                ..Default::default()
            })
            .build()
            .finish(),
    )
    .with_margin_bottom(4.)
    .finish();
    let editor = Container::new(ChildView::new(editor).finish())
        .with_margin_bottom(16.)
        .finish();

    Flex::column().with_child(label).with_child(editor).finish()
}

pub(super) fn render_ai_feature_switch(
    state_handle: SwitchStateHandle,
    is_setting_enabled: bool,
    is_setting_toggleable: bool,
    toggle_action: impl Action + Clone,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    appearance
        .ui_builder()
        .switch(state_handle)
        .check(is_setting_enabled)
        .with_disabled(!is_setting_toggleable)
        .with_disabled_styles(UiComponentStyles {
            background: Some(Fill::Solid(internal_colors::neutral_4(appearance.theme()))),
            foreground: Some(Fill::Solid(internal_colors::neutral_5(appearance.theme()))),
            ..Default::default()
        })
        .build()
        .on_click(move |ctx, _, _| {
            if is_setting_toggleable {
                ctx.dispatch_typed_action(toggle_action.clone());
            }
        })
        .finish()
}

pub(super) mod styles {
    use warp_core::ui::{appearance::Appearance, theme::Fill};
    use warpui::{AppContext, SingletonEntity};

    pub const DESCRIPTION_NEGATIVE_MARGIN_OFFSET: f32 = -12.;
    pub const DESCRIPTION_MARGIN_BOTTOM: f32 = 12.;
    pub const TOGGLE_WIDTH_MARGIN: f32 = 48.;

    pub fn header_font_color(is_enabled_setting: bool, app: &AppContext) -> Fill {
        let appearance = Appearance::as_ref(app);
        if is_enabled_setting {
            appearance
                .theme()
                .main_text_color(appearance.theme().surface_2())
        } else {
            appearance.theme().disabled_ui_text_color()
        }
    }

    pub fn description_font_color(is_enabled_setting: bool, app: &AppContext) -> Fill {
        let appearance = Appearance::as_ref(app);
        if is_enabled_setting {
            appearance
                .theme()
                .sub_text_color(appearance.theme().surface_1())
        } else {
            appearance.theme().disabled_ui_text_color()
        }
    }
}
