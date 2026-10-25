//! Escape routing: hands each Escape press down the open escape layers, highest
//! priority first, until one of them takes it.

use std::sync::atomic::{AtomicU64, Ordering};

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::FocusedInput,
    prelude::*,
};

/// Priority of a screen's own Escape action, and of any [`EscapeLayer`] that sets none.
pub const SCREEN: i32 = 0;

/// Priority of an open tooltip layer. Above [`SCREEN`], so Escape closes an open
/// tooltip before the screen it sits on acts.
pub const TOOLTIP: i32 = 100;

/// Routes Escape presses to [`EscapeLayer`] entities. Add it once per app.
pub struct EscapePlugin;

impl Plugin for EscapePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(route_escape);
    }
}

/// Makes an entity a stop on the Escape chain.
///
/// On each Escape press, layers are offered the press in order of
/// [`EscapePriority`], highest first; among equal priorities the most recently
/// spawned layer goes first. The press is offered to the next layer only when the
/// current one forwards it (see [`EscapePressed::forwarded`]). A layer with no
/// observer for [`EscapePressed`] takes the press and does nothing with it.
///
/// A layer leaves the chain when its entity is despawned or the component removed,
/// so a layer spawned under a screen root carrying `DespawnOnExit` belongs to that
/// screen. Re-inserting the component keeps the layer's place among equal
/// priorities.
///
/// Observe [`EscapePressed`] on the layer entity itself; a global observer would
/// see every layer's presses.
#[derive(Component, Default)]
#[require(EscapePriority, EscapeStamp)]
#[component(on_add = stamp)]
pub struct EscapeLayer;

/// Where an [`EscapeLayer`] sits in the Escape chain; higher goes first. Defaults
/// to [`SCREEN`]. Insert it beside the layer to change it.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct EscapePriority(pub i32);

#[derive(Component, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct EscapeStamp(u64);

/// Triggered on one [`EscapeLayer`] entity when an Escape press reaches it. It
/// does not propagate to the layer's ancestors.
///
/// Set `forwarded` to `true` in the observer to decline the press, which then goes
/// to the next layer in the chain. Leaving it `false` takes the press.
#[derive(EntityEvent, Debug)]
pub struct EscapePressed {
    pub entity: Entity,
    pub forwarded: bool,
}

static NEXT_STAMP: AtomicU64 = AtomicU64::new(1);

fn stamp(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut stamp) = world.get_mut::<EscapeStamp>(context.entity) {
        *stamp = EscapeStamp(NEXT_STAMP.fetch_add(1, Ordering::Relaxed));
    }
}

fn route_escape(
    input: On<FocusedInput<KeyboardInput>>,
    windows: Query<(), With<Window>>,
    mut commands: Commands,
) {
    let key = &input.input;
    if !windows.contains(input.event_target())
        || key.key_code != KeyCode::Escape
        || key.state != ButtonState::Pressed
        || key.repeat
    {
        return;
    }
    commands.queue(offer_escape);
}

fn offer_escape(world: &mut World) {
    let order = escape_order(
        world
            .query_filtered::<(Entity, &EscapePriority, &EscapeStamp), With<EscapeLayer>>()
            .iter(world)
            .map(|(entity, priority, stamp)| (entity, *priority, *stamp)),
    );
    for entity in order {
        let mut pressed = EscapePressed {
            entity,
            forwarded: false,
        };
        world.trigger_ref(&mut pressed);
        if !pressed.forwarded {
            break;
        }
    }
}

fn escape_order(
    layers: impl IntoIterator<Item = (Entity, EscapePriority, EscapeStamp)>,
) -> Vec<Entity> {
    let mut layers: Vec<_> = layers.into_iter().collect();
    layers.sort_by_key(|&(_, priority, stamp)| std::cmp::Reverse((priority, stamp)));
    layers.into_iter().map(|(entity, ..)| entity).collect()
}

