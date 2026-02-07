use std::collections::HashMap;

use bevy::{
    ecs::spawn::SpawnableList,
    feathers::{
        constants::fonts,
        controls::{ButtonProps, button},
        cursor::EntityCursor,
        dark_theme::create_dark_theme,
        font_styles::InheritableFont,
        handle_or_path::HandleOrPath,
        theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor, ThemeToken, UiTheme},
        tokens,
    },
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    ui_widgets::{Activate, observe},
    window::PrimaryWindow,
};

use crate::screens::Screen;
pub struct MainScreenPlugin;

impl Plugin for MainScreenPlugin {
    fn build(&self, app: &mut App) {
        let mut theme = create_dark_theme();
        theme
            .color
            .insert(TOOLTIP_CLICKABLE_BG, Color::oklcha(0.02, 0.4, 385.0, 1.0));
        theme
            .color
            .insert(TOOLTIP_CLICKABLE_TEXT, Color::oklcha(0.62, 0.5, 385.0, 1.0));
        theme
            .color
            .insert(TOOLTIP_BORDER, Color::oklcha(0.62, -0.5, 185.0, 1.0));
        let mut tooltips = HashMap::new();
        tooltips.insert(
            "Some".to_string(),
            Tooltip {
                text: "Some text".to_string(),
                name: "Some".to_string(),
            },
        );
        tooltips.insert(
            "text".to_string(),
            Tooltip {
                text: "Some text clickable".to_string(),
                name: "text".to_string(),
            },
        );
        tooltips.insert(
            "clickable".to_string(),
            Tooltip {
                text: "Some text containing a line break\n\ncontinue".to_string(),
                name: "clickable".to_string(),
            },
        );
        app.insert_resource(UiTheme(theme));
        app.insert_resource(TooltipMap { tooltips });
        app.insert_resource(TooltipStack {
            entities: Vec::new(),
        });
        app.add_systems(Startup, setup_camera);
        app.add_systems(OnEnter(Screen::Main), setup_ui);
        app.add_systems(OnEnter(Screen::Help), setup_help);
        app.add_systems(Update, handle_escape_help.run_if(in_state(Screen::Help)));
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_ui(mut commands: Commands) {
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
                observe(go_to_play),
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

fn go_to_play(_: On<Activate>, mut next: ResMut<NextState<Screen>>) {
    next.set(Screen::Gameplay);
}

fn setup_help(
    mut commands: Commands,
    known_toolips: Res<TooltipMap>,
    mut stack: ResMut<TooltipStack>,
) {
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
    spawn_tooltip(
        commands,
        &known_toolips.tooltips,
        &mut stack.entities,
        "Some text containing clickable words, and non clickable words\nand a line break",
        (px(0), px(0)),
        false,
    );
}

#[derive(Resource)]
struct TooltipMap {
    tooltips: HashMap<String, Tooltip>,
}

#[derive(Resource)]
struct TooltipStack {
    entities: Vec<Entity>,
}

struct Tooltip {
    text: String,
    name: String,
}
pub const TOOLTIP_CLICKABLE_BG: ThemeToken = ThemeToken::new_static("tooltip.clickable.bg");
pub const TOOLTIP_CLICKABLE_TEXT: ThemeToken = ThemeToken::new_static("tooltip.clickable.text");
pub const TOOLTIP_BORDER: ThemeToken = ThemeToken::new_static("tooltip.border");

fn spawn_tooltip(
    mut commands: Commands,
    known_tooltips: &HashMap<String, Tooltip>,
    stack: &mut Vec<Entity>,
    text: &str,
    at: (Val, Val),
    closable: bool,
) {
    let font_size = 9.0;
    let entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: at.0,
                top: at.1,
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(3.0)),
                ..Default::default()
            },
            ZIndex(stack.len() as i32 + 1),
            ThemeBackgroundColor(TOOLTIP_CLICKABLE_BG),
            ThemeBorderColor(TOOLTIP_BORDER),
        ))
        .with_children(|v| {
            for line in text.split('\n') {
                v.spawn(Node {
                    ..Default::default()
                })
                .with_children(|row| {
                    let delimiters = [' ', '.', ','];
                    let word_indices = line.match_indices(delimiters);
                    let mut start = 0;
                    for (i, delimiter) in word_indices {
                        let word = &line[start..i];
                        if let Some(tooltip) = known_tooltips.get(word) {
                            let t = tooltip.text.clone();
                            row.spawn((
                        clickable_text(
                            ButtonProps::default(),
                            (),
                            Spawn((
                                Text::new(tooltip.name.as_str()),
                                TextFont::from_font_size(font_size),
                                TextColor(Color::oklcha(0.92, -0.5, 385.0, 1.0)),
                            )),
                        ),
                        observe(
                            move |_: On<Activate>,
                                  commands: Commands,
                                  known: Res<TooltipMap>,
                                  mut stack: ResMut<TooltipStack>,
                                  window: Single<&Window, With<PrimaryWindow>>| {
                                if let Some(mouse) =window.cursor_position() {
                                    spawn_tooltip(
                                        commands,
                                        &known.tooltips,
                                        &mut stack.entities,
                                        &t,
                                        (px(mouse.x), px(mouse.y)),
                                        true
                                    );
                                }
                            },
                        ),
                    ));
                        } else {
                            row.spawn((Text::new(word), TextFont::from_font_size(font_size)));
                        }
                        row.spawn((Text::new(delimiter), TextFont::from_font_size(font_size)));
                        start = i + delimiter.len();
                    }
                    if start < text.len() {
                        row.spawn((
                            Text::new(&line[start..line.len()]),
                            TextFont::from_font_size(font_size),
                        ));
                    }
                });
            }
        })
        .id();
    if closable {
        stack.push(entity);
    }
}

pub fn clickable_text<C: SpawnableList<ChildOf> + Send + Sync + 'static, B: Bundle>(
    props: ButtonProps,
    overrides: B,
    children: C,
) -> impl Bundle {
    (
        Node {
            ..Default::default()
        },
        bevy::ui_widgets::Button,
        props.variant,
        EntityCursor::System(bevy::window::SystemCursorIcon::Help),
        TabIndex(0),
        ThemeFontColor(TOOLTIP_CLICKABLE_TEXT),
        InheritableFont {
            font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
            font_size: 14.0,
        },
        overrides,
        Children::spawn(children),
    )
}

fn handle_escape_help(
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<Screen>>,
    mut commands: Commands,
    mut stack: ResMut<TooltipStack>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Some(last) = stack.entities.pop() {
            commands.entity(last).despawn();
        } else {
            next.set(Screen::Main);
        }
    }
}

fn quit(_: On<Activate>, mut commands: Commands) {
    commands.write_message(AppExit::Success);
}
