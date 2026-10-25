//! How tooltips look: the theme tokens they are coloured through, the metrics
//! and timings they are laid out and opened with, and the example values a game
//! overrides.

use std::time::Duration;

use bevy::{
    feathers::theme::{SemanticToken, ThemeToken, UiTheme},
    prelude::*,
};

/// Panel background. Define `tooltip.bg` in `UiTheme` to override it.
pub const TOOLTIP_BG: ThemeToken = ThemeToken::new_static("tooltip.bg");
/// Panel border. Define `tooltip.border` in `UiTheme` to override it.
pub const TOOLTIP_BORDER: ThemeToken = ThemeToken::new_static("tooltip.border");
/// Title text. Define `tooltip.title` in `UiTheme` to override it.
pub const TOOLTIP_TITLE: ThemeToken = ThemeToken::new_static("tooltip.title");
/// Body text. Define `tooltip.text` in `UiTheme` to override it.
pub const TOOLTIP_TEXT: ThemeToken = ThemeToken::new_static("tooltip.text");
/// A reference inside a body. Define `tooltip.ref` in `UiTheme` to override it.
pub const TOOLTIP_REF: ThemeToken = ThemeToken::new_static("tooltip.ref");
/// A reference under the pointer. Define `tooltip.ref.hover` in `UiTheme` to override it.
pub const TOOLTIP_REF_HOVER: ThemeToken = ThemeToken::new_static("tooltip.ref.hover");

const EXAMPLE_COLORS: [(ThemeToken, &str, Color); 6] = [
    (
        TOOLTIP_BG,
        "tooltip.bg",
        Color::oklcha(0.22, 0.02, 260.0, 0.96),
    ),
    (
        TOOLTIP_BORDER,
        "tooltip.border",
        Color::oklcha(0.62, 0.12, 185.0, 1.0),
    ),
    (
        TOOLTIP_TITLE,
        "tooltip.title",
        Color::oklcha(0.96, 0.03, 90.0, 1.0),
    ),
    (
        TOOLTIP_TEXT,
        "tooltip.text",
        Color::oklcha(0.86, 0.01, 260.0, 1.0),
    ),
    (
        TOOLTIP_REF,
        "tooltip.ref",
        Color::oklcha(0.76, 0.14, 230.0, 1.0),
    ),
    (
        TOOLTIP_REF_HOVER,
        "tooltip.ref.hover",
        Color::oklcha(0.9, 0.14, 230.0, 1.0),
    ),
];

/// Metrics and timings for every tooltip. The plugin inserts the example values
/// below unless the game inserted its own first; changes apply to layers opened
/// afterwards.
#[derive(Resource, Clone, Debug)]
pub struct TooltipStyle {
    /// How long the pointer rests on an anchor before its tooltip opens.
    pub hover_delay: Duration,
    /// How long an unpinned tooltip survives once the pointer has left its anchor.
    pub close_delay: Duration,
    /// Width at which body text wraps.
    pub max_width: Val,
    pub padding: Val,
    pub border: Val,
    /// Space between a layer's title and its body.
    pub row_gap: Val,
    /// Space between anchor and layer, in logical pixels. The pointer crossing it
    /// counts as leaving the anchor, so an unpinned tooltip closes unless the
    /// pointer reaches the layer within `close_delay`.
    pub popover_gap: f32,
    /// Closest a layer may be placed to the window edge, in logical pixels.
    pub window_margin: f32,
    pub title_font_size: f32,
    pub body_font_size: f32,
    /// Most layers one chain may hold; a reference click past it opens nothing.
    pub max_depth: usize,
    /// `GlobalZIndex` of a chain's first layer; each deeper layer sits one above.
    pub base_z: i32,
}

impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            hover_delay: Duration::from_millis(400),
            close_delay: Duration::from_millis(200),
            max_width: px(320),
            padding: px(8),
            border: px(2),
            row_gap: px(4),
            popover_gap: 0.0,
            window_margin: 4.0,
            title_font_size: 16.0,
            body_font_size: 14.0,
            max_depth: 8,
            base_z: 1000,
        }
    }
}