#[cfg(test)]
pub(crate) fn press_escape(world: &mut World, repeat: bool) {
    use bevy::window::PrimaryWindow;

    let existing = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next();
    let window = match existing {
        Some(window) => window,
        None => world.spawn((Window::default(), PrimaryWindow)).id(),
    };
    world.trigger(FocusedInput::new(
        window,
        KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat,
            window,
        },
        window,
    ));
    world.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Taken(Vec<&'static str>);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(EscapePlugin).init_resource::<Taken>();
        app
    }

    fn layer(app: &mut App, name: &'static str, priority: Option<i32>, forward: bool) -> Entity {
        let mut entity = app.world_mut().spawn(EscapeLayer);
        if let Some(priority) = priority {
            entity.insert(EscapePriority(priority));
        }
        entity
            .observe(
                move |mut pressed: On<EscapePressed>, mut taken: ResMut<Taken>| {
                    taken.0.push(name);
                    pressed.forwarded = forward;
                },
            )
            .id()
    }

    fn press(app: &mut App, repeat: bool) {
        press_escape(app.world_mut(), repeat);
    }

    fn taken(app: &mut App) -> Vec<&'static str> {
        std::mem::take(&mut app.world_mut().resource_mut::<Taken>().0)
    }

    // Pins that priority outranks spawn order: an older high-priority layer beats a newer low one.
    #[test]
    fn higher_priority_wins_regardless_of_spawn_order() {
        let mut app = app();
        layer(&mut app, "high", Some(TOOLTIP), false);
        layer(&mut app, "low", Some(SCREEN), false);
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["high"]);
    }

    // Pins the tie rule the user chose: among equal priorities the most recently spawned layer goes first.
    #[test]
    fn newest_layer_wins_a_tie() {
        let mut app = app();
        layer(&mut app, "old", None, false);
        layer(&mut app, "new", None, false);
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["new"]);
    }

    // Pins forwarding: a layer that declines the press hands it to the next one, and only that one.
    #[test]
    fn forwarded_press_reaches_the_next_layer_only() {
        let mut app = app();
        layer(&mut app, "bottom", Some(SCREEN), false);
        layer(&mut app, "middle", Some(50), false);
        layer(&mut app, "top", Some(TOOLTIP), true);
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["top", "middle"]);
    }

    // Pins that a press every layer forwards is dropped rather than looping or panicking.
    #[test]
    fn press_forwarded_by_every_layer_is_dropped() {
        let mut app = app();
        layer(&mut app, "a", None, true);
        layer(&mut app, "b", None, true);
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["b", "a"]);
    }

    // Pins that a despawned layer drops out of routing with no bookkeeping left behind.
    #[test]
    fn despawned_layer_drops_out() {
        let mut app = app();
        layer(&mut app, "below", None, false);
        let top = layer(&mut app, "top", None, false);
        app.world_mut().despawn(top);
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["below"]);
    }

    // Pins that a layer which forwards and despawns itself in the same press does not break the walk.
    #[test]
    fn layer_despawning_itself_while_forwarding_keeps_the_walk_going() {
        let mut app = app();
        layer(&mut app, "below", None, false);
        app.world_mut().spawn(EscapeLayer).observe(
            |mut pressed: On<EscapePressed>, mut commands: Commands| {
                pressed.forwarded = true;
                commands.entity(pressed.entity).despawn();
            },
        );
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["below"]);
    }

    // Pins that EscapePressed stays on its layer and never bubbles to the entity the layer hangs from.
    #[test]
    fn press_does_not_reach_the_layers_parent() {
        let mut app = app();
        let parent = app
            .world_mut()
            .spawn_empty()
            .observe(|_: On<EscapePressed>, mut taken: ResMut<Taken>| taken.0.push("parent"))
            .id();
        let child = layer(&mut app, "child", None, false);
        app.world_mut().entity_mut(child).insert(ChildOf(parent));
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["child"]);
    }

    // Pins that holding Escape down does not pop one layer per key repeat.
    #[test]
    fn key_repeat_routes_nothing() {
        let mut app = app();
        layer(&mut app, "only", None, false);
        press(&mut app, true);
        assert!(taken(&mut app).is_empty());
    }

    // Pins the required default: a layer without an explicit priority sits at SCREEN, and an inserted one overrides it.
    #[test]
    fn priority_defaults_to_screen_and_can_be_overridden() {
        let mut app = app();
        let plain = app.world_mut().spawn(EscapeLayer).id();
        let raised = app.world_mut().spawn((EscapeLayer, EscapePriority(7))).id();
        assert_eq!(
            app.world().get::<EscapePriority>(plain),
            Some(&EscapePriority(SCREEN))
        );
        assert_eq!(
            app.world().get::<EscapePriority>(raised),
            Some(&EscapePriority(7))
        );
    }

    // Pins that re-inserting a layer keeps its place among equal priorities instead of jumping to the top.
    #[test]
    fn reinsert_keeps_the_stamp() {
        let mut app = app();
        let first = layer(&mut app, "first", None, false);
        layer(&mut app, "second", None, false);
        app.world_mut().entity_mut(first).insert(EscapeLayer);
        press(&mut app, false);
        assert_eq!(taken(&mut app), ["second"]);
    }
}
