# Grits Intent WAL — Design Document

**Date**: 2026-02-25
**Status**: Draft

## Vision

A single append-only JSONL file that serves two purposes:
1. **Coordination** — Prevents agents from colliding on the same code
2. **Provenance** — "git blame for agents" — tracks which agent and when modified each symbol

No database. No daemon. No sync protocol. Just a file.

## Data Model

### Storage

```
.grits/
└── intents.jsonl    # Append-only log of all claims and releases
```

### Entry Schema

```typescript
interface AgentIdentity {
  type: string;        // Agent kind: "claude", "codex", "pi", "human", etc.
  cwd: string;         // Working directory (typically a worktree path)
}

interface IntentEntry {
  id: string;          // Hash-based ID (e.g., "gs-a3f8")
  agent: AgentIdentity;
  op: "claim" | "release" | "amend";
  file: string;        // Relative file path
  symbol?: string;     // Function/class/method name (null = whole file)
  commit?: string;     // Git commit SHA (on release only)
  ts: string;          // ISO 8601 timestamp
}
```

Agent identity is **type + cwd**. No fallback chain — both required.

- `type` — what kind of agent (`claude`, `codex`, `pi`, `human`)
- `cwd` — where it's working (worktree path distinguishes parallel agents)

Set via `GRITS_AGENT_TYPE` env var + auto-detected `cwd`.

### Operations

**claim** — Agent declares intent to modify a symbol:
```jsonl
{"id":"gs-a3f8","agent":{"type":"claude","cwd":"/repo/.claude/worktrees/feat-auth"},"op":"claim","file":"src/utils/helpers.ts","symbol":"validateEmail","ts":"2026-02-25T20:30:00Z"}
```

**release** — Agent is done, records the commit:
```jsonl
{"id":"gs-a3f8","agent":{"type":"claude","cwd":"/repo/.claude/worktrees/feat-auth"},"op":"release","file":"src/utils/helpers.ts","symbol":"validateEmail","commit":"8a360d5","ts":"2026-02-25T20:35:00Z"}
```

**amend** — Agent discovers it needs to touch another symbol mid-work:
```jsonl
{"id":"gs-b7c1","agent":{"type":"codex","cwd":"/repo/.claude/worktrees/fix-bug"},"op":"amend","file":"src/utils/helpers.ts","symbol":"hashPassword","ts":"2026-02-25T20:32:00Z"}
```

### Deriving State

**Active claims** (current locks):
```
claims without a matching release (same id)
```

**Symbol history** (provenance):
```
all entries for a given file:symbol, ordered by ts
```

**Agent activity** (what is agent-A doing?):
```
all active claims where agent = "agent-A"
```

## CLI

### Core Commands

```bash
# Declare intent to modify a symbol
grits claim src/utils/helpers.ts:validateEmail

# Check if a symbol is safe to modify
grits check src/utils/helpers.ts:validateEmail
# → CONFLICT: agent-A claimed this symbol at 20:30:00Z

# Check a whole file
grits check src/utils/helpers.ts
# → 2 active claims:
#   agent-A → validateEmail (since 20:30)
#   agent-B → slugify (since 20:31)
#   hashPassword, truncate, formatDate — available

# Release after work is done
grits release gs-a3f8 --commit 8a360d5

# See all active intents
grits status
# → agent-A: src/utils/helpers.ts:validateEmail (5m ago)
# → agent-B: src/utils/helpers.ts:slugify (4m ago)
# → agent-C: src/core/engine.rs:start (2m ago)
```

### Provenance Commands

```bash
# Who last modified this symbol?
grits blame src/utils/helpers.ts:validateEmail
# → agent-A at 2026-02-25T20:35:00Z
# → commit: 8a360d5

# Full history of a symbol
grits log src/utils/helpers.ts:validateEmail
# → 2026-02-25 20:30 agent-A claimed
# → 2026-02-25 20:35 agent-A released (commit: 8a360d5)
# → 2026-02-24 14:10 agent-C claimed
# → 2026-02-24 14:22 agent-C released (commit: 7e72250)

# What did this agent change?
grits log --agent agent-A
# → helpers.ts:validateEmail (commit 8a360d5)
# → users.ts:createUser (commit 060de14)
```

### Output Modes

Every command supports two output modes, auto-detected:

| Flag | Mode | Use case |
|------|------|----------|
| (default) | Human-readable text | Interactive terminal use |
| `--json` | Structured JSON | Agent consumption, piping |

### Structured Errors

Errors return a consistent shape so agents can self-correct:

