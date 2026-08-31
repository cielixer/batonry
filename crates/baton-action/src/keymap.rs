//! The assembled keymap and the one place where bindings are looked up.

use std::collections::HashMap;

use keyboard_types::{Code, Modifiers};

use crate::{
    ActionId, Binding, EDITING_TEXT, Flags, Keystroke, Predicate, Registry,
    evaluate, holds,
};

/// One binding, minus its chord: the chord is the key it is stored under.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Bound {
    action: ActionId,
    when: Option<Predicate>,
}

/// A keymap whose entries all resolve in the registry used to assemble it.
///
/// **Keyed by chord**, because that is what a keypress has. A chord may
/// legitimately repeat when the conditions are disjoint, so the value is a list
/// and the guard picks from it; what must never reach here is two whose
/// conditions can hold together, since order would then decide and "whichever
/// was written first" is a rule nobody can predict from a keymap file. #12
/// fails the build on that, which is why this does not try to resolve it.
#[derive(Clone, Debug, Default)]
pub struct Keymap {
    by_chord: HashMap<Keystroke, Vec<Bound>>,
}

/// Parses and resolves every binding into a lookup-ready keymap.
///
/// A bad row is a wrong compile-time table, so this panics with the action id,
/// key, and parsing or resolution failure instead of deferring the error to a
/// keypress.
pub fn assemble(bindings: &[Binding], registry: &Registry) -> Keymap {
    let mut by_chord: HashMap<Keystroke, Vec<Bound>> =
        HashMap::with_capacity(bindings.len());

    for binding in bindings {
        let action =
            registry
                .resolve(binding.action.as_ref())
                .unwrap_or_else(|| {
                    panic!(
                        "invalid binding action {:?}, key {:?}: action is not \
                     registered",
                        binding.action, binding.key
                    )
                });
        let chord = binding.key.parse::<Keystroke>().unwrap_or_else(|error| {
            panic!(
                "invalid binding action {:?}, key {:?}: invalid chord: {error}",
                binding.action, binding.key
            )
        });
        let when = binding.when.as_deref().map(|condition| {
            condition.parse::<Predicate>().unwrap_or_else(|error| {
                panic!(
                    "invalid binding action {:?}, key {:?}: invalid when \
                     {:?}: {error}",
                    binding.action, binding.key, condition
                )
            })
        });
        by_chord
            .entry(chord)
            .or_default()
            .push(Bound { action, when });
    }

    Keymap { by_chord }
}

impl Keymap {
    /// The action a keystroke publishes, or `None` meaning **the keystroke is
    /// not ours** and the caller sends it on to the PTY.
    ///
    /// The chord is hashed; the guard filters what that finds. Suppression is
    /// asked once, because it is a property of the chord and not of any one
    /// binding on it.
    pub fn lookup(&self, chord: Keystroke, ctx: Flags) -> Option<ActionId> {
        if suppressed_while_editing(chord, ctx) {
            return None;
        }
        self.by_chord
            .get(&chord)?
            .iter()
            .find(|bound| {
                bound
                    .when
                    .as_ref()
                    .is_none_or(|predicate| evaluate(predicate, ctx))
            })
            .map(|bound| bound.action)
    }
}

/// Whether the global text-editing rule suppresses this binding.
fn suppressed_while_editing(chord: Keystroke, ctx: Flags) -> bool {
    holds(ctx, EDITING_TEXT)
        && chord.modifiers == Modifiers::empty()
        && consumed_by_a_text_field(chord.code)
}

/// The keys a focused text input consumes: what inserts a character, what
/// deletes one, and what moves the caret.
///
/// Narrower than "every bare key" on purpose, and the palette is why. Its own
/// search field sets [`EDITING_TEXT`] while `Escape` closes the palette,
/// `Enter` runs the selection and the up and down arrows move through it, so
/// suppressing every bare key would make the palette impossible to operate.
/// Left and right are here and up and down are not, for the same reason: a
/// single-line field uses one pair and not the other.
fn consumed_by_a_text_field(code: Code) -> bool {
    matches!(
        code,
        Code::ArrowLeft
            | Code::ArrowRight
            | Code::Backspace
            | Code::Delete
            | Code::End
            | Code::Home
            | Code::Backquote
            | Code::Backslash
            | Code::BracketLeft
            | Code::BracketRight
            | Code::Comma
            | Code::Digit0
            | Code::Digit1
            | Code::Digit2
            | Code::Digit3
            | Code::Digit4
            | Code::Digit5
            | Code::Digit6
            | Code::Digit7
            | Code::Digit8
            | Code::Digit9
            | Code::Equal
            | Code::IntlBackslash
            | Code::IntlRo
            | Code::IntlYen
            | Code::KeyA
            | Code::KeyB
            | Code::KeyC
            | Code::KeyD
            | Code::KeyE
            | Code::KeyF
            | Code::KeyG
            | Code::KeyH
            | Code::KeyI
            | Code::KeyJ
            | Code::KeyK
            | Code::KeyL
            | Code::KeyM
            | Code::KeyN
            | Code::KeyO
            | Code::KeyP
            | Code::KeyQ
            | Code::KeyR
            | Code::KeyS
            | Code::KeyT
            | Code::KeyU
            | Code::KeyV
            | Code::KeyW
            | Code::KeyX
            | Code::KeyY
            | Code::KeyZ
            | Code::Minus
            | Code::Period
            | Code::Quote
            | Code::Semicolon
            | Code::Slash
            | Code::Space
            | Code::Numpad0
            | Code::Numpad1
            | Code::Numpad2
            | Code::Numpad3
            | Code::Numpad4
            | Code::Numpad5
            | Code::Numpad6
            | Code::Numpad7
            | Code::Numpad8
            | Code::Numpad9
            | Code::NumpadAdd
            | Code::NumpadComma
            | Code::NumpadDecimal
            | Code::NumpadDivide
            | Code::NumpadEqual
            | Code::NumpadHash
            | Code::NumpadMultiply
            | Code::NumpadParenLeft
            | Code::NumpadParenRight
            | Code::NumpadStar
            | Code::NumpadSubtract
    )
}
