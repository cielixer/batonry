//! Merging action tables into one, and refusing duplicate names.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

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

/// Why a merge failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MergeError {
    /// Two actions claimed one name. **Both sides are named**, because "there
    /// is a duplicate somewhere" is not an actionable message.
    DuplicateId {
        /// The name claimed twice.
        id: String,
        /// The source that claimed it first.
        first_source: String,
        /// Zero-based position within `first_source`.
        first_position: usize,
        /// The source that claimed it again.
        second_source: String,
        /// Zero-based position within `second_source`.
        second_position: usize,
    },
    /// More actions than an [`ActionId`] can address.
    TooManyActions {
        /// How many were offered.
        count: usize,
    },
}

impl fmt::Display for MergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId {
                id,
                first_source,
                first_position,
                second_source,
                second_position,
            } => write!(
                f,
                "duplicate action id {id:?}: {first_source} index \
                 {first_position} collides with {second_source} index \
                 {second_position}",
            ),
            Self::TooManyActions { count } => write!(
                f,
                "cannot register {count} actions: an ActionId is a u16, so {} \
                 is the ceiling",
                CEILING
            ),
        }
    }
}

impl std::error::Error for MergeError {}

/// How many actions an [`ActionId`] can address: every `u16` is a valid index.
const CEILING: usize = u16::MAX as usize + 1;

/// Merges sources into one registry, in the order given.
///
/// A duplicate is rejected rather than letting one silently overwrite the other
/// -- which is what a bare `HashMap::insert` would do, and which shows up much
/// later as an action that quietly stopped existing.
///
/// **Rejecting is the whole policy: there is no last-one-wins.** A table loaded
/// at runtime adds actions; it does not redefine one that already exists. Giving
/// it that power would mean a file could silently change what a built-in action
/// does, and the palette would still show the built-in label.
pub fn try_merge(sources: &[Source]) -> Result<Registry, MergeError> {
    let count = sources.iter().map(|s| s.actions.len()).sum();
    if count > CEILING {
        return Err(MergeError::TooManyActions { count });
    }

    let mut actions: Vec<Action> = Vec::with_capacity(count);
    let mut by_id: HashMap<String, ActionId> = HashMap::with_capacity(count);
    // Where each name came from, kept only so a duplicate can name both sides.
    // Borrowed: every string already lives in `sources` and outlives this call,
    // and the owned copies belong inside the error that needs them.
    let mut origin: HashMap<&str, (&str, usize)> =
        HashMap::with_capacity(count);

    for source in sources {
        for (position, row) in source.actions.iter().enumerate() {
            if let Some((first_source, first_position)) =
                origin.get(row.id.as_ref())
            {
                return Err(MergeError::DuplicateId {
                    id: row.id.to_string(),
                    first_source: (*first_source).to_owned(),
                    first_position: *first_position,
                    second_source: source.name.to_string(),
                    second_position: position,
                });
            }

            // The ceiling check above makes this conversion total. It is a
            // conversion rather than an `as` cast so that loosening that check
            // cannot silently start truncating.
            let next = u16::try_from(actions.len())
                .map_err(|_| MergeError::TooManyActions { count })?;
            by_id.insert(row.id.to_string(), ActionId(next));
            origin.insert(row.id.as_ref(), (source.name.as_ref(), position));
            actions.push(row.clone());
        }
    }

    Ok(Registry { actions, by_id })
}
