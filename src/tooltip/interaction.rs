//! When tooltips open and close: hover intent on anchors, the open chain each
//! anchor owns, and closing that chain's top layer when Escape reaches it.

use std::time::Duration;

use bevy::{picking::hover::Hovered, prelude::*};

use super::{
    content::{TooltipContent, TooltipRegistry, warn_once_for_key},
    style::TooltipStyle,
    view::open_layer,
};
use crate::escape::EscapePressed;

/// Gives a UI node a tooltip that opens when the pointer rests on it.
///
/// `Key` shows the tooltip registered under that key in [`TooltipRegistry`];
/// `Inline` carries its own content, and is the empty default. An unregistered key or empty content opens
/// nothing and logs one warning. Mouse only: touch reports no hover.
#[derive(Component, Clone, Debug)]
#[require(Hovered)]
pub enum Tooltip {
    Key(String),
    Inline(TooltipContent),
}

impl Default for Tooltip {
    fn default() -> Self {
        Self::Inline(TooltipContent::default())
    }
}

#[derive(Component, Default, Debug)]
pub(crate) struct TooltipChain {
    pub(crate) layers: Vec<Entity>,
    pub(crate) pinned: bool,
}

#[derive(Resource, Default, Debug)]
pub(crate) struct PendingTooltip {
    candidate: Option<Entity>,
    elapsed: Duration,
    closing: Option<Entity>,
    closing_for: Duration,
}

pub(crate) fn track_hover_intent(
    changed: Query<(Entity, &Hovered), (With<Tooltip>, Changed<Hovered>)>,
    chains: Query<&TooltipChain>,
    parents: Query<&ChildOf>,
    mut pending: ResMut<PendingTooltip>,
) {
    let entered = changed
        .iter()
        .filter(|(_, hovered)| hovered.0)
        .map(|(entity, _)| entity)
        .max_by_key(|&entity| parents.iter_ancestors(entity).count());
    if let Some(entered) = entered {
        if pending.closing == Some(entered) {
            pending.closing = None;
        }
        pending.candidate = Some(entered);
        pending.elapsed = Duration::ZERO;
    }
    for (entity, hovered) in &changed {
        if hovered.0 {
            continue;
        }
        if pending.candidate == Some(entity) {
            pending.candidate = None;
        }
        if chains.get(entity).is_ok_and(|chain| !chain.pinned) {
            pending.closing = Some(entity);
            pending.closing_for = Duration::ZERO;
        }
    }
}

pub(crate) fn open_pending_tooltip(
    time: Res<Time>,
    style: Res<TooltipStyle>,
    registry: Res<TooltipRegistry>,
    mut pending: ResMut<PendingTooltip>,
    tooltips: Query<&Tooltip>,
    chains: Query<(), With<TooltipChain>>,
    mut commands: Commands,
) {
    if let Some(owner) = pending.closing {
        pending.closing_for += time.delta();
        if pending.closing_for >= style.close_delay {
            pending.closing = None;
            commands.queue(move |world: &mut World| close_chain(world, owner));
        }
    }
    let Some(candidate) = pending.candidate else {
        return;
    };
    pending.elapsed += time.delta();
    if pending.elapsed < style.hover_delay {
        return;
    }
    pending.candidate = None;
    pending.elapsed = Duration::ZERO;
    let Ok(tooltip) = tooltips.get(candidate) else {
        return;
    };
    if chains.contains(candidate) {
        return;
    }
    let content = match tooltip {
        Tooltip::Key(key) => match registry.get(key) {
            Some(content) => content.clone(),
            None => {
                warn_once_for_key(key, "not registered; nothing opened");
                return;
            }
        },
        Tooltip::Inline(content) => content.clone(),
    };
    if content.is_empty() {
        let name = match tooltip {
            Tooltip::Key(key) => key.clone(),
            Tooltip::Inline(_) => format!("inline on {candidate}"),
        };
        warn_once_for_key(&name, "has no title and no body; nothing opened");
        return;
    }
    commands.queue(move |world: &mut World| open_layer(world, candidate, content, false));
}

pub(crate) fn close_layer_on_escape(pressed: On<EscapePressed>, mut commands: Commands) {
    let layer = pressed.entity;
    commands.queue(move |world: &mut World| close_layer(world, layer));
}

fn close_layer(world: &mut World, layer: Entity) {
    let owner = world
        .query::<(Entity, &TooltipChain)>()
        .iter(world)
        .find(|(_, chain)| chain.layers.contains(&layer))
        .map(|(owner, _)| owner);
    let Some(owner) = owner else {
        return;
    };
    let closed = {
        let mut chain = world.get_mut::<TooltipChain>(owner).unwrap();
        let at = chain.layers.iter().position(|&l| l == layer).unwrap();
        chain.layers.split_off(at)
    };
    despawn_layers(world, closed);
    if world
        .get::<TooltipChain>(owner)
        .is_some_and(|chain| chain.layers.is_empty())
    {
        world.entity_mut(owner).remove::<TooltipChain>();
        forget_close(world, owner);
    }
}

pub(crate) fn close_chain(world: &mut World, owner: Entity) {
    let Ok(mut entity) = world.get_entity_mut(owner) else {
        forget_close(world, owner);
        return;
    };
    let Some(chain) = entity.take::<TooltipChain>() else {
        forget_close(world, owner);
        return;
    };
    despawn_layers(world, chain.layers);
    forget_close(world, owner);
}

pub(crate) fn close_other_chains(world: &mut World, keep: Entity) {
    let others: Vec<Entity> = world
        .query_filtered::<Entity, With<TooltipChain>>()
        .iter(world)
        .filter(|&owner| owner != keep)
        .collect();
    for owner in others {
        close_chain(world, owner);
    }
}

