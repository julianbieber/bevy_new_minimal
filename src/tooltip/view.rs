//! How tooltip content reaches the screen: rendering it into a node, and opening
//! a layer that hangs off an anchor.

use bevy::{
    app::PropagateStop,
    feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor},
    picking::events::{
        PointerClick, PointerDrag, PointerDragDrop, PointerDragEnd, PointerDragStart, PointerOut,
        PointerOver, PointerPress, PointerRelease, PointerScroll,
    },
    prelude::*,
    ui::OverrideClip,
    ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};

use super::{
    content::{TooltipContent, TooltipRegistry, warn_once_for_key},
    interaction::{TooltipChain, close_layer_on_escape, close_other_chains},
    style::{
        TOOLTIP_BG, TOOLTIP_BORDER, TOOLTIP_REF, TOOLTIP_REF_HOVER, TOOLTIP_TEXT, TOOLTIP_TITLE,
        TooltipStyle,
    },
};
use crate::escape::{self, EscapeLayer, EscapePriority};

/// On a text span inside rendered tooltip content: the span is a reference to the
/// tooltip registered under `key`.
#[derive(Component, Clone, Debug)]
pub struct TooltipRef {
    pub key: String,
}

/// Marks the root entity of one open tooltip layer.
#[derive(Component, Debug)]
pub struct TooltipLayer;

/// Renders `content` as children of the entity it is queued on, which must be a
/// UI node. Usable on ordinary game UI as well as inside tooltips: a click on a
/// reference opens it as a layer anchored to that node and pins the chain.
///
/// Every pointer press, release, click, drag and scroll inside the node stops
/// there, so the node never activates, drags or scrolls an element it sits
/// inside. Hover and move events still pass up. A reference whose key is not
/// registered when this runs renders as plain text and logs one warning per key.
pub fn render_content(content: TooltipContent) -> impl EntityCommand {
    move |entity: EntityWorldMut| {
        let node = entity.id();
        render_content_into(entity.into_world_mut(), node, &content);
    }
}

fn render_content_into(world: &mut World, node: Entity, content: &TooltipContent) {
    let style = world
        .get_resource::<TooltipStyle>()
        .cloned()
        .unwrap_or_default();
    if let Some(title) = &content.title {
        world.spawn((
            Text::new(title),
            TextFont::from_font_size(style.title_font_size),
            ThemeTextColor(TOOLTIP_TITLE),
            ChildOf(node),
        ));
    }
    let body_box = world.spawn((Node::default(), ChildOf(node))).id();
    let body = world
        .spawn((
            Text::default(),
            TextFont::from_font_size(style.body_font_size),
            Node {
                max_width: style.max_width,
                ..default()
            },
            ThemeTextColor(TOOLTIP_TEXT),
            ChildOf(body_box),
        ))
        .id();
    for span in &content.body {
        let mut entity = world.spawn((
            TextSpan::new(&span.label),
            TextFont::from_font_size(style.body_font_size),
            ChildOf(body),
        ));
        let registered = span.key.as_deref().filter(|key| {
            entity
                .world()
                .get_resource::<TooltipRegistry>()
                .is_some_and(|registry| registry.get(key).is_some())
        });
        match (registered, &span.key) {
            (Some(key), _) => {
                entity.insert((
                    ThemeTextColor(TOOLTIP_REF),
                    TooltipRef {
                        key: key.to_owned(),
                    },
                ));
            }
            (None, Some(key)) => {
                warn_once_for_key(key, "referenced but not registered; shown as plain text");
                entity.insert(ThemeTextColor(TOOLTIP_TEXT));
            }
            (None, None) => {
                entity.insert(ThemeTextColor(TOOLTIP_TEXT));
            }
        }
    }
    world
        .entity_mut(node)
        .observe(open_reference)
        .observe(highlight_reference)
        .observe(unhighlight_reference)
        .observe(stop_press)
        .observe(stop_release)
        .observe(stop_drag_start)
        .observe(stop_drag)
        .observe(stop_drag_end)
        .observe(stop_drag_drop)
        .observe(stop_scroll);
}

