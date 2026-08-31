//! The closed vocabulary a `when` clause names, and the set that answers it.

use std::str::FromStr;

use baton_action::{Flag, Flags, UnknownFlag, combine, holds};

mod common;

use common::EVERY_FLAG as EVERY;

#[test]
fn every_name_parses_to_itself_and_prints_back() {
    for (flag, spelling) in EVERY {
        let parsed = Flag::from_str(spelling)
            .unwrap_or_else(|e| panic!("{spelling} did not parse: {e}"));
        let set: Flags = parsed.into();
        assert_eq!(set, flag, "{spelling} parsed to the wrong flag");
        assert_eq!(
            parsed.to_string(),
            spelling,
            "{spelling} printed back wrongly"
        );
    }
}

/// A name outside the vocabulary is rejected, and the error says which.
///
/// This is the whole reason the vocabulary is closed: a misspelling that parsed
/// would become a clause that is quietly false, disabling a key with no
/// diagnosis.
#[test]
fn an_unknown_name_is_rejected_and_named() {
    for name in [
        "pane_focussed", // a plausible misspelling
        "paneFocused",   // camelCase is not the spelling
        "PANE_FOCUSED",  // neither is the constant's own name
        "pane_focused ", // nor is trailing space
        "",              // nor is nothing
        "true",          // there are no literals in this vocabulary
        "editing",       // a prefix of a real name is still not one
    ] {
        let err = Flag::from_str(name).expect_err("must not parse");
        assert_eq!(
            err,
            UnknownFlag {
                name: name.to_owned()
            }
        );
        assert!(
            err.to_string().contains(name),
            "the error does not say what was written: {err}"
        );
    }
}

/// Parsing a name yields exactly one flag, which is what makes it usable as a
/// clause's atom. A set with two bits would be a clause the grammar cannot
/// spell.
#[test]
fn a_parsed_name_holds_exactly_one_flag() {
    for (flag, spelling) in EVERY {
        let others = EVERY
            .iter()
            .filter(|(f, _)| *f != flag)
            .fold(Flags::NONE, |set, (f, _)| combine(set, *f));
        assert!(
            !holds(others, flag),
            "{spelling} shares a bit with another flag"
        );
    }
}

/// **A clause's leaf holds exactly one condition.**
///
/// This is what the round-trip contract rests on. A leaf holding two bits would
/// print as `a && b`, which the formatter emits at leaf precedence, so
/// `Not(leaf)` printed `!a && b` and re-parsed as `(!a) && b` -- a different
/// clause. A leaf holding none printed nothing at all and re-parsed as an
/// error.
///
/// Neither is reachable now, and not because a constructor rejects them:
/// `FromStr` is the only way in and its field is private, so a leaf is one
/// condition by construction. What is left to check is that construction, which
/// is what this does -- every name yields a value holding that flag and no
/// other.
#[test]
fn a_leaf_holds_exactly_the_one_condition_it_names() {
    for (flag, spelling) in EVERY {
        let leaf: Flag = spelling
            .parse()
            .unwrap_or_else(|e| panic!("{spelling} did not parse: {e}"));
        let set: Flags = leaf.into();

        assert!(holds(set, flag), "{spelling} lost its own bit");
        for (other, other_spelling) in EVERY {
            if other == flag {
                continue;
            }
            assert!(
                !holds(set, other),
                "{spelling} also holds {other_spelling}"
            );
        }
    }
}

/// Two flags never share a bit, so `holds` and `combine` really are a set.
///
/// This pins the bit assignment, which the macro counts by position: an
/// aliasing bit would make one condition answer for another with nothing
/// failing to compile.
#[test]
fn each_flag_occupies_its_own_bit() {
    for (i, (flag, spelling)) in EVERY.iter().enumerate() {
        let only = combine(Flags::NONE, *flag);
        assert!(holds(only, *flag), "{spelling} does not hold itself");

        for (j, (other, other_spelling)) in EVERY.iter().enumerate() {
            if i == j {
                continue;
            }
            assert!(
                !holds(only, *other),
                "a set holding only {spelling} also reports {other_spelling}"
            );
        }
    }
}

#[test]
fn the_empty_set_holds_nothing_and_accumulates() {
    for (flag, spelling) in EVERY {
        assert!(
            !holds(Flags::NONE, flag),
            "the empty set reports {spelling}"
        );
    }

    let full = EVERY
        .iter()
        .fold(Flags::NONE, |set, (f, _)| combine(set, *f));
    for (flag, spelling) in EVERY {
        assert!(holds(full, flag), "a full set lost {spelling}");
    }
}

/// `combine` returns a value and does not mutate, so a set can be built in a
/// `const` and shared.
#[test]
fn combine_is_a_value_and_leaves_its_input_alone() {
    const BASE: Flags = combine(Flags::NONE, Flags::PALETTE_OPEN);
    const BOTH: Flags = combine(BASE, Flags::EDITING_TEXT);

    assert!(holds(BASE, Flags::PALETTE_OPEN));
    assert!(
        !holds(BASE, Flags::EDITING_TEXT),
        "combine mutated its input"
    );
    assert!(
        holds(BOTH, Flags::PALETTE_OPEN) && holds(BOTH, Flags::EDITING_TEXT)
    );

    // Adding a flag twice is the same value: this is a set, not a counter.
    assert_eq!(combine(BOTH, Flags::EDITING_TEXT), BOTH);
}

/// **`Flags::NONE` is a value to build a set from, never one to ask about.** Every set
/// contains it, so `holds(anything, Flags::NONE)` is `true` -- including for a set that
/// holds a great deal. The question "does this hold nothing?" is `== Flags::NONE`.
///
/// Pinned so the obvious "fix" is not made: special-casing the empty set inside
/// `holds` would make it something other than containment.
#[test]
fn the_empty_set_is_a_value_and_not_a_query() {
    let busy = combine(
        combine(Flags::NONE, Flags::PALETTE_OPEN),
        Flags::EDITING_TEXT,
    );

    assert!(holds(busy, Flags::NONE), "every set contains the empty set");
    assert!(holds(Flags::NONE, Flags::NONE));
    assert_ne!(busy, Flags::NONE, "and containment is not the way to ask");
    assert_eq!(Flags::NONE, Flags::NONE);
}

/// `Default` is the named empty set, not incidentally the integer's zero.
///
/// The macro generates `default()` as `Flags::NONE`, so the hazard note on the
/// constant covers the anonymous spelling too. Pinned here because a derive
/// would satisfy this equality only by coincidence of `u32::default()`.
#[test]
fn the_default_is_the_named_empty_set() {
    assert_eq!(Flags::default(), Flags::NONE);
}

/// `holds` asks about every flag in its second argument, so a set answers for a
/// combination as readily as for one flag.
#[test]
fn holds_asks_about_all_of_what_it_is_given() {
    let typing = combine(Flags::PALETTE_OPEN, Flags::EDITING_TEXT);
    let both = combine(Flags::PALETTE_OPEN, Flags::EDITING_TEXT);

    assert!(holds(typing, both));
    assert!(
        !holds(combine(Flags::NONE, Flags::PALETTE_OPEN), both),
        "one of two is not both"
    );
}
