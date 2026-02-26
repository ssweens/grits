# Grits

Intent WAL for parallel AI agent coordination. A single Rust binary with 7 subcommands that read/append to a JSONL file. No database, no daemon.

## Install

```bash
cargo install --path .
```

## How it works

Grits tracks file-level claims in `.grits/intents.jsonl`. Agents claim files/symbols before editing, and release them when done. The JSONL file is append-only — state is derived by reading all entries.

Agent identity is auto-detected by walking the process tree (claude, codex, cursor, windsurf) with `cwd` distinguishing parallel agents in different worktrees.

## Commands

```bash
# Declare intent to modify a symbol
grits claim src/utils/helpers.ts:validateEmail
# → gs-a3f8 (agent: claude @ /repo)

# Check if a symbol is safe to modify
grits check src/utils/helpers.ts:validateEmail
# → CONFLICT: claude @ /repo claimed validateEmail

# Check a different symbol (no conflict)
grits check src/utils/helpers.ts:slugify
# → clear

# Release after work is done
grits release gs-a3f8 --commit abc123

# See all active claims
grits status

# Who last modified this symbol?
grits blame src/utils/helpers.ts:validateEmail

# Full history
grits log src/utils/helpers.ts:validateEmail

# Filter by agent
grits log --agent claude

# Print agent primer (for context injection)
grits prime
```

## JSON mode

Every command supports `--json` for structured output:

```bash
grits check src/lib.rs:foo --json
# → {"status":"clear"}

grits claim src/lib.rs:foo --json
# → {"id":"gs-a3f8","agent":{"type":"claude","cwd":"/repo"}}
```

Errors return a structured envelope:

```json
{
  "error": "CONFLICT",
  "message": "claude @ /repo has an active claim on src/lib.rs:foo",
  "hint": "Use 'grits status' to see all claims, or pick a different symbol",
  "retryable": true
}
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Conflict (retryable) |
| 2 | Invalid input (retryable) |
| 3 | IO/system error |

## Symbol validation

When claiming a symbol in an existing file with a supported language, grits validates the symbol exists using tree-sitter AST parsing:

```bash
grits claim src/lib.rs:nonexistent
# error: symbol 'nonexistent' not found in src/lib.rs
# hint: available symbols: validate_email, hash_password, UserService, UserService.new
```

Symbols use qualified dot notation: `User.new`, `UserService.create`, `Foo.bar`.

**Supported languages:** Rust, TypeScript, JavaScript, Python, Go

**Validation is skipped when:**
- No symbol is specified (whole-file claim)
- The file doesn't exist yet (agent may be creating it)
- The file's language is unsupported (no grammar available)

Only `claim` validates symbols. `check` stays fast — it only reads the store.

## Conflict rules

- Same file + same symbol = conflict
- Same file + whole file claim (no symbol) = conflict with everything in that file
- Same file + different symbols = no conflict
- Different files = no conflict

## Agent integration

### Claude Code hook

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

### CLAUDE.md / AGENTS.md

```markdown
## File Coordination (grits)

Before modifying any file:
1. `grits check <file>:<symbol>` — verify no conflicts
2. `grits claim <file>:<symbol>` — declare your intent
3. Do the work
4. `grits release <id> --commit <sha>` — record what you did
```

## Design

See [docs/2026-02-25-intent-wal-design.md](docs/2026-02-25-intent-wal-design.md) for the full design document.
