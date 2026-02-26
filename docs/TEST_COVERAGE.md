# Test Coverage

## Summary

- **44 total tests** (26 unit + 18 integration)
- All passing

## Unit tests (26)

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

### `symbols.rs` (13 tests)
| Test | Description |
|------|-------------|
| `rust_top_level_fn` | Extracts top-level `fn` names |
| `rust_struct_and_enum` | Extracts `struct` and `enum` names |
| `rust_impl_method_qualified` | `impl MyStruct { fn method }` → `MyStruct.method` |
| `rust_const_static_type` | Extracts `const`, `static`, `type` alias names |
| `typescript_class_with_method` | `class Foo { bar() }` → `Foo`, `Foo.bar` |
| `typescript_function_declaration` | Extracts top-level function declarations |
| `javascript_class_with_method` | JS class with method produces qualified names |
| `python_class_with_method` | `class Foo: def bar` → `Foo`, `Foo.bar` |
| `python_top_level_function` | Extracts top-level function definitions |
| `go_function_and_type` | Extracts `func` and `type` declarations |
| `go_method_with_receiver` | `func (f *Foo) Bar()` → `Foo.Bar` |
| `unsupported_extension_returns_none` | `.csv` → `None` |
| `no_extension_returns_none` | No extension → `None` |

## Integration tests (18)

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
| `claim_valid_symbol_succeeds` | Claim real symbol in existing .rs file passes |
| `claim_invalid_symbol_fails_with_hint` | Claim nonexistent symbol = exit 2 with available symbols |
| `claim_qualified_symbol_succeeds` | `User.new` qualified name resolves correctly |
| `claim_nonexistent_file_skips_validation` | Claim in nonexistent file passes (skip validation) |
| `claim_unsupported_language_skips_validation` | Claim in .csv file passes (skip validation) |
| `claim_invalid_symbol_json_mode` | JSON error envelope includes hint with available symbols |
