# Test Coverage

## Summary

- **25 total tests** (13 unit + 12 integration)
- All passing as of initial implementation

## Unit tests (13)

### `conflict.rs` (6 tests)
| Test | Description |
|------|-------------|
| `same_file_same_symbol_conflicts` | Same file + same symbol = conflict |
| `same_file_different_symbols_no_conflict` | Same file + different symbols = no conflict |
| `different_files_no_conflict` | Different files = no conflict |
| `whole_file_claim_conflicts_with_symbol` | Whole file claim vs symbol = conflict |
| `symbol_claim_conflicts_with_whole_file_check` | Symbol vs whole file check = conflict |
| `whole_file_vs_whole_file_conflicts` | Whole file vs whole file = conflict |

### `store.rs` (3 tests)
| Test | Description |
|------|-------------|
| `empty_store_returns_no_entries` | Fresh store has no entries or claims |
| `append_and_read_round_trips` | Write + read preserves all fields |
| `active_claims_excludes_released` | Released claims are excluded from active set |

### `id.rs` (3 tests)
| Test | Description |
|------|-------------|
| `id_has_gs_prefix` | Generated IDs start with `gs-` |
| `same_inputs_produce_same_id` | Deterministic: same inputs = same ID |
| `different_inputs_produce_different_ids` | Different inputs = different IDs |

### `identity.rs` (1 test)
| Test | Description |
|------|-------------|
| `detect_uses_process_tree` | Detects agent type from process tree, returns non-empty identity |

## Integration tests (12)

All integration tests run the compiled `grits` binary against a temp directory with a fresh `.grits/intents.jsonl`.

| Test | Description |
|------|-------------|
| `claim_then_check_shows_conflict` | Claim + check same symbol = exit 1 |
| `claim_different_symbols_no_conflict` | Claim foo + check bar = clear |
| `claim_release_then_reclaim_succeeds` | Claim + release + reclaim = success |
| `whole_file_claim_conflicts_with_symbol` | Whole file claim + symbol check = conflict |
| `status_shows_active_claims` | Status lists all active claims |
| `blame_shows_last_release` | Blame shows commit SHA from release |
| `log_shows_history` | Log shows claim + release entries |
| `log_by_agent_filters` | Log --agent filters by agent type |
| `json_mode_check_clear` | --json check returns `{"status":"clear"}` |
| `json_mode_claim` | --json claim returns ID with gs- prefix |
| `release_nonexistent_id_fails` | Releasing unknown ID exits with code 2 |
| `prime_outputs_primer` | Prime outputs the agent primer text |