fn despawn_layers(world: &mut World, layers: Vec<Entity>) {
    for layer in layers.into_iter().rev() {
        if let Ok(entity) = world.get_entity_mut(layer) {
            entity.despawn();
        }
    }
}

fn forget_close(world: &mut World, owner: Entity) {
    if let Some(mut pending) = world.get_resource_mut::<PendingTooltip>()
        && pending.closing == Some(owner)
    {
        pending.closing = None;
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::popover::Popover;

    use super::*;
    use crate::escape::{self, EscapeLayer, EscapePlugin, EscapePriority, press_escape};

    #[derive(Resource, Default)]
    struct ScreenActions(u32);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(EscapePlugin)
            .init_resource::<TooltipRegistry>()
            .init_resource::<TooltipStyle>()
            .init_resource::<PendingTooltip>()
            .init_resource::<ScreenActions>();
        app
    }

    fn anchor(app: &mut App) -> Entity {
        app.world_mut().spawn(Node::default()).id()
    }

    fn open(app: &mut App, from: Entity, pin: bool) -> Entity {
        open_layer(app.world_mut(), from, TooltipContent::new("body"), pin);
        app.world_mut().flush();
        let world = app.world_mut();
        let mut chains = world.query::<&TooltipChain>();
        *chains.iter(world).next().unwrap().layers.last().unwrap()
    }

    fn layers(app: &mut App, owner: Entity) -> Option<Vec<Entity>> {
        app.world()
            .get::<TooltipChain>(owner)
            .map(|chain| chain.layers.clone())
    }

    // Pins that Escape on a pinned three-deep chain closes the newest layer first and removes the chain once empty.
    #[test]
    fn escape_pops_a_pinned_chain_innermost_first() {
        let mut app = app();
        let owner = anchor(&mut app);
        let first = open(&mut app, owner, true);
        let second = open(&mut app, first, true);
        let third = open(&mut app, second, true);
        assert_eq!(layers(&mut app, owner), Some(vec![first, second, third]));

        press_escape(app.world_mut(), false);
        assert_eq!(layers(&mut app, owner), Some(vec![first, second]));
        assert!(app.world().get_entity(third).is_err());

        press_escape(app.world_mut(), false);
        press_escape(app.world_mut(), false);
        assert_eq!(layers(&mut app, owner), None);
    }

    // Pins that Escape closing a chain whose close delay is counting down cancels that countdown.
    #[test]
    fn escape_during_close_countdown_clears_it() {
        let mut app = app();
        let owner = anchor(&mut app);
        open(&mut app, owner, false);
        app.world_mut().resource_mut::<PendingTooltip>().closing = Some(owner);

        press_escape(app.world_mut(), false);
        assert_eq!(layers(&mut app, owner), None);
        assert_eq!(app.world().resource::<PendingTooltip>().closing, None);
    }

    // Pins the per-screen convention: Escape closes an open tooltip first, and reaches the screen's own action only once none is open.
    #[test]
    fn tooltip_outranks_the_screen_layer() {
        let mut app = app();
        app.world_mut()
            .spawn((EscapeLayer, EscapePriority(escape::SCREEN)))
            .observe(|_: On<EscapePressed>, mut actions: ResMut<ScreenActions>| actions.0 += 1);
        let owner = anchor(&mut app);
        open(&mut app, owner, true);

        press_escape(app.world_mut(), false);
        assert_eq!(layers(&mut app, owner), None);
        assert_eq!(app.world().resource::<ScreenActions>().0, 0);

        press_escape(app.world_mut(), false);
        assert_eq!(app.world().resource::<ScreenActions>().0, 1);
    }

    // Pins that a reference click past TooltipStyle.max_depth opens nothing rather than growing the chain.
    #[test]
    fn chain_stops_at_max_depth() {
        let mut app = app();
        app.world_mut().resource_mut::<TooltipStyle>().max_depth = 2;
        let owner = anchor(&mut app);
        let first = open(&mut app, owner, true);
        let second = open(&mut app, first, true);
        open_layer(app.world_mut(), second, TooltipContent::new("x"), true);
        app.world_mut().flush();
        assert_eq!(layers(&mut app, owner), Some(vec![first, second]));
    }

    // Pins the anchor rule: a layer opened from a span hangs off a plain node, never the Text node (its layout breaks) or a popover (Popover skips popover parents).
    #[test]
    fn layer_opened_from_a_span_anchors_to_a_plain_node() {
        let mut app = app();
        let owner = anchor(&mut app);
        open(&mut app, owner, true);
        let world = app.world_mut();
        let span = world
            .query_filtered::<Entity, With<TextSpan>>()
            .iter(world)
            .next()
            .unwrap();
        let second = open(&mut app, span, true);
        let parent = app.world().get::<ChildOf>(second).unwrap().parent();
        assert!(app.world().get::<Text>(parent).is_none());
        assert!(app.world().get::<Popover>(parent).is_none());
        assert!(app.world().get::<ComputedNode>(parent).is_some());
    }

    // Pins that opening a chain on one anchor closes the chain open on another, so at most one exists.
    #[test]
    fn opening_a_chain_closes_any_other() {
        let mut app = app();
        let a = anchor(&mut app);
        let b = anchor(&mut app);
        open(&mut app, a, true);
        open(&mut app, b, false);
        assert_eq!(layers(&mut app, a), None);
        assert_eq!(layers(&mut app, b).map(|l| l.len()), Some(1));
    }
}
