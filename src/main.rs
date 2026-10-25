//! Builds the app: the engine, UI and game plugins, and the theme they share.

use bevy::{
    feathers::{FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme},
    prelude::*,
};

use crate::{escape::EscapePlugin, screens::ScreenPlugin, tooltip::TooltipPlugin};

mod escape;
mod gameplay;
mod main_screen;
mod screens;
mod tooltip;

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FeathersPlugins,
            EscapePlugin,
            TooltipPlugin,
            ScreenPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .run()
}
