# `crates/baton-iced` -- why this is here

A hardcopy of the `iced` 0.14.0 facade (MIT) whose **only change is one
dependency line**: it takes `iced_winit` from `crates/baton-iced-winit` instead
of crates.io.

- Upstream: crates.io `iced 0.14.0`
- Licence: MIT, same as upstream
- Our delta: `UPSTREAM.diff` -- Cargo.toml only, no source file touched

This is the last link in the chain that carries the patched winit down to our
crates. Everything else `iced` depends on -- `iced_widget`, `iced_wgpu`,
`iced_runtime`, `iced_program`, ... -- still comes from crates.io unchanged;
none of them reference winit.

Our crates depend on it as `iced = { path = "crates/baton-iced", package =
"baton-iced" }`, so their source says `use iced::...` like any other project.

Reasoning and the upgrade procedure: [`../baton-iced-winit/NOTICE.md`](../baton-iced-winit/NOTICE.md).
