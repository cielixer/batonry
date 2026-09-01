# `.claude/skills` -- what these are and where they came from

Three skills that drive development on this repository, plus the Codex runner
they share.

    baton-plan    design, then write the ticket, then have a second model review it
    baton-ticket  implement one ticket in reviewed batches, ending at a draft PR
    baton-gate    full suite plus two parallel reviews, updating that PR

    codex/        one script, four prompt roles, a review checklist

They are **specific to this repository** on purpose. They know that `CLAUDE.md`
is the contract, that `crates/winit` is a frozen copy that must not be edited,
that everything git tracks is English, and that the person running them does not
review test code. A general-purpose workflow cannot know any of that, and those
are the facts that actually prevent mistakes here.

## Where the ideas came from

**[TRIP-workflow](https://github.com/PiLastDigit/TRIP-workflow)** by PiLastDigit
-- the model split and the delegation design. Implementation runs a fast model
in a persistent thread, batch by batch, with the requester's corrections carried
forward as notes; review runs a *different, stronger* model in a read-only
sandbox. One thread per unit of work, keyed state, and parsing a trailing tag out
of the report are all theirs.

**[superpowers](https://github.com/anthropics/claude-code)**
`subagent-driven-development` by Anthropic -- the discipline around that loop.
Design before plan before implement. Run continuously instead of checking in.
Rule on ambiguity rather than stalling, and write the ruling down. A circuit
breaker on the fix loop so a review that will not converge gets adjudicated
instead of argued with. A ledger that lets a later session resume.

The two compose rather than compete, because they delegate for different
reasons: one wants **a different model's opinion**, the other wants **a bounded,
recorded process**. Cross-model review is the part neither could get from
running more of itself.

## Why the scripts here are ours

`codex/codex-run.sh` implements TRIP's design; it is not a copy of TRIP's
scripts. TRIP's README carries an MIT badge, but the repository has no `LICENSE`
file and the GitHub API reports no license, so the badge links to a 404. Rather
than commit someone else's code into a public MIT repository on an unresolved
licence, the design was reimplemented -- which this project's own rule already
required: `CLAUDE.md` section 1 says reference implementations are read, not
copied, and the licence is checked first.

It ended up shorter anyway: 190 lines against roughly 490, because five scripts
became one with subcommands and the model selection became an explicit `--role`
instead of being inferred from a directory path.

## What was deliberately dropped from TRIP

| dropped | why |
|---|---|
| Its numbered documentation tree (`1-plans`, `2-changelog`, `3-code-review`, …) | The planning notes for this project live outside the repository and have their own structure. A second tree inside it would compete with them |
| `ARCHI.md` | `CLAUDE.md` already is that file. Two canonical descriptions of the architecture is the exact failure its own section 8 forbids |
| Plan documents | The GitHub ticket **is** the plan: four required sections, one ticket per PR per session. A plan file would be a second copy that drifts |
| `TRIP-3-release` | Versioning, changelogs, tags and fast-forward merges, for a pre-alpha with no releases. The branch and PR flow is already decided |

## Requirements

`codex` (OpenAI Codex CLI), `jq`, `gh`. Optional: GNU `timeout` or `gtimeout`
(`brew install coreutils`) so `CODEX_TIMEOUT` can bound a run; without it the
runner warns and runs unbounded.

Models default to `gpt-5.6-luna` at high effort on the fast tier for
implementation, and `gpt-5.6-sol` at xhigh on standard routing for review.
Override per run with `CODEX_MODEL`, `CODEX_EFFORT`, `CODEX_TIER`.

## One thing to know before cloning

These skills refer to a decisions log, which git does not
track: the planning notes for this project are deliberately kept out of the
repository. So a fresh clone gets skills whose most important reference is
missing. That is a real limitation rather than an oversight -- the alternative
was publishing prose that was never written for anyone else to read.

**Nothing the repository tracks depends on those notes**, which is the point of
keeping them out. Project vocabulary is defined in each crate's `//!` docs, where
`cargo doc` renders it and where it cannot drift from the code, rather than in a
document a clone does not have. The skills stay in the repository because they
belong with the code they govern, and because how a project is run is worth
seeing.
