use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use warp_core::features::FeatureFlag;
use warpui::elements::{
    Container, Element, Flex, FormattedTextElement, HighlightedHyperlink, MouseStateHandle,
    ParentElement,
};
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{AppContext, SingletonEntity};

use super::ai_page::{AISettingsPageAction, AISettingsPageView};
use super::ai_shared::{render_ai_setting_description, render_ai_setting_toggle, styles};
use super::settings_page::{
    HEADER_PADDING, SettingsWidget, build_sub_header, render_full_pane_width_ai_button,
};
use crate::appearance::Appearance;
use crate::settings::{AISettings, MemoryEnabled, RuleSuggestionsEnabled};

#[derive(Default)]
pub(super) struct AIFactWidget {
    rules_toggle: SwitchStateHandle,
    rules_link_index: HighlightedHyperlink,
    manage_rules_button: MouseStateHandle,
    rule_suggestions_toggle: SwitchStateHandle,
}

impl AIFactWidget {
    fn render_rules_toggle(
        &self,
        view: &AISettingsPageView,
        ai_settings: &AISettings,
        appearance: &Appearance,
        app: &warpui::AppContext,
    ) -> Box<dyn Element> {
        let toggle = render_ai_setting_toggle::<MemoryEnabled>(
            crate::t!("settings-ai-rules-label"),
            AISettingsPageAction::ToggleRules,
            *ai_settings.memory_enabled,
            ai_settings.is_any_ai_enabled(app),
            self.rules_toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        );

        let rules_description = vec![
            FormattedTextFragment::plain_text(format!(
                "{} ",
                crate::t!("settings-ai-rules-description")
            )),
            FormattedTextFragment::hyperlink(crate::t!("settings-ai-learn-more"), ""),
        ];
        let description = Container::new(
            FormattedTextElement::new(
                FormattedText::new([FormattedTextLine::Line(rules_description)]),
                appearance.ui_font_body(),
                appearance.ui_font_family(),
                appearance.ui_font_family(),
                styles::description_font_color(ai_settings.is_any_ai_enabled(app), app).into(),
                self.rules_link_index.clone(),
            )
            .with_heading_to_font_size_multipliers(
                appearance.heading_font_size_multipliers().clone(),
            )
            .with_hyperlink_font_color(appearance.theme().accent().into_solid())
            .register_default_click_handlers(|url, ctx, _| {
                ctx.dispatch_typed_action(AISettingsPageAction::HyperlinkClick(url));
            })
            .finish(),
        )
        .with_margin_top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
        .with_margin_bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
        .with_margin_right(styles::TOGGLE_WIDTH_MARGIN)
        .finish();

        Flex::column()
            .with_child(toggle)
            .with_child(description)
            .finish()
    }

    fn render_rule_suggestions_toggle(
        &self,
        view: &AISettingsPageView,
        ai_settings: &AISettings,
        app: &warpui::AppContext,
    ) -> Box<dyn Element> {
        let toggle = render_ai_setting_toggle::<RuleSuggestionsEnabled>(
            crate::t!("settings-ai-suggested-rules-label"),
            AISettingsPageAction::ToggleRuleSuggestions,
            *ai_settings.rule_suggestions_enabled_internal,
            ai_settings.is_any_ai_enabled(app),
            self.rule_suggestions_toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        );

        let description = render_ai_setting_description(
            crate::t!("settings-ai-suggested-rules-description"),
            ai_settings.is_any_ai_enabled(app),
            app,
        );

        Flex::column()
            .with_child(toggle)
            .with_child(description)
            .finish()
    }
}

impl SettingsWidget for AIFactWidget {
    type View = AISettingsPageView;

    fn search_terms(&self) -> &str {
        "agent oz ai a.i. knowledge fact memory memories rules zap drive context workflows notebooks environment variables"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::AIRules.is_enabled()
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);

        let header = build_sub_header(
            appearance,
            crate::t!("settings-ai-knowledge-section"),
            Some(styles::header_font_color(is_any_ai_enabled, app)),
        )
        .with_margin_bottom(HEADER_PADDING)
        .finish();

        let button = render_full_pane_width_ai_button(
            &crate::t!("settings-ai-manage-rules"),
            is_any_ai_enabled,
            self.manage_rules_button.clone(),
            AISettingsPageAction::OpenAIFactCollection,
            appearance,
        );

        let mut column = Flex::column()
            .with_child(header)
            .with_child(self.render_rules_toggle(view, ai_settings, appearance, app));

        if FeatureFlag::SuggestedRules.is_enabled() {
            column.add_child(self.render_rule_suggestions_toggle(view, ai_settings, app));
        }

        // 去中心化分支:不再渲染 "Zap Drive as agent context" 开关。
        let _ = self;
        let _ = view;
        column.with_child(button).finish()
    }
}
