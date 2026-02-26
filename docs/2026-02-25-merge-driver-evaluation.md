# AST-Aware Merge Driver Evaluation Results

**Date**: 2026-02-25
**Mergiraf version**: 0.16.3
**Git version**: git version 2.50.1 (Apple Git-155)

## Results Matrix

| # | Scenario | Expected | Git Default | Mergiraf | Winner |
|---|----------|----------|-------------|----------|--------|
| 1 | Different functions, same file (TS) | clean | PASS (clean) | PASS (clean) | Tie |
| 2 | Same function, different lines (TS) | clean* | PASS (clean) | PASS (clean) | Tie |
| 3 | Add function + modify existing (TS) | clean | PASS (clean) | PASS (clean) | Tie |
| 4 | Both add different functions at EOF (TS) | clean | FAIL (conflict) | FAIL (conflict) | Tie (both fail) |
| 5 | Both add functions at same location (TS) | clean | FAIL (conflict) | FAIL (conflict) | Tie (both fail) |
| 6 | Delete function + modify different (TS) | clean | PASS (clean) | PASS (clean) | Tie |
| 7 | Delete function + modify deleted (TS) | conflict | PASS (conflict) | PASS (conflict) | Tie |
| 8 | Rename + call old name (semantic) | conflict† | clean | clean | Tie (both miss) |
| 9 | Different methods in same class (TS) | clean | PASS (clean) | PASS (clean) | Tie |
| 10 | Both modify imports (TS) | clean | FAIL (conflict) | **PASS (clean)** | **Mergiraf** |
| 11 | Different functions in Python file | clean | PASS (clean) | PASS (clean) | Tie |
| 12 | Different impl methods in Rust file | clean | PASS (clean) | PASS (clean) | Tie |
| 13 | Different sections in JSON config | clean | PASS (clean) | PASS (clean) | Tie |
| 14 | Large file, disjoint changes (perf) | clean | PASS (clean) | PASS (clean) | Tie |
| 15 | Syntax errors in source file | clean | PASS (clean) | PASS (clean) | Tie |

**\*** Scenario 2 was originally expected to conflict, but investigation showed the changes were 7 lines apart within a 15-line function — different hunks. Both drivers correctly merged this.

**†** Scenario 8 is a **semantic** conflict (calling a renamed function). No structural merge driver can detect this — it requires type-checking or reference resolution.

## Corrected Scores

| Driver | Structural Pass | Total Structural | Score |
|--------|----------------|------------------|-------|
| Git Default | 10 | 13 | 10/13 (77%) |
| Mergiraf | **11** | 13 | **11/13 (85%)** |

*Excludes scenario 2 (corrected expectation) and scenario 8 (semantic — untestable by merge drivers).*

## Performance

| Driver | Avg merge time | Notes |
|--------|---------------|-------|
| Git Default | ~25ms | Baseline |
| Mergiraf | ~42ms | ~1.7x slower, still imperceptible |

Both are well within interactive-speed thresholds. Mergiraf's overhead (~17ms) is negligible.

## Analysis

### Where Mergiraf wins (1 scenario)

**Scenario 10 — Both modify imports**: Git sees two agents changing the same import line and conflicts. Mergiraf understands import statement structure and correctly merges both additions into a single import statement.

### Where both fail equally (2 scenarios)

**Scenarios 4 & 5 — Both add new functions at the same position**: When two agents both append a new function at EOF (or insert at the same anchor point), neither git nor Mergiraf can determine ordering. The base file has no structural anchor between the insertions. This is a fundamental limitation of all structural merge — parallel insertions at the same position are inherently ambiguous.

### Where neither tool can help (1 scenario)

**Scenario 8 — Semantic conflict (rename + old reference)**: Agent A renames `validateEmail` → `isValidEmail`. Agent B adds a new function that calls `validateEmail`. Both drivers merge cleanly, producing broken code. This is a semantic/type-level conflict that no merge driver can detect — it would require a type checker or reference resolver.

### What passed everywhere (11 scenarios)

For the bread-and-butter multi-agent case — different agents modifying different functions/methods/sections in the same file — both git and Mergiraf handle it well. This includes:
- Different functions in the same TS file
- Different methods in the same class
- Delete + modify (different functions)
- Add function + modify existing
- Python, Rust, and JSON files
- Large files with 60 functions
- Files with syntax errors (graceful fallback)

## Decision

### Recommendation: **Use Mergiraf**, with caveats

**Mergiraf provides a small but real improvement** over git's default merge:
- It correctly handles the import merge case (scenario 10), which is a *very common* multi-agent conflict
- It maintains the same correctness as git on all other scenarios
- Performance overhead is negligible (~17ms)
- Excellent language coverage (30+ languages including all our targets)
- Graceful fallback on syntax errors (falls back to line-based merge)
- Easy to set up (cargo install + .gitattributes)

**However, it does not solve the hard problems:**
- Parallel insertions at the same position (scenarios 4 & 5) — needs an ordering strategy
- Semantic conflicts (scenario 8) — needs type-checking, not structural merge

### What's still needed: Intent WAL

The cases Mergiraf can't solve (parallel insertions, semantic conflicts) are exactly what the **Intent WAL** approach would address:
- Agents declare intent before working ("I'm adding a function to helpers.ts")
- Coordination layer detects potential conflicts *before* agents start
- For parallel insertions: establishes ordering convention (e.g., alphabetical, or first-declared-wins)
- For semantic conflicts: detects when one agent's change would break another's references

### Setup

To enable Mergiraf globally:

```bash
# Install
cargo install --git https://codeberg.org/mergiraf/mergiraf.git

# Register as git merge driver
git config --global merge.mergiraf.name mergiraf
git config --global merge.mergiraf.driver 'mergiraf merge --git %O %A %B -s %S -x %X -y %Y -p %P'

# Add to .gitattributes (per-repo)
mergiraf languages --gitattributes >> .gitattributes
```

## Raw Data

### Timing (milliseconds)

| Scenario | Git Default | Mergiraf |
|----------|-------------|----------|
| 1 | 29 | 51 |
| 2 | 26 | 35 |
| 3 | 27 | 39 |
| 4 | 19 | 42 |
| 5 | 19 | 38 |
| 6 | 28 | 40 |
| 7 | 22 | 39 |
| 8 | 25 | 38 |
| 9 | 30 | 35 |
| 10 | 19 | 45 |
| 11 | 25 | 49 |
| 12 | 28 | 55 |
| 13 | 26 | 36 |
| 14 | 24 | 38 |
| 15 | 27 | 45 |
