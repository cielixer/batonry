//! One declaration form for the crate's bitsets.

/// Declares a bitset's constants and its two operations.
///
/// The type is supplied rather than generated, so it keeps its own doc comment
/// and its own derives. What this removes is the numbering and the arithmetic:
/// a bit is where its constant sits in the list, so inserting one in the middle
/// renumbers nothing by hand and no two can be given the same shift by mistake;
/// containment and union are written once here instead of once per bitset.
///
/// **The operations are named by the caller**, because the question each answers
/// is not the same in both places. `Channels` asks whether a surface can reach
/// an action; `Flags` asks whether a condition holds. Naming them for the
/// arithmetic instead would lose that at every call site, and free functions
/// cannot share a name at the crate root anyway.
///
/// **Generated per type rather than written once over a trait**, which is not a
/// choice: both are `const fn` because the built-in tables are `const`, and
/// const traits are not supported on stable Rust -- calling a trait method from
/// a `const fn` is `error[E0015]`.
///
/// With a fourth name it also emits a table pairing each constant with the
/// spelling it is written as, for a set whose members are parsed or printed.
/// One table, so a parser and a formatter cannot drift apart and neither needs
/// an arm for a member it was never given.
///
/// ```ignore
/// bitset!(Channels, reachable_from, union: PALETTE, CLICK,);
/// bitset!(Flags, holds, combine, NAMES: PANE_FOCUSED = "pane_focused",);
/// ```
macro_rules! bitset {
    ($ty:ident, $contains:ident, $union:ident, $names:ident:
     $($(#[$doc:meta])* $name:ident = $spelling:literal,)*) => {
        bitset!(@ops $ty, $contains, $union);
        bitset!(@bit $ty, 0u32; $($(#[$doc])* $name,)*);

        /// Every member with its canonical spelling, in bit order.
        const $names: &[($ty, &str)] = &[$(($name, $spelling),)*];
    };
    ($ty:ident, $contains:ident, $union:ident:
     $($(#[$doc:meta])* $name:ident,)*) => {
        bitset!(@ops $ty, $contains, $union);
        bitset!(@bit $ty, 0u32; $($(#[$doc])* $name,)*);
    };
    (@ops $ty:ident, $contains:ident, $union:ident) => {
        #[doc = concat!("Whether `set` includes every member of `wanted`.")]
        ///
        /// The empty set is included by every set, so this answers `true` for it
        /// whatever `set` holds. That makes the empty set a value to build with
        /// and never one to ask about; equality is how to ask.
        pub const fn $contains(set: $ty, wanted: $ty) -> bool {
            set.0 & wanted.0 == wanted.0
        }

        #[doc = concat!("Both sets together.")]
        ///
        /// A `const fn` and not `BitOr`, because operator traits are not
        /// callable in the `const` context the built-in tables are built in.
        pub const fn $union(set: $ty, extra: $ty) -> $ty {
            $ty(set.0 | extra.0)
        }
    };
    (@bit $ty:ident, $n:expr; $(#[$doc:meta])* $name:ident, $($rest:tt)*) => {
        $(#[$doc])*
        pub const $name: $ty = $ty(1 << ($n));
        bitset!(@bit $ty, $n + 1; $($rest)*);
    };
    (@bit $ty:ident, $n:expr;) => {};
}

pub(crate) use bitset;
