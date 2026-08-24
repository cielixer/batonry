//! One keystroke, and the one syntax that writes it down.
//!
//! **Matching is on the physical key, never the character it produces.**
//! Measured on macOS: with the Korean 2-Set input source active, the physical
//! `A` key still arrives as `Code::KeyA`, while the logical key becomes
//! `ㅁ` -- and it stays `ㅁ` with Command held, because macOS does not
//! special-case command chords. An application matching on the produced
//! character therefore loses `⌘A` the moment someone types Korean, which in
//! this application is constantly. The same argument holds for every non-Latin
//! layout.
//!
//! The vocabulary is [`keyboard_types`], which is the W3C UI Events set: 216
//! physical codes and a modifier bitflag, platform-neutral, and not a windowing
//! library. Keeping the key table out of this crate means the whole set is
//! reachable and none of it is ours to maintain.

use std::fmt;
use std::str::FromStr;

use keyboard_types::{Code, Modifiers};

/// A modifier set plus one physical key. Plain data.
///
/// One physical keystroke is exactly one value, which conflict detection relies
/// on: [`Modifiers`] is an unordered bitflag set, so `SHIFT | META` and
/// `META | SHIFT` are the same value, and a [`Code`] is a [`Code`].
///
/// [`Modifiers`] defines fourteen flags; this syntax and
/// [`Display`](fmt::Display) name four. A value built by hand carrying, say,
/// `Modifiers::FN` prints without it and does not round-trip. Nothing else can
/// arrive from an event: winit reports exactly four modifier states.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Keystroke {
    /// The modifiers held down.
    pub modifiers: Modifiers,
    /// The physical key, independent of the layout in effect.
    pub code: Code,
}

/// Why a keystroke string is not valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeystrokeError {
    /// The string is empty, or ends in `+`, or names only modifiers.
    MissingKey,
    /// A `+`-separated component before the key is empty.
    EmptyComponent,
    /// A component contains a non-ASCII byte. Keys are named by physical
    /// position, and those names are ASCII; the character a key *produces* is
    /// deliberately not what a binding refers to.
    NonAscii,
    /// A component before the key is not one of the four modifiers.
    UnknownModifier {
        /// What was written.
        name: String,
    },
    /// A modifier appears more than once.
    RepeatedModifier {
        /// What was written.
        name: String,
    },
    /// Modifiers are not in `meta`, `shift`, `alt`, `control` order.
    ModifierOutOfOrder {
        /// The modifier that should have come later.
        previous: String,
        /// The modifier that should have come earlier.
        current: String,
    },
    /// The final component is not a physical key name.
    UnknownKey {
        /// What was written.
        name: String,
    },
}

impl fmt::Display for KeystrokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => f.write_str("keystroke names no key"),
            Self::EmptyComponent => {
                f.write_str("keystroke has an empty component")
            },
            Self::NonAscii => f.write_str(
                "keystroke must be ASCII: keys are named by physical position, \
                 not by the character they produce",
            ),
            Self::UnknownModifier { name } => {
                write!(f, "unknown modifier {name:?}")
            },
            Self::RepeatedModifier { name } => {
                write!(f, "modifier {name:?} is repeated")
            },
            Self::ModifierOutOfOrder { previous, current } => write!(
                f,
                "modifiers must read meta+shift+alt+control; {current:?} \
                 follows {previous:?}",
            ),
            Self::UnknownKey { name } => write!(
                f,
                "unknown key {name:?}: use a W3C code name such as \
                 \"BracketRight\", or a single letter or digit",
            ),
        }
    }
}

impl std::error::Error for KeystrokeError {}

/// The four modifiers this syntax names, in canonical order.
///
/// The parser, the formatter and the ordering check all read this list, so they
/// cannot drift and a fifth modifier is one edit.
///
/// **Not `cmd`, and not `command` either.** Those words belong to one platform
/// and this crate makes no platform choice: `META` is `⌘` on macOS and the
/// Windows key elsewhere. Which modifier is primary is the application's
/// decision, made where platform decisions belong.
const MODIFIERS: [(Modifiers, &str); 4] = [
    (Modifiers::META, "meta"),
    (Modifiers::SHIFT, "shift"),
    (Modifiers::ALT, "alt"),
    (Modifiers::CONTROL, "control"),
];

