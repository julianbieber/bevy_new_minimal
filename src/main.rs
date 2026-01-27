use bevy::prelude::*;

fn main() -> AppExit {
    App::new().add_plugins(DefaultPlugins).run()
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
    Gameplay,
}

////////////

pub struct MainScreenPlugin;

impl Plugin for MainScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Main), setup_ui);
    }
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn main_root() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            ..Default::default()
        },
        children![],
    )
}

////////////////

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, _app: &mut App) {
        todo!()
    }
}
