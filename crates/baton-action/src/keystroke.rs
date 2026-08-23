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
/// One physical keystroke is exactly one value -- which whatever detects
/// conflicting bindings relies on, since it groups by this -- but that comes
/// from the representation rather than from hiding it: [`Modifiers`] is a
/// bitflag set with no ordering, so `SHIFT | META` and `META | SHIFT` are the
/// same value, and a [`Code`] is a [`Code`].
///
/// # The one thing to know about building one by hand
///
/// [`Modifiers`] defines fourteen flags; this syntax names four, and so does
/// [`Display`](fmt::Display). A value carrying, say, `Modifiers::FN` therefore
/// prints without it and does not round-trip. That is not a hazard in practice
/// -- winit reports exactly four modifier states, so nothing else can arrive
/// from an event -- and if it ever becomes one, the formatter grows rather than
/// the fields becoming private again.
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

/// Canonical position, so one physical keystroke has one spelling.
const fn rank(m: Modifiers) -> u8 {
    match m {
        Modifiers::META => 0,
        Modifiers::SHIFT => 1,
        Modifiers::ALT => 2,
        _ => 3,
    }
}

/// The four this syntax accepts, by their W3C names.
///
/// **Deliberately not `cmd`.** That word belongs to one platform, and this
/// crate makes no platform choice: `META` is `⌘` on macOS and the Windows key
/// elsewhere. Which modifier an application treats as its primary is the
/// application's decision, made where platform decisions are allowed to live.
fn modifier(name: &str) -> Option<Modifiers> {
    match name {
        "meta" => Some(Modifiers::META),
        "shift" => Some(Modifiers::SHIFT),
        "alt" => Some(Modifiers::ALT),
        "control" => Some(Modifiers::CONTROL),
        _ => None,
    }
}

fn modifier_name(m: Modifiers) -> &'static str {
    match m {
        Modifiers::META => "meta",
        Modifiers::SHIFT => "shift",
        Modifiers::ALT => "alt",
        _ => "control",
    }
}

/// A single letter or digit is shorthand; anything else is a W3C code name.
///
/// The long form is what keeps this crate free of a key table -- every one of
/// the 216 codes is reachable, and none of them is ours to keep up to date.
fn key(name: &str) -> Result<Code, KeystrokeError> {
    let bytes = name.as_bytes();
    if bytes.len() == 1 {
        let expanded = match bytes[0] {
            b'a'..=b'z' => format!("Key{}", name.to_ascii_uppercase()),
            b'0'..=b'9' => format!("Digit{name}"),
            _ => name.to_owned(),
        };
        if let Ok(code) = Code::from_str(&expanded) {
            return Ok(code);
        }
    }
    Code::from_str(name).map_err(|_| KeystrokeError::UnknownKey {
        name: name.to_owned(),
    })
}

impl FromStr for Keystroke {
    type Err = KeystrokeError;

    /// Parses `meta+shift+KeyK`, `meta+k`, `Escape`, `F2`, `PageUp`.
    ///
    /// Modifiers come first, in the canonical order `meta`, `shift`, `alt`,
    /// `control`. Out-of-order spellings are **rejected rather than
    /// normalised**, because rejecting is the cheaper guarantee: there is then
    /// no second spelling that could hash differently.
    ///
    /// `FromStr` rather than a free function because a keymap file is
    /// configuration, and a deserializer reaches a value through `parse`. One
    /// way in, for the same reason the fields are private.
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
        let mut previous: Option<Modifiers> = None;
        for name in parts {
            if name.is_empty() {
                return Err(KeystrokeError::EmptyComponent);
            }
            let Some(m) = modifier(name) else {
                return Err(KeystrokeError::UnknownModifier {
                    name: name.to_owned(),
                });
            };
            if modifiers.contains(m) {
                return Err(KeystrokeError::RepeatedModifier {
                    name: name.to_owned(),
                });
            }
            if let Some(earlier) = previous
                && rank(m) < rank(earlier)
            {
                return Err(KeystrokeError::ModifierOutOfOrder {
                    previous: modifier_name(earlier).to_owned(),
                    current: name.to_owned(),
                });
            }
            previous = Some(m);
            modifiers |= m;
        }

        // A modifier in the key position means no key was named.
        if modifier(key_name).is_some() {
            return Err(KeystrokeError::MissingKey);
        }

        Ok(Keystroke {
            modifiers,
            code: key(key_name)?,
        })
    }
}

impl fmt::Display for Keystroke {
    /// The canonical spelling, so a parse round-trips.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for m in [
            Modifiers::META,
            Modifiers::SHIFT,
            Modifiers::ALT,
            Modifiers::CONTROL,
        ] {
            if self.modifiers.contains(m) {
                write!(f, "{}+", modifier_name(m))?;
            }
        }
        write!(f, "{}", self.code)
    }
}
