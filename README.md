# Baton

**One window for the terminals you keep open on other machines.**

Baton is an SSH client and terminal for macOS. Panes from different servers live
side by side in one window, hosts are organised and inherit their settings, and
everything is reachable from a command palette.

> ### Status: pre-alpha — not usable yet
>
> `cargo run` prints a line and exits. There is no window, no installer, and no
> release. What exists is the terminal grid, a patched input stack, and the
> measurements that settled the architecture.
>
> Follow [milestone M1](https://github.com/cielixer/batonry/milestone/1) to see
> how far along it is.

---

## What it will do

Milestone 1 is the whole SSH client. The goal is narrow and testable: **replace
the SSH client and terminal you use today.**

- **Terminal** — GPU rendered, truecolor, underline styles, hyperlinks, correct
  CJK and emoji widths, working Korean input
- **Splits and tabs** — vertical and horizontal splits, drag resize, keyboard
  focus movement, scrollback, incremental search, rectangular copy
- **Workspaces** — one task holding panes from several hosts at once. Quit the
  app and reopen it, and the arrangement comes back
- **Host management** — nested groups with inheritance, tags, keys and SSH
  certificates, jump hosts, port forwarding, snippets, and importing your
  existing `~/.ssh/config`
- **Real connections** — connection reuse, automatic reconnect, and connection
  state visible at all times
- **Command palette** — every action lives here, so a day can be spent without
  the mouse

Later milestones add sessions that survive quitting the app, agent sessions, and
moving a task from one server to another. The terminal comes first, because
nothing built on a terminal that breaks is worth using.

## What it will not do

Not "later" — these are structural.

| | why |
|---|---|
| No server, no account, no relay | The only authentication surface is your own SSH. Neither code nor credentials pass through a third party |
| No stored credential bytes | Names and how to obtain them, nothing more. Passphrases go to the OS keychain or `ssh-agent` |
| No agent forwarding by default | Anyone with root on the remote could impersonate you. Jump hosts instead |
| No sync, team vaults, session recording, or SSO | All of it needs a server. Share configuration as text files in git |
| No mobile client | The SSH key would have to live on the phone |
| No Windows | Connection reuse is Unix-only by construction |

## Building

Requires a recent stable Rust and macOS.

```sh
cargo build --workspace
cargo test  --workspace
```

Three crates under `crates/` are copies of upstream libraries rather than
dependencies, because Cargo cannot rename a patched package. Each carries a
`NOTICE.md` explaining why it is there and an `UPSTREAM.diff` recording our
divergence line by line — two are unmodified apart from one dependency line, and
`baton-winit` holds the macOS input fix. They are excluded from the workspace and
must not be reformatted: the diffs are only readable while the surrounding code
stays put.

## Contributing

It is too early for code contributions; the interfaces move weekly. Bug reports
and ideas are welcome once there is something to run.

## License

MIT — see [`LICENSE`](LICENSE). Copied crates keep their upstream licence and
notice inside the crate directory.
