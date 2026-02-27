# Test Coverage

## Summary

- **70 total tests** (35 unit + 35 integration)
- All passing

## Unit tests (35)

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

### `symbols.rs` (22 tests)
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
| `contains_qualified_name` | `table.contains("User.new")` → true |
| `contains_unqualified_name` | `table.contains("new")` → true |
| `contains_nonexistent` | `table.contains("fake")` → false |
| `suggest_qualified_form` | Query "new" when "User.new" exists → suggests "User.new" |
| `suggest_case_insensitive` | Query "user" when "User" exists → suggests "User" |
| `suggest_prefix` | Query "valid" when "validate_email" exists → suggests "validate_email" |
| `suggest_no_match` | Query "zzz" → empty suggestions |
| `format_hint_groups_containers` | Groups nested symbols: "User { new }, validate_email" |
| `format_hint_top_level_only` | No grouping for top-level only: "foo, bar" |

## Integration tests (35)

All integration tests run the compiled `grits` binary against a temp directory with a fresh git repo.

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
| `init_creates_grits_dir` | Creates `.grits/` and `.grits/.gitignore` |
| `init_configures_mergiraf_when_available` | When mergiraf on PATH, sets merge driver in `.git/config` |
| `init_writes_gitattributes` | Creates `.gitattributes` with mergiraf mappings |
| `init_sets_diff3` | Sets `merge.conflictStyle = diff3` |
| `init_succeeds_without_mergiraf` | Still succeeds and warns when mergiraf missing |
| `init_is_idempotent` | Running init twice succeeds (no --force needed) |
| `init_json_mode` | Structured JSON output |
| `agents_check_no_file` | Reports no agent file found |
| `agents_add_creates_file` | `--add` creates AGENTS.md with blurb |
| `agents_add_appends_to_existing` | Appends blurb to existing AGENTS.md |
| `agents_remove_strips_blurb` | `--remove` removes blurb cleanly |
| `agents_add_idempotent` | Second `--add` is no-op when blurb present |
| `uninstall_reverses_init` | Removes `.grits/`, unsets git config |
| `uninstall_reverses_agents` | Removes AGENTS.md when it only contained blurb |
| `uninstall_preserves_non_grits_content` | Strips blurb but keeps other content in AGENTS.md |
| `uninstall_nothing_is_noop` | Clean repo prints "nothing to uninstall" |
| `uninstall_json_mode` | Structured JSON output |
