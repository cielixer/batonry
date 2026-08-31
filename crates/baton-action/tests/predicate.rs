//! The `when` clause language: what it parses, what it refuses, and its size.

use std::str::FromStr;

use baton_action::{Flags, Predicate, PredicateError, combine, evaluate};

fn parse(s: &str) -> Predicate {
    Predicate::from_str(s)
        .unwrap_or_else(|e| panic!("{s:?} did not parse: {e}"))
}

fn ctx(flags: &[Flags]) -> Flags {
    flags
        .iter()
        .fold(Flags::NONE, |set, flag| combine(set, *flag))
}

/// The four operators mean what they say, against a context built by hand.
#[test]
fn the_operators_evaluate() {
    let open = ctx(&[Flags::PALETTE_OPEN]);
    let both = ctx(&[Flags::PALETTE_OPEN, Flags::EDITING_TEXT]);

    assert!(evaluate(&parse("palette_open"), open));
    assert!(!evaluate(&parse("palette_open"), Flags::NONE));
    assert!(evaluate(&parse("!palette_open"), Flags::NONE));

    assert!(evaluate(&parse("palette_open && editing_text"), both));
    assert!(!evaluate(&parse("palette_open && editing_text"), open));

    assert!(evaluate(&parse("palette_open || editing_text"), open));
    assert!(!evaluate(
        &parse("palette_open || editing_text"),
        Flags::NONE
    ));

    // `==` is truth-value equality, so two false operands are equal.
    assert!(evaluate(&parse("palette_open == editing_text"), both));
    assert!(evaluate(
        &parse("palette_open == editing_text"),
        Flags::NONE
    ));
    assert!(!evaluate(&parse("palette_open == editing_text"), open));
}

/// Precedence is `!` then `==` then `&&` then `||`, and it is checked by
/// evaluation rather than by inspecting the tree: a wrong tree that happens to
/// evaluate correctly is not a bug worth failing on.
#[test]
fn precedence_binds_the_way_the_grammar_says() {
    let a = Flags::PALETTE_OPEN;
    let b = Flags::EDITING_TEXT;
    let c = Flags::PANE_FOCUSED;

    // `a || b && c` is `a || (b && c)`: true on `a` alone.
    let clause = parse("palette_open || editing_text && pane_focused");
    assert!(evaluate(&clause, ctx(&[a])));
    assert!(!evaluate(&clause, ctx(&[b])));

    // Parentheses change it: `(a || b) && c` is false without `c`.
    let grouped = parse("(palette_open || editing_text) && pane_focused");
    assert!(!evaluate(&grouped, ctx(&[a])));
    assert!(evaluate(&grouped, ctx(&[a, c])));

    // `!` binds tighter than `&&`.
    let negated = parse("!palette_open && pane_focused");
    assert!(evaluate(&negated, ctx(&[c])));
    assert!(!evaluate(&negated, ctx(&[a, c])));

    // `==` binds tighter than `&&`: `a == b && c` is `(a == b) && c`.
    let mixed = parse("palette_open == editing_text && pane_focused");
    assert!(evaluate(&mixed, ctx(&[a, b, c])));
    assert!(evaluate(&mixed, ctx(&[c])), "two false operands are equal");
    assert!(!evaluate(&mixed, ctx(&[a, c])));
}

/// Printing and parsing are inverses, which is what says the tree the parser
/// built is the tree the grammar describes.
#[test]
fn printing_round_trips_through_parsing() {
    for clause in [
        "palette_open",
        "!palette_open",
        "!!palette_open",
        "palette_open && editing_text",
        "palette_open || editing_text",
        "palette_open == editing_text",
        "palette_open || editing_text && pane_focused",
        "(palette_open || editing_text) && pane_focused",
        "palette_open && (editing_text || pane_focused)",
        "!(palette_open && editing_text)",
        "(palette_open == editing_text) == pane_focused",
        "palette_open && editing_text && pane_focused",
        "palette_open && (editing_text && pane_focused)",
    ] {
        let tree = parse(clause);
        let printed = tree.to_string();
        assert_eq!(printed, clause, "{clause:?} did not print back");
        assert_eq!(parse(&printed), tree, "{printed:?} did not re-parse equal");
    }
}

