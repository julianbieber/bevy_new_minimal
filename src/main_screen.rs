//! The screens outside gameplay, and the example tooltips they show.

use bevy::{
    feathers::{
        controls::FeathersButton,
        theme::{ThemeBackgroundColor, ThemedText},
        tokens,
    },
    input_focus::AutoFocus,
    prelude::*,
    ui_widgets::Activate,
};

use crate::{
    escape::{self, EscapeLayer, EscapePressed, EscapePriority},
    screens::Screen,
    tooltip::{RegisterTooltip, Tooltip, TooltipContent, render_content},
};
/// Spawns the main menu on entering `Screen::Main` and the help page on entering
/// `Screen::Help`, and registers the example tooltips both show. Escape leaves
/// the help page only when `EscapePlugin` is also added.
pub struct MainScreenPlugin;

impl Plugin for MainScreenPlugin {
    fn build(&self, app: &mut App) {
        app.register_tooltip(
            "help",
            TooltipContent::new("Opens a page about how [[tooltip|tooltips]] work.")
                .with_title("Help"),
        )
        .register_tooltip(
            "tooltip",
            TooltipContent::new(
                "A panel that explains what is under the pointer. It can mention other \
                 tooltips by [[reference]].",
            )
            .with_title("Tooltip"),
        )
        .register_tooltip(
            "reference",
            TooltipContent::new(
                "A highlighted word inside a tooltip. Click it to open its own tooltip as a \
                 new [[layer]] on top.",
            )
            .with_title("Reference"),
        )
        .register_tooltip(
            "layer",
            TooltipContent::new(
                "One open tooltip in a chain. Clicking a reference pins the chain, and \
                 Escape closes the newest layer first. A chain has a [[depth|maximum depth]].",
            )
            .with_title("Layer"),
        )
        .register_tooltip(
            "depth",
            TooltipContent::new("A chain holds at most TooltipStyle::max_depth layers."),
        );
        app.add_systems(Startup, setup_camera);
        app.add_systems(OnEnter(Screen::Main), setup_ui);
        app.add_systems(OnEnter(Screen::Help), setup_help);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_ui(mut commands: Commands) {
    commands.spawn_scene(main_root());
}

fn main_root() -> impl Scene {
    bsn! {
        DespawnOnExit<Screen>(Screen::Main)
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: percent(100),
            height: percent(100),
            row_gap: px(10),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[
            @FeathersButton{
                @caption: bsn! {Text("Play!") ThemedText}
            }
            on(go_to_play)
            AutoFocus
            --
            @FeathersButton{
                @caption: bsn! {Text("Help") ThemedText}
            }
            on(go_to_help)
            Tooltip::Key("help")
            AutoFocus
            --
            @FeathersButton{
                @caption: bsn! {Text("Quit") ThemedText}
            }
            on(quit)
            AutoFocus
        ]
    }
}

fn go_to_help(_: On<Activate>, mut next: ResMut<NextState<Screen>>) {
    next.set(Screen::Help);
}

fn go_to_play(_: On<Activate>, mut next: ResMut<NextState<Screen>>) {
    next.set(Screen::Gameplay);
}

fn setup_help(mut commands: Commands) {
    let root = commands
        .spawn((
            DespawnOnExit(Screen::Help),
            EscapeLayer,
            EscapePriority(escape::SCREEN),
            Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(24)),
                ..default()
            },
            ThemeBackgroundColor(tokens::WINDOW_BG),
        ))
        .observe(back_to_main)
        .id();
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            ChildOf(root),
        ))
        .queue(render_content(
            TooltipContent::new(
                "Hover the Help button on the main screen to see a [[tooltip]]. Highlighted \
                 words are references: click one to pin a new [[layer]] on top of this page. \
                 Escape closes the top layer first, and goes back to the main screen once \
                 none is open.",
            )
            .with_title("Help"),
        ));
}

fn back_to_main(_: On<EscapePressed>, mut next: ResMut<NextState<Screen>>) {
    next.set(Screen::Main);
}

fn quit(_: On<Activate>, mut commands: Commands) {
    commands.write_message(AppExit::Success);
}
