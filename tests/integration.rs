use std::process::Command;

fn grits(args: &[&str], dir: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_grits"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run grits")
}

fn grits_stdout(args: &[&str], dir: &std::path::Path) -> String {
    let out = grits(args, dir);
    String::from_utf8(out.stdout).unwrap()
}

fn setup() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    // Create a .git directory so grits finds the root
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    dir
}

/// Setup with a real git repo (needed for `git config` commands in init).
fn setup_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to git init");
    dir
}

/// Create a source file in the temp dir and return its relative path.
fn write_file(dir: &std::path::Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, content).unwrap();
}

/// Extract a claim ID from grits claim output (first whitespace-delimited token).
fn extract_id(output: &str) -> &str {
    output.split_whitespace().next().unwrap()
}

#[test]
fn claim_then_check_shows_conflict() {
    let dir = setup();

    let claim_out = grits_stdout(&["claim", "src/lib.rs:foo"], dir.path());
    let id = extract_id(&claim_out);
    assert!(id.starts_with("gs-"), "expected gs- prefix, got: {id}");

    let check = grits(&["check", "src/lib.rs:foo"], dir.path());
    assert_eq!(check.status.code(), Some(1), "expected exit code 1 for conflict");

    let stderr = String::from_utf8(check.stderr).unwrap();
    assert!(stderr.contains("conflict"), "expected conflict in stderr: {stderr}");
}

#[test]
fn claim_different_symbols_no_conflict() {
    let dir = setup();

    grits(&["claim", "src/lib.rs:foo"], dir.path());

    let check = grits(&["check", "src/lib.rs:bar"], dir.path());
    assert_eq!(check.status.code(), Some(0), "different symbols should not conflict");

    let stdout = String::from_utf8(check.stdout).unwrap();
    assert!(stdout.contains("clear"));
}

#[test]
fn claim_release_then_reclaim_succeeds() {
    let dir = setup();

    let claim_out = grits_stdout(&["claim", "src/lib.rs:foo"], dir.path());
    let id = extract_id(&claim_out);

    let release = grits(&["release", id, "--commit", "abc123"], dir.path());
    assert!(release.status.success(), "release should succeed");

    // Re-claim same symbol — should work since it was released
    let reclaim = grits(&["claim", "src/lib.rs:foo"], dir.path());
    assert!(reclaim.status.success(), "reclaim after release should succeed");
}

#[test]
fn whole_file_claim_conflicts_with_symbol() {
    let dir = setup();

    grits(&["claim", "src/lib.rs"], dir.path());

    let check = grits(&["check", "src/lib.rs:foo"], dir.path());
    assert_eq!(check.status.code(), Some(1), "whole file claim should conflict with symbol");
}

#[test]
fn status_shows_active_claims() {
    let dir = setup();

    grits(&["claim", "a.rs:x"], dir.path());
    grits(&["claim", "b.rs:y"], dir.path());

    let out = grits_stdout(&["status"], dir.path());
    assert!(out.contains("2 active claim(s)"));
    assert!(out.contains("a.rs:x"));
    assert!(out.contains("b.rs:y"));
}

#[test]
fn blame_shows_last_release() {
    let dir = setup();

    let claim_out = grits_stdout(&["claim", "src/lib.rs:foo"], dir.path());
    let id = extract_id(&claim_out);
    grits(&["release", id, "--commit", "deadbeef"], dir.path());

    let blame_out = grits_stdout(&["blame", "src/lib.rs:foo"], dir.path());
    assert!(blame_out.contains("deadbeef"), "blame should show commit: {blame_out}");
}

#[test]
fn log_shows_history() {
    let dir = setup();

    let claim_out = grits_stdout(&["claim", "src/lib.rs:foo"], dir.path());
    let id = extract_id(&claim_out);
    grits(&["release", id, "--commit", "abc"], dir.path());

    let log_out = grits_stdout(&["log", "src/lib.rs:foo"], dir.path());
    assert!(log_out.contains("claim"));
    assert!(log_out.contains("release"));
    assert!(log_out.contains("abc"));
}

#[test]
fn log_by_agent_filters() {
    let dir = setup();

    grits(&["claim", "a.rs:x"], dir.path());

    // Log by the detected agent type (could be "claude" or "human")
    let status_out = grits_stdout(&["status", "--json"], dir.path());
    let v: serde_json::Value = serde_json::from_str(&status_out).unwrap();
    let agent_type = v["claims"][0]["agent"]["type"].as_str().unwrap();

    let log_out = grits_stdout(&["log", "--agent", agent_type], dir.path());
    assert!(log_out.contains("a.rs:x"));
}

