//! What a tooltip says: its content, the reference markup that content is
//! written in, and the registry of tooltips a game has named.

use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex},
};

use bevy::prelude::*;

/// One run of a tooltip body. With a `key` it is a reference to the tooltip
/// registered under that key, shown as `label`; without one it is plain text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TooltipSpan {
    pub label: String,
    pub key: Option<String>,
}

impl TooltipSpan {
    pub fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            key: None,
        }
    }

    /// A reference to the tooltip registered under `key`, shown as `label`.
    pub fn reference(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            key: Some(key.into()),
        }
    }
}

/// An optional title over a body of [`TooltipSpan`]s. Build it with
/// [`TooltipContent::new`] from markup, or from the fields directly for a game
/// that writes its own markup.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TooltipContent {
    pub title: Option<String>,
    pub body: Vec<TooltipSpan>,
}

impl TooltipContent {
    /// Builds content from body markup, parsed once here and never per frame.
    ///
    /// `[[key]]` is a reference to the tooltip registered under `key`, shown as
    /// `key`; `[[key|label]]` shows it as `label`. The inner text splits on the
    /// first `|`, and both halves are trimmed. A reference with an empty key, and
    /// an unclosed `[[` through to the end of the string, are kept as literal text.
    pub fn new(markup: &str) -> Self {
        Self {
            title: None,
            body: parse_body(markup),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// True when there is neither a title nor any body text to show.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.body.iter().all(|span| span.label.is_empty())
    }
}

fn parse_body(markup: &str) -> Vec<TooltipSpan> {
    let mut spans = Vec::new();
    let mut text = String::new();
    let mut rest = markup;
    while let Some(open) = rest.find("[[") {
        let Some(close) = rest[open + 2..].find("]]").map(|i| open + 2 + i) else {
            break;
        };
        text.push_str(&rest[..open]);
        let inner = &rest[open + 2..close];
        let (key, label) = inner.split_once('|').unwrap_or((inner, inner));
        let (key, label) = (key.trim(), label.trim());
        if key.is_empty() {
            text.push_str(&rest[open..close + 2]);
        } else {
            if !text.is_empty() {
                spans.push(TooltipSpan::text(std::mem::take(&mut text)));
            }
            spans.push(TooltipSpan::reference(key, label));
        }
        rest = &rest[close + 2..];
    }
    text.push_str(rest);
    if !text.is_empty() {
        spans.push(TooltipSpan::text(text));
    }
    spans
}

/// The tooltips a game has named, looked up by key from [`super::Tooltip::Key`]
/// anchors and from references inside other tooltips.
#[derive(Resource, Default)]
pub struct TooltipRegistry {
    tooltips: HashMap<String, TooltipContent>,
}

impl TooltipRegistry {
    /// Registers `content` under `key`, replacing any earlier entry.
    pub fn insert(&mut self, key: impl Into<String>, content: TooltipContent) {
        self.tooltips.insert(key.into(), content);
    }

    pub fn get(&self, key: &str) -> Option<&TooltipContent> {
        self.tooltips.get(key)
    }
}

pub trait RegisterTooltip {
    /// Registers `content` under `key` in [`TooltipRegistry`], creating the
    /// registry first, so it may run before or after `TooltipPlugin` is added.
    fn register_tooltip(&mut self, key: impl Into<String>, content: TooltipContent) -> &mut Self;
}

impl RegisterTooltip for App {
    fn register_tooltip(&mut self, key: impl Into<String>, content: TooltipContent) -> &mut Self {
        self.init_resource::<TooltipRegistry>();
        self.world_mut()
            .resource_mut::<TooltipRegistry>()
            .insert(key, content);
        self
    }
}

static WARNED_KEYS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

pub(crate) fn warn_once_for_key(key: &str, problem: &str) {
    let mut warned = WARNED_KEYS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if warned.insert(key.to_owned()) {
        warn!("tooltip `{key}`: {problem}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(label: &str) -> TooltipSpan {
        TooltipSpan::text(label)
    }

    fn reference(key: &str, label: &str) -> TooltipSpan {
        TooltipSpan::reference(key, label)
    }

    // Pins that markup with no references is one plain span.
    #[test]
    fn plain_text_is_one_span() {
        assert_eq!(parse_body("just words"), [text("just words")]);
    }

    // Pins that [[key]] shows the key itself as the label.
    #[test]
    fn bare_reference_uses_key_as_label() {
        assert_eq!(
            parse_body("see [[fire]] now"),
            [text("see "), reference("fire", "fire"), text(" now")]
        );
    }

    // Pins that the label splits off at the first | only, so a label may contain |.
    #[test]
    fn label_splits_on_first_pipe_only() {
        assert_eq!(parse_body("[[k|a|b]]"), [reference("k", "a|b")]);
    }

    // Pins that whitespace around key and label is trimmed, so a spaced key still resolves.
    #[test]
    fn key_and_label_are_trimmed() {
        assert_eq!(parse_body("[[ fire | Fire ]]"), [reference("fire", "Fire")]);
    }

    // Pins that a reference with no key names nothing and stays literal text.
    #[test]
    fn empty_key_is_literal() {
        assert_eq!(parse_body("a [[ |x]] b"), [text("a [[ |x]] b")]);
    }

    // Pins that an unclosed [[ is literal through to the end of the string.
    #[test]
    fn unclosed_reference_is_literal() {
        assert_eq!(
            parse_body("[[a]] then [[b"),
            [reference("a", "a"), text(" then [[b")]
        );
    }

    // Pins that text between references coalesces into one span, literal reference text included.
    #[test]
    fn adjacent_text_coalesces() {
        assert_eq!(parse_body("x [[|y]] z"), [text("x [[|y]] z")]);
        assert_eq!(
            parse_body("[[a]][[b]]"),
            [reference("a", "a"), reference("b", "b")]
        );
    }

    // Pins that registering before the plugin exists still lands in the registry.
    #[test]
    fn register_tooltip_creates_the_registry() {
        let mut app = App::new();
        app.register_tooltip("k", TooltipContent::new("body"));
        assert_eq!(
            app.world().resource::<TooltipRegistry>().get("k"),
            Some(&TooltipContent::new("body"))
        );
    }
}
