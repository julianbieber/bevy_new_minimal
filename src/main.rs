use bevy::{
    feathers::{
        FeathersPlugins,
        controls::{ButtonProps, button},
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, UiTheme},
        tokens,
    },
    prelude::*,
    ui_widgets::{Activate, observe},
};

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins, ScreenPlugin))
        .run()
}

pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Screen>();
        app.add_plugins((MainScreenPlugin, GameplayPlugin));
    }
}

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum Screen {
    #[default]
    Main,
    Help,
    Gameplay,
}

////////////

pub struct MainScreenPlugin;

impl Plugin for MainScreenPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiTheme(create_dark_theme()));
        app.add_systems(OnEnter(Screen::Main), setup_ui);
        app.add_systems(OnEnter(Screen::Help), setup_help);
    }
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(main_root());
}

/// 3 Buttons:
/// * Play
/// * Help
/// * Quit
fn main_root() -> impl Bundle {
    (
        DespawnOnExit(Screen::Main),
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: percent(100),
            height: percent(100),
            row_gap: px(10),
            ..Default::default()
        },
        ThemeBackgroundColor(tokens::WINDOW_BG),
        children![
            (
                button(ButtonProps::default(), (), Spawn(Text::new("Play!"))),
                observe(|_: On<Activate>| { info!("play pressed") }),
            ),
            (
                button(ButtonProps::default(), (), Spawn(Text::new("Help"))),
                observe(go_to_help),
            ),
            (
                button(ButtonProps::default(), (), Spawn(Text::new("Quit"))),
                observe(quit),
            )
        ],
    )
}

fn go_to_help(_: On<Activate>, mut next: ResMut<NextState<Screen>>) {
    next.set(Screen::Help);
}

fn setup_help(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(Screen::Help),
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: percent(100),
            height: percent(100),
            row_gap: px(10),
            ..Default::default()
        },
        ThemeBackgroundColor(tokens::WINDOW_BG),
        children![Text::new("Some text to explain how to play the game")],
    ));
}

fn quit(_: On<Activate>, mut commands: Commands) {
    commands.write_message(AppExit::Success);
}

////////////////

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, _app: &mut App) {}
}