#[test]
fn json_mode_check_clear() {
    let dir = setup();

    let out = grits_stdout(&["check", "src/lib.rs:foo", "--json"], dir.path());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], "clear");
}

#[test]
fn json_mode_claim() {
    let dir = setup();

    let out = grits_stdout(&["claim", "src/lib.rs:foo", "--json"], dir.path());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["id"].as_str().unwrap().starts_with("gs-"));
}

#[test]
fn release_nonexistent_id_fails() {
    let dir = setup();

    let out = grits(&["release", "gs-nonexistent", "--commit", "abc"], dir.path());
    assert_eq!(out.status.code(), Some(2), "releasing unknown id should exit 2");
}

#[test]
fn prime_outputs_primer() {
    let dir = setup();
    let out = grits_stdout(&["prime"], dir.path());
    assert!(out.contains("grits check"));
    assert!(out.contains("grits claim"));
    assert!(out.contains("grits release"));
}

// -- Symbol validation tests --

#[test]
fn claim_valid_symbol_succeeds() {
    let dir = setup();
    write_file(dir.path(), "src/lib.rs", "fn validate_email() {}\nfn hash_password() {}");

    let out = grits(&["claim", "src/lib.rs:validate_email"], dir.path());
    assert!(out.status.success(), "claiming valid symbol should succeed");
}

#[test]
fn claim_invalid_symbol_fails_with_hint() {
    let dir = setup();
    write_file(dir.path(), "src/lib.rs", "fn validate_email() {}\nfn hash_password() {}");

    let out = grits(&["claim", "src/lib.rs:nonexistent"], dir.path());
    assert_eq!(out.status.code(), Some(2), "claiming invalid symbol should exit 2");

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("symbol 'nonexistent' not found"), "should say symbol not found: {stderr}");
    assert!(stderr.contains("validate_email"), "hint should list available symbols: {stderr}");
    assert!(stderr.contains("hash_password"), "hint should list available symbols: {stderr}");
}

#[test]
fn claim_qualified_symbol_succeeds() {
    let dir = setup();
    write_file(
        dir.path(),
        "src/lib.rs",
        "struct User {}\nimpl User {\n    fn new() -> Self { User {} }\n}",
    );

    let out = grits(&["claim", "src/lib.rs:User.new"], dir.path());
    assert!(out.status.success(), "claiming qualified symbol should succeed");
}

#[test]
fn claim_nonexistent_file_skips_validation() {
    let dir = setup();
    // Don't create the file — validation should be skipped

    let out = grits(&["claim", "src/new_file.rs:anything"], dir.path());
    assert!(out.status.success(), "claiming symbol in nonexistent file should succeed");
}

#[test]
fn claim_unsupported_language_skips_validation() {
    let dir = setup();
    write_file(dir.path(), "data.csv", "id,name\n1,Alice");

    let out = grits(&["claim", "data.csv:anything"], dir.path());
    assert!(out.status.success(), "claiming symbol in unsupported file type should succeed");
}

#[test]
fn claim_invalid_symbol_json_mode() {
    let dir = setup();
    write_file(dir.path(), "src/lib.rs", "fn real_fn() {}");

    let out = grits(&["claim", "src/lib.rs:fake_fn", "--json"], dir.path());
    assert_eq!(out.status.code(), Some(2));

    let stderr = String::from_utf8(out.stderr).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(v["error"], "INVALID_INPUT");
    assert!(v["hint"].as_str().unwrap().contains("real_fn"));
}

// -- Init tests --

#[test]
fn init_creates_grits_dir() {
    let dir = setup_git_repo();

    let out = grits(&["init"], dir.path());
    assert!(out.status.success(), "init should succeed");

    assert!(dir.path().join(".grits").is_dir());
    assert!(dir.path().join(".grits/.gitignore").exists());

    let gitignore = std::fs::read_to_string(dir.path().join(".grits/.gitignore")).unwrap();
    assert!(gitignore.contains("*.lock"));
    assert!(gitignore.contains("*.tmp"));
}

