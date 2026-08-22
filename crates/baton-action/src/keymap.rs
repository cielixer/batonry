//! Bindings and the canonical key-chord syntax.
//!
//! A binding is deliberately separate from [`crate::ActionMeta`]. The action
//! says what exists; this table says how a particular keymap reaches it. That
//! is what lets a runtime-loaded TOML value use owned strings without leaking
//! them into a supposedly permanent action table.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// How an action is reached in a keymap.
///
/// `Cow` is the seam between the built-in table and a user's file: defaults
/// borrow string literals, while deserialized values own their strings.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Binding {
    /// The stable id of the action this binding reaches.
    pub action: Cow<'static, str>,
    /// The canonical ASCII spelling of the chord.
    pub key: Cow<'static, str>,
    /// An opaque context guard, parsed later by issue #11.
    pub when: Option<Cow<'static, str>>,
}

/// The built-in bindings for the ten actions implemented in stage 1.
pub const DEFAULT_KEYMAP: &[Binding] = &[
    Binding {
        action: Cow::Borrowed("palette.open"),
        key: Cow::Borrowed("cmd+k"),
        when: Some(Cow::Borrowed("!palette_open")),
    },
    Binding {
        action: Cow::Borrowed("palette.close"),
        key: Cow::Borrowed("escape"),
        when: Some(Cow::Borrowed("palette_open")),
    },
    Binding {
        action: Cow::Borrowed("palette.confirm"),
        key: Cow::Borrowed("enter"),
        when: Some(Cow::Borrowed("palette_open")),
    },
    Binding {
        action: Cow::Borrowed("palette.next"),
        key: Cow::Borrowed("down"),
        when: Some(Cow::Borrowed("palette_open")),
    },
    Binding {
        action: Cow::Borrowed("palette.prev"),
        key: Cow::Borrowed("up"),
        when: Some(Cow::Borrowed("palette_open")),
    },
    Binding {
        action: Cow::Borrowed("app.quit"),
        key: Cow::Borrowed("cmd+q"),
        when: None,
    },
    Binding {
        action: Cow::Borrowed("term.copy"),
        key: Cow::Borrowed("cmd+c"),
        when: Some(Cow::Borrowed("has_selection")),
    },
    Binding {
        action: Cow::Borrowed("term.paste"),
        key: Cow::Borrowed("cmd+v"),
        when: Some(Cow::Borrowed("pane_live")),
    },
    Binding {
        action: Cow::Borrowed("term.select_all"),
        key: Cow::Borrowed("cmd+a"),
        when: Some(Cow::Borrowed("pane_focused")),
    },
    Binding {
        action: Cow::Borrowed("term.clear"),
        key: Cow::Borrowed("cmd+shift+k"),
        when: Some(Cow::Borrowed("pane_live")),
    },
];

/// A parsed key chord suitable for equality and hash-map conflict checks.
///
/// The fields stay private so every chord enters the system through the one
/// canonical parser. Two spellings of the same physical chord must not be
/// allowed to become distinct map keys.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Chord {
    modifiers: Modifiers,
    key: Key,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Modifiers(u8);

impl Modifiers {
    const fn empty() -> Self {
        Self(0)
    }

    const fn contains(self, modifier: Modifier) -> bool {
        self.0 & modifier.bit() != 0
    }

