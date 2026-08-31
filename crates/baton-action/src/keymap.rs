//! The assembled keymap and the one place where bindings are looked up.

use std::collections::HashMap;

use keyboard_types::{Code, Modifiers};

use std::borrow::Cow;

use crate::{
    ActionId, Flags, Keystroke, Predicate, Registry, evaluate, holds,
    satisfiable_together,
};

/// One way to reach one action.
///
/// `when` stays opaque here. It is a guard on the *binding*, so what it decides
/// is whether a keystroke becomes an action at all or falls through to whatever
/// the input router is pointed at -- not whether an action is greyed out.
/// Parsing and evaluating it belongs to whoever owns the context it names.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Binding {
    /// The id of the action this reaches. Joins to a registry.
    pub action: Cow<'static, str>,
    /// The chord's canonical ASCII spelling, parseable by [`crate::Keystroke`].
    pub key: Cow<'static, str>,
    /// An opaque condition. Empty means the binding always applies.
    pub when: Option<Cow<'static, str>>,
}

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

/// Two bindings on one chord whose conditions can hold together.
///
/// Not resolved by priority, ever: which of the two fires would be decided by
/// table order, which nobody can predict from a keymap file. A conflict is a
/// bug in the keymap and the fix is to change a key -- `⌘D` once carried both
/// "favourite host" and "split vertically", and `host_selected` with
/// `pane_focused` really are true together.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conflict {
    /// The chord both bindings claim.
    pub chord: Keystroke,
    /// The first claimant, in table order.
    pub first: ActionId,
    /// The second claimant.
    pub second: ActionId,
    /// An assignment under which both fire, respecting [`Flags::EXCLUSIVE`].
    pub context: Flags,
}

impl Keymap {
    /// Every pair of same-chord bindings that can both fire.
    ///
    /// Satisfiability of the two conditions together is decided by exhaustive
    /// sweep under the exclusivity assumptions of [`Flags::EXCLUSIVE`]; a binding
    /// with no condition is satisfied everywhere, so two of those on one
    /// chord conflict in the empty context. Chords are visited in canonical
    /// spelling order so the output is stable between runs.
    ///
    /// The editing-suppression rule is deliberately not modelled: a pair only
    /// satisfiable while its chord is suppressed still reports, which
    /// over-approximates in the safe direction.
    ///
    /// Empty means the table is sound. This is what the keymap test asserts,
    /// and CI runs that test on every pull request.
    pub fn conflicts(&self) -> Vec<Conflict> {
        let mut chords: Vec<&Keystroke> = self.by_chord.keys().collect();
        chords.sort_by_key(|chord| chord.to_string());

        let mut found = Vec::new();
        for chord in chords {
            let bound = &self.by_chord[chord];
            for i in 0..bound.len() {
                for j in (i + 1)..bound.len() {
                    let context = match (&bound[i].when, &bound[j].when) {
                        (None, None) => Some(Flags::NONE),
                        (Some(p), None) | (None, Some(p)) => {
                            satisfiable_together(p, p, Flags::EXCLUSIVE)
                        },
                        (Some(a), Some(b)) => {
                            satisfiable_together(a, b, Flags::EXCLUSIVE)
                        },
                    };
                    if let Some(context) = context {
                        found.push(Conflict {
                            chord: *chord,
                            first: bound[i].action,
                            second: bound[j].action,
                            context,
                        });
                    }
                }
            }
        }
        found
    }
}

/// Whether the global text-editing rule suppresses this binding.
fn suppressed_while_editing(chord: Keystroke, ctx: Flags) -> bool {
    holds(ctx, Flags::EDITING_TEXT)
        && chord.modifiers == Modifiers::empty()
        && consumed_by_a_text_field(chord.code)
}

/// The keys a focused text input consumes: what inserts a character, what
/// deletes one, and what moves the caret.
///
/// Narrower than "every bare key" on purpose, and the palette is why. Its own
/// search field sets [`Flags::EDITING_TEXT`] while `Escape` closes the palette,
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
