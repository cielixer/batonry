//! Merging action tables into one, and refusing duplicate names.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::action::{Action, Channels, reachable_from};

/// A registry-issued index.
///
/// **Not an id.** A name is a string that crosses process boundaries; this is a
/// position handed out at boot, so resolving it is a bounds check rather than a
/// lookup.
///
/// **It means nothing outside the [`Registry`] that issued it**, not merely
/// outside the run. Two registries in one process hand out overlapping indices,
/// so an index from one silently addresses an unrelated row in the other. The
/// type does not carry its origin; a second registry is the hazard to design
/// against.
///
/// The field and the accessor are both private: a caller holding the integer is
/// a caller that can compute one the registry never issued.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ActionId(u16);

/// The position an [`ActionId`] stands for. Crate-private on purpose: the whole
/// point of the opaque handle is that nothing outside computes one.
const fn index(id: ActionId) -> usize {
    id.0 as usize
}

/// One contribution to the action table, with a name.
///
/// `actions` is a `Cow`, which is what lets a table that arrives at runtime be
/// the same kind of thing as the built-in one: the constant borrows a slice, a
/// loaded table owns a `Vec`, and the merge does not care which it is given.
///
/// The name is not decoration. With anonymous slices a duplicate could only be
/// reported as "an insert failed", which tells whoever hits it nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Source {
    /// Used in diagnostics. Usually a crate name, or the file a table came from.
    pub name: Cow<'static, str>,
    /// What this source contributes.
    pub actions: Cow<'static, [Action]>,
}

/// The merged actions in contribution order, plus a name index.
///
/// They stay contiguous so iteration is a walk and an [`ActionId`] is an index.
/// The one type here with methods, because it is the one that owns an invariant:
/// the map and the slice have to agree.
#[derive(Debug, Default)]
pub struct Registry {
    actions: Vec<Action>,
    by_id: HashMap<String, ActionId>,
}

impl Registry {
    /// The action an issued index stands for. A bounds check.
    pub fn get(&self, id: ActionId) -> Option<&Action> {
        self.actions.get(index(id))
    }

    /// Resolves a permanent name to an issued index. **Not a scan.**
    pub fn resolve(&self, name: &str) -> Option<ActionId> {
        self.by_id.get(name).copied()
    }

    /// Every action in contribution order, each with the id it was issued.
    ///
    /// **The id comes with it, and that is the point.** Handing out the slice
    /// alone would leave a caller holding an [`Action`] and no way to name it,
    /// since the index accessor is private -- it would have to [`resolve`] the
    /// id string it had just walked past.
    ///
    /// [`resolve`]: Registry::resolve
    pub fn iter(&self) -> impl Iterator<Item = (ActionId, &Action)> {
        self.actions
            .iter()
            .enumerate()
            .map(|(i, a)| (ActionId(i as u16), a))
    }

    /// The actions a surface can invoke, each with its id.
    ///
    /// What a palette is: this filter over the registry, and nothing else. It is
    /// here rather than at the call site because [`Channels`] arithmetic has one
    /// hazard -- the empty set is contained by everything -- and one place to get
    /// it right is better than one per surface.
    pub fn reachable(
        &self,
        surface: Channels,
    ) -> impl Iterator<Item = (ActionId, &Action)> {
        self.iter()
            .filter(move |(_, a)| reachable_from(a.channels, surface))
    }

    /// How many actions are registered.
    pub fn count(&self) -> usize {
        self.actions.len()
    }
}

/// How many actions an [`ActionId`] can address: every `u16` is a valid index.
const CEILING: usize = u16::MAX as usize + 1;

/// Merges sources into one registry, in the order given.
///
/// **An id is unique, and a duplicate is a bug rather than a condition**, so this
/// panics instead of returning. Every source is a compile-time constant, which
/// means two rows claiming one name is a wrong table and there is nothing a
/// caller could do with the news. A test in this crate reaches it first. When a
/// keymap file starts contributing rows, its loader is where they get validated,
/// and that is the better place: it can name a line rather than a source.
///
/// **There is no last-one-wins.** A later source adds actions; it does not
/// redefine one that exists. An `Action` row is a description and not behaviour
/// -- the behaviour is whatever matches on the resolved [`ActionId`] -- so
/// overwriting a row could never change what an action does, only what it claims
/// to do. A palette entry that copies while calling itself something else is
/// worse than a refusal.
pub fn merge(sources: &[Source]) -> Registry {
    let count = sources.iter().map(|s| s.actions.len()).sum();
    let mut actions: Vec<Action> = Vec::with_capacity(count);
    let mut by_id: HashMap<String, ActionId> = HashMap::with_capacity(count);
    // Where each name came from, kept only so a duplicate can name both sides.
    // Borrowed: every string already lives in `sources` and outlives this call.
    let mut origin: HashMap<&str, (&str, usize)> =
        HashMap::with_capacity(count);

    for source in sources {
        for (position, row) in source.actions.iter().enumerate() {
            if let Some((first_source, first_position)) =
                origin.get(row.id.as_ref())
            {
                panic!(
                    "duplicate action id {:?}: {} index {} collides with {} \
                     index {}",
                    row.id, first_source, first_position, source.name, position
                );
            }

            // A conversion rather than an `as` cast: `as` would truncate in
            // silence, and an index that wraps to 0 aliases a real action.
            let next = u16::try_from(actions.len()).unwrap_or_else(|_| {
                panic!(
                    "cannot register {count} actions: an ActionId is a u16, \
                     so {CEILING} is the ceiling"
                )
            });
            by_id.insert(row.id.to_string(), ActionId(next));
            origin.insert(row.id.as_ref(), (source.name.as_ref(), position));
            actions.push(row.clone());
        }
    }

    Registry { actions, by_id }
}
