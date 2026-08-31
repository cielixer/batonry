//! One declaration form for the crate's bitsets.

/// Declares a bitset's constants and its two operations.
///
/// The type is supplied rather than generated, so it keeps its own doc comment
/// and its own derives; the constants land as **associated constants**
/// (`Channels::PALETTE`, `Flags::PANE_FOCUSED`), so a use site names the type
/// it is reaching into. What this removes is the numbering and the arithmetic:
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
/// **The empty set is declared here too**, named and documented by the caller
/// for the same reason the operations are, and [`Default`] is generated to
/// return it -- tying "the default" to the named constant rather than to the
/// integer's zero, so the constant's hazard note covers both spellings. With
/// it inside, the only raw-bits construction outside this macro is the
/// conflict sweep's `assignments()` (#96).
///
/// With a table name it also emits an associated table pairing each constant
/// with its canonical spelling, for a set whose members are parsed or printed.
/// One table, so a parser and a formatter cannot drift apart and neither needs
/// an arm for a member it was never given. **The spelling is the identifier,
/// ASCII-lowercased, derived at compile time** (#99) -- so renaming a constant
/// renames the vocabulary users write, and the identifier is chosen to match
/// the specification, not the other way round. The tests pin every spelling
/// against a table transcribed from the specification by hand, which is what
/// catches a rename that breaks the vocabulary.
///
/// ```ignore
/// bitset!(Channels, reachable_from, union, KEY_ONLY: PALETTE, CLICK,);
/// bitset!(Flags, holds, combine, NAMES, NONE: PANE_FOCUSED,);  // spells "pane_focused"
/// ```
macro_rules! bitset {
    ($ty:ident, $contains:ident, $union:ident, $names:ident,
     $(#[$zdoc:meta])* $zero:ident:
     $($(#[$doc:meta])* $name:ident,)*) => {
        bitset!(@ops $ty, $contains, $union);
        bitset!(@zero $ty, $(#[$zdoc])* $zero);
        impl $ty {
            bitset!(@bit $ty, 0u32; $($(#[$doc])* $name,)*);

            /// Every member with its canonical spelling -- the identifier,
            /// ASCII-lowercased -- in bit order.
            const $names: &'static [($ty, &'static str)] = &[$((
                $ty::$name,
                {
                    const S: &str = stringify!($name);
                    const ARR: [u8; S.len()] =
                        crate::bitset::lower::<{ S.len() }>(S);
                    match ::core::str::from_utf8(&ARR) {
                        Ok(spelling) => spelling,
                        Err(_) => unreachable!(),
                    }
                },
            ),)*];
        }
    };
    ($ty:ident, $contains:ident, $union:ident,
     $(#[$zdoc:meta])* $zero:ident:
     $($(#[$doc:meta])* $name:ident,)*) => {
        bitset!(@ops $ty, $contains, $union);
        bitset!(@zero $ty, $(#[$zdoc])* $zero);
        impl $ty {
            bitset!(@bit $ty, 0u32; $($(#[$doc])* $name,)*);
        }
    };
    (@zero $ty:ident, $(#[$zdoc:meta])* $zero:ident) => {
        impl $ty {
            $(#[$zdoc])*
            pub const $zero: $ty = $ty(0);
        }

        impl Default for $ty {
            #[doc = concat!("[`", stringify!($ty), "::", stringify!($zero),
                "`], so the hazard note there covers this spelling too.")]
            fn default() -> $ty {
                $ty::$zero
            }
        }
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

/// The identifier's bytes, ASCII-lowercased, for deriving a spelling in a
/// `const` table. `const`, so no allocation: the caller names the length.
pub(crate) const fn lower<const N: usize>(s: &str) -> [u8; N] {
    let bytes = s.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i].to_ascii_lowercase();
        i += 1;
    }
    out
}

pub(crate) use bitset;