#[test]
fn init_configures_mergiraf_when_available() {
    let dir = setup_git_repo();

    // Only test mergiraf config if mergiraf is on PATH
    let mergiraf_available = Command::new("mergiraf")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let out = grits(&["init"], dir.path());
    assert!(out.status.success());

    if mergiraf_available {
        let driver = Command::new("git")
            .args(["config", "--get", "merge.mergiraf.driver"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(driver.status.success(), "mergiraf driver should be configured");
        let driver_val = String::from_utf8(driver.stdout).unwrap();
        assert!(driver_val.contains("mergiraf merge"), "driver should invoke mergiraf merge");
    }
}

#[test]
fn init_writes_gitattributes() {
    let dir = setup_git_repo();

    // Only test gitattributes if mergiraf is on PATH
    let mergiraf_available = Command::new("mergiraf")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    grits(&["init"], dir.path());

    if mergiraf_available {
        let attrs = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(attrs.contains("*.rs merge=mergiraf"));
        assert!(attrs.contains("*.ts merge=mergiraf"));
        assert!(attrs.contains("*.py merge=mergiraf"));
        assert!(attrs.contains("*.go merge=mergiraf"));
    }
}

#[test]
fn init_sets_diff3() {
    let dir = setup_git_repo();

    let mergiraf_available = Command::new("mergiraf")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    grits(&["init"], dir.path());

    if mergiraf_available {
        let style = Command::new("git")
            .args(["config", "--get", "merge.conflictStyle"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(style.status.success());
        let val = String::from_utf8(style.stdout).unwrap();
        assert_eq!(val.trim(), "diff3");
    }
}

#[test]
fn init_succeeds_without_mergiraf() {
    let dir = setup_git_repo();

    // Init always succeeds — just warns if mergiraf is missing
    let out = grits(&["init"], dir.path());
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Initialized grits"));
    assert!(dir.path().join(".grits").is_dir());
}

#[test]
fn init_already_initialized_fails() {
    let dir = setup_git_repo();

    let first = grits(&["init"], dir.path());
    assert!(first.status.success());

    let second = grits(&["init"], dir.path());
    assert_eq!(second.status.code(), Some(2), "second init should fail without --force");

    let stderr = String::from_utf8(second.stderr).unwrap();
    assert!(stderr.contains("already exists"));
}

#[test]
fn init_force_reinitializes() {
    let dir = setup_git_repo();

    grits(&["init"], dir.path());

    let out = grits(&["init", "--force"], dir.path());
    assert!(out.status.success(), "init --force should succeed on re-init");
}

#[test]
fn init_json_mode() {
    let dir = setup_git_repo();

    let out = grits_stdout(&["init", "--json"], dir.path());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["initialized"], true);
    assert_eq!(v["path"], ".grits/");
}

// -- Agents tests --

#[test]
fn agents_check_no_file() {
    let dir = setup_git_repo();

    let out = grits_stdout(&["agents"], dir.path());
    assert!(out.contains("no agent file found"));
}

#[test]
fn agents_add_creates_file() {
    let dir = setup_git_repo();

    let out = grits(&["agents", "--add", "--force"], dir.path());
    assert!(out.status.success());

    let agents_md = dir.path().join("AGENTS.md");
    assert!(agents_md.exists(), "AGENTS.md should be created");

    let content = std::fs::read_to_string(&agents_md).unwrap();
    assert!(content.contains("grits-agent-instructions-v1"));
    assert!(content.contains("grits check"));
    assert!(content.contains("grits claim"));
    assert!(content.contains("grits release"));
}

#[test]
fn agents_add_appends_to_existing() {
    let dir = setup_git_repo();

    // Create an existing AGENTS.md
    write_file(dir.path(), "AGENTS.md", "# Existing content\n\nSome rules.\n");

    let out = grits(&["agents", "--add", "--force"], dir.path());
    assert!(out.status.success());

    let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.starts_with("# Existing content"), "should preserve original content");
    assert!(content.contains("grits-agent-instructions-v1"), "should have blurb");
}

#[test]
fn agents_remove_strips_blurb() {
    let dir = setup_git_repo();

    // Add then remove
    grits(&["agents", "--add", "--force"], dir.path());
    assert!(dir.path().join("AGENTS.md").exists());

    let out = grits(&["agents", "--remove", "--force"], dir.path());
    assert!(out.status.success());

    // Backup should exist
    assert!(dir.path().join("AGENTS.md.bak").exists());

    // Blurb should be gone
    let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(!content.contains("grits-agent-instructions-v1"));
}

#[test]
fn agents_add_idempotent() {
    let dir = setup_git_repo();

    grits(&["agents", "--add", "--force"], dir.path());
    let first_content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();

    let out = grits(&["agents", "--add", "--force"], dir.path());
    assert!(out.status.success());

    let second_content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert_eq!(first_content, second_content, "second --add should be no-op");

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("already present"));
}