    const fn insert(self, modifier: Modifier) -> Self {
        Self(self.0 | modifier.bit())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Modifier {
    Cmd,
    Shift,
    Alt,
    Ctrl,
}

impl Modifier {
    /// Canonical position. One spelling per chord is what lets [`Chord`] be a
    /// hash-map key in the conflict check, so the order is enforced rather
    /// than normalised away.
    const fn rank(self) -> u8 {
        match self {
            Self::Cmd => 0,
            Self::Shift => 1,
            Self::Alt => 2,
            Self::Ctrl => 3,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Cmd => "cmd",
            Self::Shift => "shift",
            Self::Alt => "alt",
            Self::Ctrl => "ctrl",
        }
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Cmd => 1 << 0,
            Self::Shift => 1 << 1,
            Self::Alt => 1 << 2,
            Self::Ctrl => 1 << 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Key {
    Letter(u8),
    Digit(u8),
    Escape,
    Enter,
    Up,
    Down,
    Left,
    Right,
    Comma,
    Function(u8),
}

/// Why a key-chord string is not valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The string contains no key.
    MissingKey,
    /// A `+`-separated component other than the final key is empty.
    EmptyComponent,
    /// The spelling contains a non-ASCII byte.
    NonAscii,
    /// A component before the key is not one of the four modifiers.
    UnknownModifier { name: String },
    /// A modifier appears more than once.
    RepeatedModifier { name: String },
    /// Modifiers are not in `cmd`, `shift`, `alt`, `ctrl` order.
    ModifierOutOfOrder { previous: String, current: String },
    /// The final component is not a supported key name.
    UnknownKey { name: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => f.write_str("key chord is missing its key"),
            Self::EmptyComponent => {
                f.write_str("key chord contains an empty component")
            },
            Self::NonAscii => {
                f.write_str("key chord must contain only ASCII characters")
            },
            Self::UnknownModifier { name } => {
                write!(f, "unknown key modifier {name:?}")
            },
            Self::RepeatedModifier { name } => {
                write!(f, "key modifier {name:?} is repeated")
            },
            Self::ModifierOutOfOrder { previous, current } => write!(
                f,
                "key modifiers must be ordered cmd+shift+alt+ctrl; {current:?} follows {previous:?}",
            ),
            Self::UnknownKey { name } => write!(f, "unknown key name {name:?}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Chord {
    type Err = ParseError;

    /// Parses one lowercase ASCII chord such as `cmd+shift+k` or `escape`.
    ///
    /// Modifiers must precede the key, in the canonical order `cmd`, `shift`,
    /// `alt`, `ctrl`. Enforcing one spelling rather than normalising means
    /// conflict detection can use [`Chord`] directly as a hash-map key.
    ///
    /// This is `FromStr` and not a free function because the user's keymap is
    /// TOML: a deserializer reaches a chord through `parse`, and there must be
    /// exactly one way in -- the same reason [`Chord`]'s fields are private.
    fn from_str(input: &str) -> Result<Chord, ParseError> {
        if input.is_empty() {
            return Err(ParseError::MissingKey);
        }
        if !input.is_ascii() {
            return Err(ParseError::NonAscii);
        }

        let mut components: Vec<&str> = input.split('+').collect();
        let key_name = components.pop().ok_or(ParseError::MissingKey)?;
        if key_name.is_empty() {
            return Err(ParseError::MissingKey);
        }

        let mut modifiers = Modifiers::empty();
        let mut previous: Option<Modifier> = None;
        for name in components {
            if name.is_empty() {
                return Err(ParseError::EmptyComponent);
            }
            let Some(modifier) = parse_modifier(name) else {
                return Err(ParseError::UnknownModifier {
                    name: name.to_owned(),
                });
            };
            if modifiers.contains(modifier) {
                return Err(ParseError::RepeatedModifier {
                    name: name.to_owned(),
                });
            }
            if let Some(earlier) = previous
                && modifier.rank() < earlier.rank()
            {
                return Err(ParseError::ModifierOutOfOrder {
                    previous: earlier.name().to_owned(),
                    current: name.to_owned(),
                });
            }
            previous = Some(modifier);
            modifiers = modifiers.insert(modifier);
        }

        if parse_modifier(key_name).is_some() {
            return Err(ParseError::MissingKey);
        }

        Ok(Chord {
            modifiers,
            key: parse_key(key_name)?,
        })
    }
}

fn parse_modifier(name: &str) -> Option<Modifier> {
    match name {
        "cmd" => Some(Modifier::Cmd),
        "shift" => Some(Modifier::Shift),
        "alt" => Some(Modifier::Alt),
        "ctrl" => Some(Modifier::Ctrl),
        _ => None,
    }
}

fn parse_key(name: &str) -> Result<Key, ParseError> {
    let key = match name {
        "escape" => Key::Escape,
        "enter" => Key::Enter,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "comma" => Key::Comma,
        _ => {
            let bytes = name.as_bytes();
            if bytes.len() == 1 {
                match bytes[0] {
                    b'a'..=b'z' => Key::Letter(bytes[0] - b'a'),
                    b'0'..=b'9' => Key::Digit(bytes[0] - b'0'),
                    _ => return unknown_key(name),
                }
            } else if let Some(number) = name.strip_prefix('f') {
                let Ok(number) = number.parse::<u8>() else {
                    return unknown_key(name);
                };
                if (1..=24).contains(&number) {
                    Key::Function(number)
                } else {
                    return unknown_key(name);
                }
            } else {
                return unknown_key(name);
            }
        },
    };
    Ok(key)
}

fn unknown_key(name: &str) -> Result<Key, ParseError> {
    Err(ParseError::UnknownKey {
        name: name.to_owned(),
    })
}
