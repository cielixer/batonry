# baton-store -- the implementation contract

**This file supplements the repository's [`CLAUDE.md`](../../CLAUDE.md).** The root contract governs the whole project; this one holds rules that apply to this crate alone. On a conflict, the root wins. Two of the root's architecture contracts land here: **A4** (`Host` keeps `parent` and `jump` separate) and **A6** (`launch_spec` is persisted completely).

**Paths are written relative to the repository root.** `sys/data-model.md` and `DECISIONS.md` are under `docs/milestones/01-ssh-client/`.

---

## 1. Shape

- **One SQLite database on the user's machine.** Nothing is written to a remote.
- The tables are `host_group`, `host`, `key_ref`, `snippet`, `workspace`, `tab`, `pane`, `session`, `launch_spec` and `app_pref`.
- **A list-valued field (`jump`, `keys`, `tags`, `forwards`, `env`) is rows, not a column.** The real table count is sixteen. The full DDL is canonically in `sys/data-model.md`.
- `host`'s fields are `id, alias, group, user, hostname, port, parent, jump[], keys[], certificate, agent, env, startup_snippet, default_dir, tags[], forwards[], terminal{}`.

## 2. Constraints, enforced in the schema and not only in the UI

- `host_group` **cannot exceed depth 2.** Block it in the schema and in validation both.
- `host.favorite: bool`. A favourite is a flag, not a separate table.
- **A name is unique within its category.** `host.alias`, `workspace.name` and `snippet.name` are globally unique; a `host_group` name is unique **within the same parent**. A collision is refused at save time. This is a constraint, not UI copy.
- **`host.builtin: bool`** is true for the `local` row alone. Its alias is `local` and **the user cannot change it**: **refuse `UPDATE` and `DELETE` with a trigger.** Blocking it in the UI only means import, and export-then-import, route around it. Migration 0 inserts the row, and `builtin DESC` is always the first sort key.
- **This is how group inheritance works.** Absent an explicit override, the parent's value is inherited. The UI distinguishes an inherited value from a specified one and **names the group it came from**.

## 3. Workspaces, scratch, and sessions

- **A `workspace` is a named one, and only a named one.** **A scratch is one per host and holds a single host.** The `tab` row carries `is_scratch=1, scratch_host_id=<host>` and `pane` hangs off it by `tab_id`. `pane`'s `is_scratch` and `scratch_host_id` are **a mirror, with a composite foreign key that stops it from lying.** Naming one creates a `workspace` row and runs `UPDATE pane SET tab_id=?, is_scratch=0`. The canonical description is [`sys/data-model.md`](../../docs/milestones/01-ssh-client/sys/data-model.md) §1.3. **The arc moved from `pane` up to `tab` because the split tree and the tab order needed somewhere to live.**
- **A scratch cannot hold several hosts.** Enforce that in the schema. Mixing hosts is the definition of a workspace.
- `session` **is valid only within the lifetime of the app.** Mark them dead on shutdown. What gets restored is *a workspace's layout and what it connects to*, never a process.

## 4. `app_pref`

`app_pref(key, value)` holds app-wide settings such as "do not ask again". **Do not put a per-host or per-group setting here.**

The keys are settled at eight: `sidebar.mode`, `sidebar.width`, `sidebar.collapsed_groups` (a JSON array), `workspace.last`, `palette.recent`, `quit.confirm`, `theme.name` and `view.font_scale`. **The font size itself is `term_font_size` on `host` and `host_group`; `app_pref` holds only the scale**, and the effective size is the inherited value times the scale. When adding one, **edit `data-model.md` §1.5 and this section together.**

## 5. Export

Hosts, groups, snippets and workspaces **export and import as text**, so they can be shared through git. View state does not (see `crates/baton-ui/CLAUDE.md`).
