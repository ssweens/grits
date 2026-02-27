# Grits

Intent WAL for parallel AI agent coordination. A single Rust binary with 9 subcommands that read/append to a JSONL file. No database, no daemon.

## Install

```bash
cargo install --path .
```

## Setup

```bash
# Initialize grits in a git repository (creates .grits/, configures mergiraf if available)
grits init

# Inject agent workflow guidance into AGENTS.md
grits agents --add --force
```

`grits init` creates the `.grits/` directory and, if [mergiraf](https://mergiraf.org) is on PATH, configures it as an AST-aware git merge driver. Mergiraf resolves structural conflicts (like parallel import additions) that standard git cannot. If mergiraf isn't installed, init still succeeds — just skips merge driver setup.

`grits agents --add` creates or appends workflow guidance to AGENTS.md (or CLAUDE.md if one exists). Use `--remove` to strip it. Use `--force` to skip confirmation.

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

# Initialize grits + mergiraf merge driver
grits init
grits init --force   # reinitialize

# Manage agent file guidance
grits agents              # check status
grits agents --add --force   # add blurb to AGENTS.md
grits agents --remove --force   # strip blurb
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

When claiming a symbol in an existing file with a supported language, grits validates the symbol exists using tree-sitter AST parsing. Invalid symbols show grouped available symbols and "did you mean?" suggestions:

```bash
grits claim src/lib.rs:nonexistent
# error: symbol 'nonexistent' not found in src/lib.rs
# hint: available symbols: User { new, create }, validate_email

grits claim src/lib.rs:valid
# error: symbol 'valid' not found in src/lib.rs
# hint: did you mean validate_email? available: User { new, create }, validate_email

grits claim src/lib.rs:user
# error: symbol 'user' not found in src/lib.rs
# hint: did you mean User? available: User { new, create }, validate_email
```

Suggestions are tiered: qualified form match ("new" → "User.new"), case-insensitive match ("user" → "User"), then prefix match ("valid" → "validate_email").

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

Use `grits agents --add --force` to inject the workflow blurb, or add manually:

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
