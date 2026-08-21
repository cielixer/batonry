# `crates/baton-iced-winit` -- why this is here

A hardcopy of `iced_winit` 0.14.0 (MIT) whose **only change is one dependency
line**: it takes `winit` from `crates/baton-winit` instead of crates.io.

- Upstream: crates.io `iced_winit 0.14.0`
- Licence: MIT, same as upstream
- Our delta: `UPSTREAM.diff` -- Cargo.toml only, no source file touched

## Why it exists

macOS Korean input needs a patched winit (see
[`../baton-winit/NOTICE.md`](../baton-winit/NOTICE.md)). The dependency chain is

    our crates -> iced -> iced_winit -> winit

and `iced_winit` is the crate that names `winit`. `[patch.crates-io]` could
redirect it, but patch substitutes by package identity: the replacement has to
*be* called `winit`, so the crate we maintain could not carry our own name.

Hardcopying this crate lets us use a renamed dependency instead --
`winit = { path = "../baton-winit", package = "baton-winit" }` -- which is legal
in `[dependencies]`. The code still says `use winit::...` and is untouched.

**The fallback is loud, not silent.** With `[patch]`, deleting one line made
cargo quietly resolve the published winit: the build succeeded and Korean input
broke. Now the redirect is an ordinary path dependency, and
`crates/baton-term/tests/ime.rs` asserts it is still in place.

## Upgrading iced

Re-copy this crate from the new `iced_winit`, re-apply the one dependency line,
and do the same for `../baton-iced`. The source is verbatim upstream, so there
is nothing to merge.
