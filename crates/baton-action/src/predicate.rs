//! The condition language a keymap binding's `when` clause is written in.
//!
//! Deliberately small. It decides whether a keystroke is ours at all -- pass and
//! the action is published, fail and the keys go through the input router to the
//! PTY -- so every operator it gains becomes availability that somebody has to
//! debug. The grammar is five node kinds and four operators, and that is the
//! thing to hold, not a line count.

use std::fmt;
use std::str::FromStr;

use crate::context::{assignments, excludes};
use crate::{Flag, Flags, UnknownFlag, holds};

/// A parsed `when` clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Predicate {
    /// A context condition is true.
    Flag(Flag),
    /// The negation of a predicate.
    Not(Box<Predicate>),
    /// Both sides are true.
    And(Box<Predicate>, Box<Predicate>),
    /// Either side is true.
    Or(Box<Predicate>, Box<Predicate>),
    /// Both sides have the same truth value.
    Eq(Box<Predicate>, Box<Predicate>),
}

/// Why a `when` clause could not be parsed.
///
/// Every variant carries enough to fix the clause. "Parse error" alone would
/// leave someone reading a keymap file with nowhere to go.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredicateError {
    /// The clause is empty or only whitespace.
    Empty,
    /// A name that is not a context flag. Carries which.
    UnknownFlag(UnknownFlag),
    /// Something appeared where a predicate had to start.
    Unexpected {
        /// What was written there.
        found: String,
        /// Byte offset into the clause.
        at: usize,
    },
    /// A `(` with no `)`.
    Unclosed {
        /// Byte offset of the opening parenthesis's clause remainder.
        at: usize,
    },
    /// `a == b == c`, which is rejected rather than given a meaning.
    ChainedEq {
        /// Byte offset of the second `==`.
        at: usize,
    },
    /// A complete predicate, then more text.
    Trailing {
        /// What was left over.
        found: String,
        /// Byte offset into the clause.
        at: usize,
    },
}

impl fmt::Display for PredicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the clause is empty"),
            Self::UnknownFlag(e) => write!(f, "{e}"),
            Self::Unexpected { found, at } => {
                write!(f, "unexpected {found:?} at byte {at}")
            },
            Self::Unclosed { at } => {
                write!(f, "unclosed parenthesis at byte {at}")
            },
            Self::ChainedEq { at } => write!(
                f,
                "`==` does not chain; parenthesise one side (byte {at})"
            ),
            Self::Trailing { found, at } => {
                write!(f, "trailing {found:?} after the clause at byte {at}")
            },
        }
    }
}

impl std::error::Error for PredicateError {}

/// Whether `predicate` holds in `ctx`.
pub fn evaluate(predicate: &Predicate, ctx: Flags) -> bool {
    match predicate {
        Predicate::Flag(flag) => holds(ctx, (*flag).into()),
        Predicate::Not(inner) => !evaluate(inner, ctx),
        Predicate::And(left, right) => {
            evaluate(left, ctx) && evaluate(right, ctx)
        },
        Predicate::Or(left, right) => {
            evaluate(left, ctx) || evaluate(right, ctx)
        },
        Predicate::Eq(left, right) => {
            evaluate(left, ctx) == evaluate(right, ctx)
        },
    }
}

/// Characters that end an identifier. Everything else is part of a name, so an
/// unknown one is reported by [`Flag`] rather than by a second list here.
const BREAKS: &str = "!&|=() \t\r\n";

type Parsed = Result<Predicate, PredicateError>;

impl FromStr for Predicate {
    type Err = PredicateError;

    /// Parses `palette_open`, `!palette_open`, `pane_live && has_selection`,
    /// `(a || b) && c`, `a == b`.
    ///
    /// `!` binds tightest, then `==`, then `&&`, then `||`. `&&` and `||`
    /// associate to the left; `==` does not associate at all.
    fn from_str(input: &str) -> Parsed {
        if input.trim().is_empty() {
            return Err(PredicateError::Empty);
        }
        let mut p = Parser { rest: input, at: 0 };
        let predicate = p.or()?;
        p.spaces();
        if p.at < p.rest.len() {
            return Err(PredicateError::Trailing {
                found: p.word(),
                at: p.at,
            });
        }
        Ok(predicate)
    }
}