fn open_reference(mut click: On<PointerClick>, refs: Query<&TooltipRef>, mut commands: Commands) {
    click.propagate(false);
    if click.button != PointerButton::Primary {
        return;
    }
    let span = click.original_event_target();
    let Ok(reference) = refs.get(span) else {
        return;
    };
    let key = reference.key.clone();
    commands.queue(move |world: &mut World| {
        let Some(content) = world
            .get_resource::<TooltipRegistry>()
            .and_then(|registry| registry.get(&key))
            .cloned()
        else {
            warn_once_for_key(&key, "referenced but not registered");
            return;
        };
        open_layer(world, span, content, true);
    });
}

fn highlight_reference(
    over: On<PointerOver>,
    refs: Query<(), With<TooltipRef>>,
    mut commands: Commands,
) {
    let span = over.original_event_target();
    if refs.contains(span) {
        commands
            .entity(span)
            .insert(ThemeTextColor(TOOLTIP_REF_HOVER));
    }
}

fn unhighlight_reference(
    out: On<PointerOut>,
    refs: Query<(), With<TooltipRef>>,
    mut commands: Commands,
) {
    let span = out.original_event_target();
    if refs.contains(span) {
        commands.entity(span).insert(ThemeTextColor(TOOLTIP_REF));
    }
}

fn stop_press(mut press: On<PointerPress>) {
    press.propagate(false);
}

fn stop_release(mut release: On<PointerRelease>) {
    release.propagate(false);
}

fn stop_drag_start(mut drag: On<PointerDragStart>) {
    drag.propagate(false);
}

fn stop_drag(mut drag: On<PointerDrag>) {
    drag.propagate(false);
}

fn stop_drag_end(mut drag: On<PointerDragEnd>) {
    drag.propagate(false);
}

fn stop_drag_drop(mut drop: On<PointerDragDrop>) {
    drop.propagate(false);
}

fn stop_scroll(mut scroll: On<PointerScroll>) {
    scroll.propagate(false);
}

pub(crate) fn open_layer(world: &mut World, from: Entity, content: TooltipContent, pin: bool) {
    let Some(anchor) = find_up(world, from, |world, entity| {
        world.get::<ComputedNode>(entity).is_some()
            && world.get::<Text>(entity).is_none()
            && world.get::<Popover>(entity).is_none()
    }) else {
        return;
    };
    let owner = find_up(world, anchor, |world, entity| {
        world.get::<TooltipChain>(entity).is_some()
    })
    .unwrap_or(anchor);
    let style = world
        .get_resource::<TooltipStyle>()
        .cloned()
        .unwrap_or_default();
    let depth = world
        .get::<TooltipChain>(owner)
        .map_or(0, |chain| chain.layers.len());
    if depth >= style.max_depth {
        warn!(
            "tooltip chain already holds {} layers (TooltipStyle.max_depth); not opening another",
            style.max_depth
        );
        return;
    }
    if depth == 0 {
        close_other_chains(world, owner);
    }
    let placement = |side| PopoverPlacement {
        side,
        align: PopoverAlign::Start,
        gap: style.popover_gap,
    };
    let layer = world
        .spawn((
            TooltipLayer,
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                row_gap: style.row_gap,
                padding: UiRect::all(style.padding),
                border: UiRect::all(style.border),
                ..default()
            },
            ThemeBackgroundColor(TOOLTIP_BG),
            ThemeBorderColor(TOOLTIP_BORDER),
            Popover {
                positions: vec![
                    placement(PopoverSide::Bottom),
                    placement(PopoverSide::Top),
                    placement(PopoverSide::Right),
                    placement(PopoverSide::Left),
                ],
                window_margin: style.window_margin,
            },
            GlobalZIndex(style.base_z + depth as i32),
            OverrideClip,
            PropagateStop::<TextFont>::default(),
            PropagateStop::<TextColor>::default(),
            EscapeLayer,
            EscapePriority(escape::TOOLTIP),
            ChildOf(anchor),
        ))
        .observe(close_layer_on_escape)
        .id();
    render_content_into(world, layer, &content);
    if let Some(mut chain) = world.get_mut::<TooltipChain>(owner) {
        chain.layers.push(layer);
        chain.pinned |= pin;
    } else {
        world.entity_mut(owner).insert(TooltipChain {
            layers: vec![layer],
            pinned: pin,
        });
    }
}

fn find_up(world: &World, from: Entity, found: impl Fn(&World, Entity) -> bool) -> Option<Entity> {
    let mut entity = from;
    loop {
        if found(world, entity) {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}