/// The flag a name stands for, and its canonical position.
///
/// The position comes back with the flag because the position is what the
/// ordering check wants.
fn modifier(name: &str) -> Option<(Modifiers, u8)> {
    MODIFIERS
        .iter()
        .position(|(_, spelling)| *spelling == name)
        .map(|i| (MODIFIERS[i].0, i as u8))
}

/// A single letter or digit is shorthand; anything else is a W3C code name.
///
/// The long form keeps a key table out of this crate: all 216 codes are
/// reachable, `F1` through `F24` included.
fn parse_code(name: &str) -> Result<Code, KeystrokeError> {
    let expanded = match name.as_bytes() {
        // Lowercase only. One spelling per keystroke, normalising nothing, so
        // `A` is rejected for the same reason `Meta` is.
        [c @ b'a'..=b'z'] => {
            Some(format!("Key{}", c.to_ascii_uppercase() as char))
        },
        [c @ b'0'..=b'9'] => Some(format!("Digit{}", *c as char)),
        _ => None,
    };
    // One parse, and an allocation only for the shorthand.
    Code::from_str(expanded.as_deref().unwrap_or(name)).map_err(|_| {
        KeystrokeError::UnknownKey {
            name: name.to_owned(),
        }
    })
}

impl FromStr for Keystroke {
    type Err = KeystrokeError;

    /// Parses `meta+shift+KeyK`, `meta+k`, `Escape`, `F2`, `PageUp`.
    ///
    /// Modifiers come first, in the canonical order `meta`, `shift`, `alt`,
    /// `control`. Out-of-order spellings are **rejected rather than
    /// normalised**: no second spelling can then hash differently.
    ///
    /// `FromStr` because a keymap file is configuration, and a deserializer
    /// reaches a value through `parse`.
    fn from_str(input: &str) -> Result<Keystroke, KeystrokeError> {
        if input.is_empty() {
            return Err(KeystrokeError::MissingKey);
        }
        if !input.is_ascii() {
            return Err(KeystrokeError::NonAscii);
        }

        let mut parts: Vec<&str> = input.split('+').collect();
        let key_name = parts.pop().ok_or(KeystrokeError::MissingKey)?;
        if key_name.is_empty() {
            return Err(KeystrokeError::MissingKey);
        }

        let mut modifiers = Modifiers::empty();
        let mut previous: Option<(&str, u8)> = None;
        for name in parts {
            if name.is_empty() {
                return Err(KeystrokeError::EmptyComponent);
            }
            let Some((flag, rank)) = modifier(name) else {
                return Err(KeystrokeError::UnknownModifier {
                    name: name.to_owned(),
                });
            };
            if modifiers.contains(flag) {
                return Err(KeystrokeError::RepeatedModifier {
                    name: name.to_owned(),
                });
            }
            if let Some((earlier, earlier_rank)) = previous
                && rank < earlier_rank
            {
                return Err(KeystrokeError::ModifierOutOfOrder {
                    previous: earlier.to_owned(),
                    current: name.to_owned(),
                });
            }
            previous = Some((name, rank));
            modifiers |= flag;
        }

        // A modifier in the key position means no key was named.
        if modifier(key_name).is_some() {
            return Err(KeystrokeError::MissingKey);
        }

        Ok(Keystroke {
            modifiers,
            code: parse_code(key_name)?,
        })
    }
}

impl fmt::Display for Keystroke {
    /// The canonical spelling, so a parse round-trips.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, spelling) in MODIFIERS {
            if self.modifiers.contains(flag) {
                write!(f, "{spelling}+")?;
            }
        }
        write!(f, "{}", self.code)
    }
}
