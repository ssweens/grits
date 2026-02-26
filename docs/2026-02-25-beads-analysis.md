# Beads Analysis — Ideas for Intent WAL

**Date**: 2026-02-25
**Source**: [github.com/steveyegge/beads](https://github.com/steveyegge/beads) (17k stars)
**Author**: Steve Yegge
**Language**: Go, backed by Dolt (version-controlled SQL)

## What Beads Is

A distributed, git-backed graph issue tracker designed as persistent memory for AI coding agents. Replaces unstructured markdown plans with a dependency-aware task graph backed by Dolt.

## Key Architecture Decisions

### Cell-Level Merge (via Dolt)

Dolt merges at `(row, column)` granularity instead of line-level. Two agents updating different fields on the same issue merge cleanly. Conflict only when both write different values to the same cell.

Current design: all agents share `main` branch with SQL transactions (moved away from branch-per-worker because agents couldn't see each other's work).

### Atomic Claim (`--claim`)

`bd update bd-a3f8 --claim` sets assignee + status in a single SQL transaction. Prevents race conditions where two agents grab the same task.

### Work Frontier (`bd ready`)

Returns only tasks with no open blockers and no current owner. Uses a materialized `blocked_issues_cache` table (752ms → 29ms query time on 10K issues). Children of an epic run in parallel by default — only explicit `blocks` edges create sequence.

### Hash-Based IDs

SHA-256 of (title + description + creator + timestamp + nonce) → base36 encoding. Length scales adaptively with DB size using birthday paradox formula. Prevents merge collisions when agents create tasks in parallel across branches.

### Wisps (Ephemeral Work)

Temporary child tasks that never sync to remotes. Stored in separate `dolt_ignore`d tables. Agents can plan locally, then either squash into permanent records or burn (discard).

### Compaction (Memory Decay)

Two-tier AI summarization:
- 30+ days closed → Tier 1 (~70% size reduction via Claude Haiku)
- 90+ days closed → Tier 2 (~95% reduction)
- Original data survives in Dolt commit history
- `bd restore <id>` recovers full history

### Contributor Isolation

Contributors get a separate DB at `~/.beads-planning/`. Maintainers write to `./.beads/`. Detection via git remote URL analysis (SSH = maintainer, HTTPS = contributor).

## What Beads Does NOT Do

- No source code conflict prevention
- No file-level awareness (doesn't know which files a task will touch)
- No semantic conflict detection
- No merge driver integration
- Purely task-level coordination

## Relevance to Intent WAL

### The Gap

Beads partitions at the **task level**. Mergiraf resolves at the **text level**. The Intent WAL sits between — agents declare which **files and functions** they intend to modify, and the system detects overlaps before work begins.

### Design Borrowings

| Beads Concept | Intent WAL Equivalent |
|---|---|
| `bd ready` (unblocked tasks) | `intent check` (no conflicting intents) |
| `bd update --claim` (atomic) | `intent declare --file X --fn Y` (atomic) |
| Dependency DAG | File/function lock graph |
| Wisps (ephemeral) | Tentative intents (revocable before commit) |
| Compaction (30d/90d) | Intent expiry after merge |
| Cell-level merge (Dolt) | Structured intent store (not flat files) |
| Hash IDs | Intent IDs for distributed creation |

### Three-Layer Architecture

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│   Beads      │     │  Intent WAL  │     │  Mergiraf    │
│  (task-level │ --> │  (file/func  │ --> │  (AST-aware  │
│   partition) │     │   intents)   │     │   merge)     │
└─────────────┘     └──────────────┘     └──────────────┘
  "Don't work on      "I'm about to       "If you do
   the same task"      edit helpers.ts:     conflict, merge
                       validateEmail()"     structurally"
```
