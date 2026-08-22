//! Merging named action sources into one table.

use std::collections::HashMap;
use std::fmt;

use crate::action::{ActionId, ActionMeta};

/// A named set of action rows contributed by one crate.
///
/// The name is not decoration. With anonymous `&[ActionMeta]` slices a
/// duplicate id could only be reported as "a hash-map insert failed", which
/// tells whoever hits it nothing about where to look.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionSource {
    /// Used in diagnostics. The contributing crate's name.
    pub name: &'static str,
    /// The rows this source contributes.
    pub actions: &'static [ActionMeta],
}

/// The merged action table and its lookup index.
///
/// Metadata stays contiguous so iteration is a walk and an [`ActionId`] is an
/// index. The map is only an index into that slice -- it does not own or copy
/// the rows.
#[derive(Debug)]
pub struct Registry {
    actions: Vec<ActionMeta>,
    by_id: HashMap<&'static str, ActionId>,
}

impl Registry {
    /// Metadata for an id the registry issued. O(1), a bounds check.
    pub fn get(&self, id: ActionId) -> Option<&ActionMeta> {
        self.actions.get(id.index())
    }

    /// Resolves a stable id string. **Not a scan of the action slice.**
    pub fn id(&self, name: &str) -> Option<ActionId> {
        self.by_id.get(name).copied()
    }

    /// Every row, in registry order.
    pub fn actions(&self) -> &[ActionMeta] {
        &self.actions
    }

    /// How many actions are registered.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Whether nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Iterates rows in registry order.
    pub fn iter(&self) -> impl Iterator<Item = &ActionMeta> {
        self.actions.iter()
    }
}

/// Why a merge failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    /// Two rows claimed one id. **Both sides are named**, because "there is a
    /// duplicate somewhere" is not an actionable message.
    DuplicateId {
        /// The id claimed twice.
        id: &'static str,
        /// The source that claimed it first.
        first_source: &'static str,
        /// Zero-based row position within `first_source`.
        first_position: usize,
        /// The source that claimed it again.
        second_source: &'static str,
        /// Zero-based row position within `second_source`.
        second_position: usize,
    },
    /// More rows than an [`ActionId`] can address.
    TooManyActions {
        /// How many rows were offered.
        count: usize,
    },
}

impl fmt::Display for RegistryError {
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
                "duplicate action id {id:?}: {first_source} row \
                 {first_position} collides with {second_source} row \
                 {second_position}",
            ),
            Self::TooManyActions { count } => write!(
                f,
                "cannot register {count} actions: an ActionId is a u16, so \
                 {} is the ceiling",
                ACTION_CEILING
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// How many actions an [`ActionId`] can address: every `u16` is a valid index.
const ACTION_CEILING: usize = u16::MAX as usize + 1;

/// Merges named sources into one registry.
///
/// A duplicate is rejected rather than letting one row silently overwrite the
/// other in the index -- which is what a bare `HashMap::insert` would do, and
/// which would show up much later as an action that quietly stopped existing.
pub fn try_merge(sources: &[ActionSource]) -> Result<Registry, RegistryError> {
    let count = sources.iter().map(|s| s.actions.len()).sum();
    if count > ACTION_CEILING {
        return Err(RegistryError::TooManyActions { count });
    }

    let mut actions = Vec::with_capacity(count);
    let mut by_id = HashMap::with_capacity(count);
    // Where each id came from, kept only so a duplicate can name both sides.
    let mut origin: HashMap<&'static str, (&'static str, usize)> =
        HashMap::with_capacity(count);

    for source in sources {
        for (position, action) in source.actions.iter().enumerate() {
            if let Some(&(first_source, first_position)) = origin.get(action.id)
            {
                return Err(RegistryError::DuplicateId {
                    id: action.id,
                    first_source,
                    first_position,
                    second_source: source.name,
                    second_position: position,
                });
            }

            // The ceiling check above makes this conversion total. It is
            // written as a conversion rather than an `as` cast so that
            // loosening that check cannot silently start truncating.
            let index = u16::try_from(actions.len())
                .map_err(|_| RegistryError::TooManyActions { count })?;
            actions.push(*action);
            by_id.insert(action.id, ActionId::from_index(index));
            origin.insert(action.id, (source.name, position));
        }
    }

    Ok(Registry { actions, by_id })
}
