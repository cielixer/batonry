//! The palette is a search UI over the registry (A10). Nucleo does the
//! matching; this module never implements a fuzzy matcher.

use baton_action::{Channels, Registry, reachable_from};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// A palette result with its registry position and fuzzy-match score.
pub(crate) struct Hit {
    pub(crate) index: usize,
    pub(crate) score: u32,
}

/// Ranks labels by fuzzy-match score, preserving registry order for ties.
pub(crate) fn rank<'a>(
    query: &str,
    labels: impl Iterator<Item = &'a str>,
) -> Vec<Hit> {
    if query.is_empty() {
        return labels
            .enumerate()
            .map(|(index, _)| Hit { index, score: 0 })
            .collect();
    }

    let pattern =
        Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buffer = Vec::new();
    let mut hits: Vec<Hit> = labels
        .enumerate()
        .filter_map(|(index, label)| {
            pattern
                .score(Utf32Str::new(label, &mut buffer), &mut matcher)
                .map(|score| Hit { index, score })
        })
        .collect();

    hits.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.index.cmp(&right.index))
    });
    hits
}

/// Whether a palette result can execute in the current stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Availability {
    /// The action is wired for execution.
    Ready,
    /// The action is registered but execution is not wired yet.
    Unavailable(&'static str),
}

/// The registry data a palette row needs to render and dispatch.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PaletteResult<'a> {
    pub(crate) id: &'a str,
    pub(crate) label: &'a str,
    pub(crate) availability: Availability,
}

/// Ranks every registered action and describes its stage-1 availability.
pub(crate) fn palette_results<'a>(
    registry: &'a Registry,
    query: &str,
) -> Vec<PaletteResult<'a>> {
    // Only what the palette surface can reach: a key-only action (the
    // palette's own navigation, select-all) is not a palette entry --
    // that is what the channel bits are for.
    let actions: Vec<_> = registry
        .iter()
        .filter(|(_, action)| {
            reachable_from(action.channels, Channels::PALETTE)
        })
        .collect();
    rank(
        query,
        actions.iter().map(|(_, action)| action.label.as_ref()),
    )
    .into_iter()
    .filter_map(|hit| {
        let (_, action) = actions.get(hit.index).copied()?;
        let availability = if action.id.starts_with("term.") {
            // The action-wiring ticket will connect terminal actions.
            Availability::Unavailable("Waits for the action-wiring ticket")
        } else {
            Availability::Ready
        };
        Some(PaletteResult {
            id: action.id.as_ref(),
            label: action.label.as_ref(),
            availability,
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELS: [&str; 5] =
        ["Quit", "Command Palette", "Close Palette", "Copy", "Paste"];

    /// An empty query is the palette at rest: everything, registry order.
    #[test]
    fn an_empty_query_lists_everything_in_registry_order() {
        let hits = rank("", LABELS.into_iter());
        let order: Vec<usize> = hits.iter().map(|h| h.index).collect();
        assert_eq!(order, [0, 1, 2, 3, 4]);
    }

    /// Fuzzy means subsequence, case-insensitively: "cmdp" finds the
    /// palette, and a better match outranks a looser one.
    #[test]
    fn fuzzy_matching_ranks_the_tighter_match_first() {
        let hits = rank("pal", LABELS.into_iter());
        let found: Vec<usize> = hits.iter().map(|h| h.index).collect();
        assert!(found.contains(&1) && found.contains(&2));
        assert!(!found.contains(&0), "Quit does not contain p-a-l");

        let exactish = rank("Copy", LABELS.into_iter());
        assert_eq!(exactish[0].index, 3, "the literal match ranks first");
    }

    /// Ties keep registry order, so ranking is deterministic run to run.
    #[test]
    fn equal_scores_keep_registry_order() {
        let hits = rank("p", ["Print", "Print"].into_iter());
        let order: Vec<usize> = hits.iter().map(|h| h.index).collect();
        assert_eq!(order, [0, 1]);
    }
}