pub(crate) fn add_missing_tooltip_tokens(mut theme: ResMut<UiTheme>) {
    for (token, semantic_name, color) in EXAMPLE_COLORS {
        let assigned = theme.0.token_assignments.get(&token).cloned();
        if assigned
            .as_ref()
            .is_some_and(|semantic| theme.0.semantic_base.contains_key(semantic))
        {
            continue;
        }
        let semantic = assigned.unwrap_or_else(|| SemanticToken::new_static(semantic_name));
        let props = &mut theme.0;
        props.token_assignments.insert(token, semantic.clone());
        props.semantic_base.insert(semantic, color);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn run(theme: UiTheme) -> UiTheme {
        let mut world = World::new();
        world.insert_resource(theme);
        world.run_system_once(add_missing_tooltip_tokens).unwrap();
        world.remove_resource::<UiTheme>().unwrap()
    }

    // Pins that a colour the game defined is left alone.
    #[test]
    fn game_defined_token_survives() {
        let mut theme = UiTheme::default();
        let semantic = SemanticToken::new_static("game.panel");
        theme
            .0
            .token_assignments
            .insert(TOOLTIP_BG, semantic.clone());
        theme.0.semantic_base.insert(semantic.clone(), Color::BLACK);
        let theme = run(theme);
        assert_eq!(theme.0.token_assignments.get(&TOOLTIP_BG), Some(&semantic));
        assert_eq!(theme.0.semantic_base.get(&semantic), Some(&Color::BLACK));
    }

    // Pins that every undefined tooltip token is filled, so nothing renders as the fuchsia fallback.
    #[test]
    fn undefined_tokens_are_filled() {
        let theme = run(UiTheme::default());
        for (token, ..) in EXAMPLE_COLORS {
            assert_ne!(
                theme.0.color(&token),
                Color::from(bevy::color::palettes::basic::FUCHSIA)
            );
        }
    }

    // Pins that a token assigned to a semantic with no colour gets a colour, not a second assignment.
    #[test]
    fn assigned_but_uncoloured_token_gets_a_colour() {
        let mut theme = UiTheme::default();
        let semantic = SemanticToken::new_static("game.ref");
        theme
            .0
            .token_assignments
            .insert(TOOLTIP_REF, semantic.clone());
        let theme = run(theme);
        assert_eq!(theme.0.token_assignments.get(&TOOLTIP_REF), Some(&semantic));
        assert!(theme.0.semantic_base.contains_key(&semantic));
    }

    // Pins that tokens outside the plugin's six are never touched.
    #[test]
    fn foreign_tokens_untouched() {
        let mut theme = UiTheme::default();
        let foreign = ThemeToken::new_static("game.button");
        let semantic = SemanticToken::new_static("game.button");
        theme
            .0
            .token_assignments
            .insert(foreign.clone(), semantic.clone());
        theme.0.semantic_base.insert(semantic.clone(), Color::WHITE);
        let theme = run(theme);
        assert_eq!(theme.0.token_assignments.get(&foreign), Some(&semantic));
        assert_eq!(theme.0.semantic_base.get(&semantic), Some(&Color::WHITE));
        assert_eq!(theme.0.token_assignments.len(), EXAMPLE_COLORS.len() + 1);
    }

    // Pins convergence: a second run over a complete theme does not mark UiTheme changed, so the run condition falls false.
    #[test]
    fn complete_theme_is_not_marked_changed() {
        let mut world = World::new();
        world.insert_resource(UiTheme::default());
        world.run_system_once(add_missing_tooltip_tokens).unwrap();
        let before = world.resource_ref::<UiTheme>().last_changed();
        world.increment_change_tick();
        world.run_system_once(add_missing_tooltip_tokens).unwrap();
        assert_eq!(world.resource_ref::<UiTheme>().last_changed(), before);
    }
}
