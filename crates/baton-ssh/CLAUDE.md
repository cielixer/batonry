# baton-ssh -- the implementation contract

**This file supplements the repository's [`CLAUDE.md`](../../CLAUDE.md).** The root contract governs the whole project; this one holds rules that apply to this crate alone. On a conflict, the root wins. The root's §3 bans in particular still hold here: no inbound port, no agent forwarding by default, no credential bytes stored, no SSH implementation of our own linked, and never proceeding quietly on a changed host key.

**Paths are written relative to the repository root.** `DECISIONS.md` and `evidence/*` are under `docs/milestones/01-ssh-client/`.

---

## 1. The enforced configuration

```
ControlMaster        auto
ControlPath          ~/.baton/ssh/%C-<jhash>  # <jhash> = eight characters of our own hash of the
                                             # jump path, written in as a literal string. It is not
                                             # an ssh token: substitute it when generating the config.
ControlPersist       10m
ServerAliveInterval  15
ServerAliveCountMax  3
ForwardAgent         no
```

## 2. Diagnostics leave through `-E`, never stderr

**Always pass `-E <fifo>`.** That is what gets ssh's diagnostics out of the pty. **Conversation goes over the pty; diagnostics go over `-E`.**
`-E` accepts a FIFO (measured). It does **not** accept `/dev/fd/N` (measured). The five measurements are in [`evidence/ssh.md`](../../docs/milestones/01-ssh-client/evidence/ssh.md).

| What | Where it goes |
|---|---|
| passphrases, 2FA, passwords, and **confirming a host key seen for the first time** | the **pty**. The user answers directly |
| the warning that a host key has **changed**, DNS failures, refusals, timeouts, authentication failures, mux state | the **`-E` FIFO**. We parse it |

**Do not parse stderr.** Given `-E`, stderr is empty. There is exactly **one input to error classification, the `-E` channel.**

## 3. `%C` is not enough, and `-o` does not reach a jump hop

**`%C` alone is insufficient (measured).** `%C = hash(%l%h%p%r%j)`, but **`%j` is only the last hop, not the whole jump path.** So `-J b1,b2` and `-J b2` **produce the same `%C`.** Two connections over different paths sharing one socket means traffic does not take the path we intended.
Therefore **we hash the entire jump path ourselves and append eight characters as a suffix.** ssh has no token for this, so **write the hash into the generated config as a literal.** A connection with no jump gets the suffix `direct`.

**`-o` is not passed down to a jump hop (measured).** `ProxyJump` internally invokes a recursive `ssh -W`, and **the only flags that inner process inherits are `-l -p -F -v`.** Neither `-o` nor `-i` crosses. Supplying the configuration above through `-o` therefore **applies it to the final hop only and never to the bastion.**
Therefore **generate a config file per connection and pass it with `-F <path>`.** Do not force settings with `-o`. `-F` is inherited, so the same configuration binds every hop.

## 4. Connection behaviour

- **Jump only via `ProxyJump`.** Do not write a `ProxyCommand` directly.
- `probe` reads the remote's `MaxSessions` and `MaxStartups` and **warns when `MaxSessions` is under 20.**
- Reconnect with exponential backoff. Always show the state in the UI: `connected / reconnecting (n) / disconnected (reason)`.
- **Detecting a disconnect is not immediate.** `ServerAliveInterval 15` × `CountMax 3` measured at 56 seconds. **Write the UI copy as "about a minute"** and never as "detected immediately".
- **Show the jump path as one line in the status bar.** No path diagram and no "shorter route" suggestion in M1.