```json
{
  "error": "CONFLICT",
  "message": "agent-A has an active claim on src/utils/helpers.ts:validateEmail",
  "hint": "Use 'grits status' to see all claims, or pick a different symbol",
  "retryable": true
}
```

Exit codes:
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Conflict (retryable — pick different target) |
| 2 | Invalid input (retryable — fix args) |
| 3 | IO/system error |

### Agent Identity

Agent identity is `type` + `cwd` (see Entry Schema above). Set via:
```bash
export GRITS_AGENT_TYPE="claude"   # required
```
The `cwd` is auto-detected. No fallback chain — `grits claim` fails if `GRITS_AGENT_TYPE` is not set.

## Agent Integration

### How Agents Learn About Grits

Grits ships a `grits prime` command that outputs a short markdown block for injection into an agent's context. This is the single integration point — no plugins, no MCP, just text.

```bash
$ grits prime
```

Outputs (~100 tokens):

```markdown
# Grits — File Coordination

Before modifying any file, check for conflicts:
  grits check <file>:<symbol>

If clear, claim it:
  grits claim <file>:<symbol>

When done, release it:
  grits release <id> --commit <sha>

Use --json for structured output. Run `grits status` to see all active claims.
```

### Hook: SessionStart

For Claude Code, grits registers as a `SessionStart` hook. When an agent session begins, `grits prime` runs automatically and its output enters the agent's context window.

```json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "",
      "hooks": [{ "type": "command", "command": "grits prime" }]
    }]
  }
}
```

The agent now knows to check/claim before editing. No AGENTS.md editing required for this path — but it works there too.

### Hook: PreToolUse (Edit/Write guard)

Grits can optionally register a `PreToolUse` hook that fires before file edits. The hook runs `grits check` on the target file and blocks the edit if there's an active claim by another agent.

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Edit|Write",
      "hooks": [{ "type": "command", "command": "grits guard $FILE" }]
    }]
  }
}
```

This is the safety net — even if an agent forgets to `grits check`, the hook catches it.

### AGENTS.md / CLAUDE.md Integration

For setups without hooks, add to the project's agent instructions:

```markdown
## File Coordination (grits)

Before modifying any file:
1. `grits check <file>:<symbol>` — verify no conflicts
2. `grits claim <file>:<symbol>` — declare your intent
3. Do the work
4. `grits release <id> --commit <sha>` — record what you did
```

This works with any agent framework that reads instruction files.

### The Agent Workflow

```
1. Agent receives task
2. Agent identifies files/symbols it needs to modify
3. For each target:
   a. grits check file:symbol --json
   b. If clear → grits claim file:symbol
   c. If conflict → wait, pick different target, or escalate
4. Do the work in the worktree
5. Commit
6. grits release <id> --commit <sha>
```

## Conflict Detection Rules

### File + Symbol Granularity

Two claims conflict when:
1. **Same file, same symbol** — always conflicts
2. **Same file, one claims whole file (symbol=null)** — conflicts with everything in that file
3. **Same file, one claims "append"** — conflicts with other "append" claims

Two claims do NOT conflict when:
1. **Same file, different symbols** — agents can work on different functions in parallel
2. **Different files** — no overlap possible

### The Append Problem

From our Mergiraf evaluation, scenarios 4 and 5 showed that parallel appends to the same file always conflict. Grits solves this by:

1. Agent A: `grits claim helpers.ts --op append`
2. Agent B: `grits check helpers.ts` → sees Agent A has an append claim
3. Agent B either waits, or negotiates (claims a specific insertion point)

## Integration Points

### With Git Worktrees

Each agent works in a worktree. The `.grits/intents.jsonl` file lives in the main worktree (shared). Agents read/write to it before starting work in their worktree.

### With Mergiraf

Grits prevents the conflicts Mergiraf can't handle (parallel appends, semantic conflicts). Mergiraf handles the conflicts grits doesn't need to worry about (import merging, textual overlap in different hunks).

## Design Principles

1. **Non-invasive** — Never runs git commands. Never modifies source code.
2. **Explicit** — No auto-commits, no daemons, no background processes.
3. **Append-only** — The JSONL is a WAL. No updates, no deletes.
4. **Agent-first** — Every command supports `--json`. Structured errors with hints.
5. **Human-readable** — `cat .grits/intents.jsonl` tells you everything.
6. **Standalone** — No dependency on any task tracker, MCP server, or external system.
7. **Hookable** — `grits prime` for SessionStart, `grits guard` for PreToolUse. Both optional.