/// The clause that used to break the round-trip, written the way it has to be.
///
/// `Not` around a leaf holding two bits printed `!a && b` and came back as
/// `(!a) && b`. That leaf is now unbuildable -- `Predicate`'s variants are
/// public, but a `Flag` comes only from parsing a single name -- so the same
/// intent has to be spelled with parentheses, and that spelling round-trips.
#[test]
fn a_negated_conjunction_keeps_its_parentheses() {
    let clause = parse("!(palette_open && editing_text)");
    assert_eq!(clause.to_string(), "!(palette_open && editing_text)");
    assert_eq!(parse(&clause.to_string()), clause);

    // And the shape it must not be confused with prints differently.
    let other = parse("!palette_open && editing_text");
    assert_ne!(other, clause);
    assert_eq!(other.to_string(), "!palette_open && editing_text");
}

/// Whitespace between tokens is insignificant, including none at all.
#[test]
fn spacing_does_not_change_the_tree() {
    let canonical = parse("palette_open && !editing_text");
    for spelling in [
        "palette_open&&!editing_text",
        "  palette_open   &&   !  editing_text  ",
        "palette_open\t&&\n!editing_text",
    ] {
        assert_eq!(
            parse(spelling),
            canonical,
            "{spelling:?} parsed differently"
        );
    }
}

/// Everything that is not a clause is refused, and the error says what and
/// where. A clause that parsed but meant nothing would disable a key with no
/// diagnosis, which is the failure this language exists to prevent.
#[test]
fn everything_that_is_not_a_clause_is_refused() {
    for (clause, expected) in [
        ("", "empty"),
        ("   ", "empty"),
        ("pane_focussed", "unknown"), // a plausible misspelling
        ("PaletteOpen", "unknown"),   // not the spelling
        ("true", "unknown"),          // there are no literals
        ("palette_open &&", "unexpected"), // an operand is missing
        ("&& palette_open", "unexpected"),
        ("palette_open ||", "unexpected"),
        ("!", "unexpected"),
        ("(palette_open", "unclosed"),
        // `(` alone is a missing operand before it is a missing bracket, and
        // the parser reports the first thing wrong rather than the last.
        ("(", "unexpected"),
        ("()", "unexpected"),
        ("palette_open)", "trailing"),
        ("palette_open editing_text", "trailing"),
        ("palette_open & editing_text", "trailing"), // one `&` is not `&&`
        ("palette_open == editing_text == pane_focused", "chained"),
    ] {
        let err = Predicate::from_str(clause)
            .map(|p| p.to_string())
            .expect_err(&format!("{clause:?} must not parse"));
        let text = err.to_string();
        let kind = match err {
            PredicateError::Empty => "empty",
            PredicateError::UnknownFlag(_) => "unknown",
            PredicateError::Unexpected { .. } => "unexpected",
            PredicateError::Unclosed { .. } => "unclosed",
            PredicateError::ChainedEq { .. } => "chained",
            PredicateError::Trailing { .. } => "trailing",
        };
        assert_eq!(kind, expected, "{clause:?} gave the wrong error: {text}");
        assert!(!text.is_empty(), "{clause:?} produced an empty message");
    }
}

/// An unknown identifier names itself, so a keymap file can be corrected.
#[test]
fn an_unknown_identifier_is_quoted_back() {
    let err = Predicate::from_str("pane_focussed && palette_open")
        .expect_err("a misspelling must not parse");
    assert!(
        err.to_string().contains("pane_focussed"),
        "the error does not say which name: {err}"
    );
}