struct Parser<'a> {
    rest: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn or(&mut self) -> Parsed {
        self.binary("||", Self::and, Predicate::Or)
    }

    fn and(&mut self) -> Parsed {
        self.binary("&&", Self::eq, Predicate::And)
    }

    fn binary(
        &mut self,
        op: &str,
        next: fn(&mut Self) -> Parsed,
        build: fn(Box<Predicate>, Box<Predicate>) -> Predicate,
    ) -> Parsed {
        let mut left = next(self)?;
        while self.take(op) {
            left = build(Box::new(left), Box::new(next(self)?));
        }
        Ok(left)
    }

    fn eq(&mut self) -> Parsed {
        let left = self.unary()?;
        if !self.take("==") {
            return Ok(left);
        }
        let right = self.unary()?;
        self.spaces();
        // Rejected rather than given a meaning: `a == b == c` is far likelier
        // to be a mistake than an intent, and refusing keeps one spelling.
        if self.rest[self.at..].starts_with("==") {
            return Err(PredicateError::ChainedEq { at: self.at });
        }
        Ok(Predicate::Eq(Box::new(left), Box::new(right)))
    }

    fn unary(&mut self) -> Parsed {
        if self.take("!") {
            return Ok(Predicate::Not(Box::new(self.unary()?)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Parsed {
        if self.take("(") {
            let inner = self.or()?;
            if self.take(")") {
                return Ok(inner);
            }
            return Err(PredicateError::Unclosed { at: self.at });
        }
        let start = self.at;
        while let Some(c) = self.peek() {
            if BREAKS.contains(c) {
                break;
            }
            self.at += c.len_utf8();
        }
        if start == self.at {
            return Err(PredicateError::Unexpected {
                found: self.word(),
                at: self.at,
            });
        }
        self.rest[start..self.at]
            .parse::<Flag>()
            .map(Predicate::Flag)
            .map_err(PredicateError::UnknownFlag)
    }

    fn take(&mut self, op: &str) -> bool {
        self.spaces();
        let hit = self.rest[self.at..].starts_with(op);
        if hit {
            self.at += op.len();
        }
        hit
    }

    fn spaces(&mut self) {
        while let Some(c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.at += c.len_utf8();
        }
    }

    fn peek(&self) -> Option<char> {
        self.rest[self.at..].chars().next()
    }

    /// What sits at the cursor, for an error message: the next identifier, or
    /// the single character if the cursor is on one that ends identifiers.
    fn word(&self) -> String {
        let tail = &self.rest[self.at..];
        match tail.chars().next() {
            None => String::from("end of input"),
            Some(c) if BREAKS.contains(c) => c.to_string(),
            Some(_) => {
                let end =
                    tail.find(|c| BREAKS.contains(c)).unwrap_or(tail.len());
                tail[..end].to_owned()
            },
        }
    }
}

/// How tightly a node binds. Higher wins, and matches the grammar's order.
fn binding(predicate: &Predicate) -> u8 {
    match predicate {
        Predicate::Or(..) => 1,
        Predicate::And(..) => 2,
        Predicate::Eq(..) => 3,
        Predicate::Not(..) => 4,
        Predicate::Flag(..) => 5,
    }
}

impl fmt::Display for Predicate {
    /// The canonical spelling, so parsing what this prints yields an equal
    /// value. Parentheses appear exactly where precedence would otherwise
    /// change the meaning.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `least` is what a position accepts unparenthesised. The right side of
        // a left-associative operator demands one step tighter, which is what
        // keeps `a && (b && c)` from printing as `a && b && c`; `==` demands it
        // on both sides, because it does not associate.
        fn side(
            child: &Predicate,
            least: u8,
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result {
            if binding(child) < least {
                write!(f, "({child})")
            } else {
                write!(f, "{child}")
            }
        }

        match self {
            Predicate::Flag(flag) => write!(f, "{flag}"),
            Predicate::Not(inner) => {
                f.write_str("!")?;
                side(inner, 4, f)
            },
            Predicate::And(l, r) => {
                side(l, 2, f)?;
                f.write_str(" && ")?;
                side(r, 3, f)
            },
            Predicate::Or(l, r) => {
                side(l, 1, f)?;
                f.write_str(" || ")?;
                side(r, 2, f)
            },
            Predicate::Eq(l, r) => {
                side(l, 4, f)?;
                f.write_str(" == ")?;
                side(r, 4, f)
            },
        }
    }
}

/// A context in which both predicates hold, if any exists.
///
/// Sweeps every assignment of the flags -- all `2^17` of them, which a test
/// pays in milliseconds -- skipping assignments that violate `exclusive`
/// ("at most one of these holds", see [`Flags::EXCLUSIVE`](crate::Flags::EXCLUSIVE)), and returns the
/// first satisfying context. Exhaustive on purpose: the honest options were
/// this or a solver, and the sweep is the one that is easy to trust.
///
/// Analysis over the language, not part of it: the grammar stays five node
/// kinds and four operators.
pub fn satisfiable_together(
    a: &Predicate,
    b: &Predicate,
    exclusive: &[Flags],
) -> Option<Flags> {
    assignments()
        .filter(|ctx| !excludes(*ctx, exclusive))
        .find(|ctx| evaluate(a, *ctx) && evaluate(b, *ctx))
}
