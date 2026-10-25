//! Tooltips: named or inline content attached to UI nodes, opened on hover,
//! layered by clicking references inside them, and styled by the game through
//! theme tokens and [`TooltipStyle`].

pub mod content;
pub mod interaction;
pub mod style;
pub mod view;

use bevy::{feathers::theme::UiTheme, prelude::*};

pub use content::{RegisterTooltip, TooltipContent, TooltipRegistry};
pub use interaction::Tooltip;
pub use style::TooltipStyle;
pub use view::render_content;

/// Opens and closes tooltips. Names no game state and no screen.
///
/// Inserts [`TooltipRegistry`] and [`TooltipStyle`] unless the game already did,
/// and adds the example colour of every tooltip token `UiTheme` leaves undefined,
/// again whenever the game swaps its theme. It never replaces `UiTheme`.
///
/// Tooltip layers close on Escape only when the game also adds
/// [`crate::escape::EscapePlugin`]; without it they still close on hover-out.
pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipRegistry>()
            .init_resource::<interaction::PendingTooltip>()
            .init_resource::<TooltipStyle>()
            .add_systems(
                Update,
                (
                    style::add_missing_tooltip_tokens
                        .run_if(resource_exists_and_changed::<UiTheme>),
                    interaction::track_hover_intent,
                    interaction::open_pending_tooltip,
                )
                    .chain(),
            );
    }
}
