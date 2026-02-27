use std::fs;
use std::path::{Path, PathBuf};

use crate::GritsError;

const START_MARKER: &str = "<!-- grits-agent-instructions-v1 -->";
const END_MARKER: &str = "<!-- end-grits-agent-instructions -->";

const BLURB: &str = r#"<!-- grits-agent-instructions-v1 -->

---

## File Coordination (grits)

This project uses grits for file-level coordination between parallel AI agents.
Claims are tracked in `.grits/intents.jsonl`.

### Workflow

Before modifying any file:
1. `grits check <file>:<symbol>` — verify no conflicts
2. `grits claim <file>:<symbol>` — declare your intent
3. Do the work
4. `grits release <id> --commit <sha>` — record what you did

Use `grits status` to see all active claims. Use `--json` for structured output.

### Rules
- Always check before claiming
- Use qualified names for nested symbols: `User.new`, `Foo.bar`
- Release claims promptly after committing

### Merge Driver

This project uses mergiraf for AST-aware git merges. It resolves structural
conflicts (like parallel import additions) that standard git cannot. No action
needed — it's configured via `.gitattributes`.

<!-- end-grits-agent-instructions -->"#;

/// File names to search for in priority order.
const AGENT_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md", "agents.md", "claude.md"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Check,
    Add,
    Remove,
}

pub fn run(mode: Mode, force: bool, json: bool) -> Result<(), GritsError> {
    let root = crate::find_root()?;

    match mode {
        Mode::Check => run_check(&root, json),
        Mode::Add => run_add(&root, force, json),
        Mode::Remove => run_remove(&root, force, json),
    }
}

/// Check mode: report whether an agent file exists and has the grits blurb.
fn run_check(root: &Path, json: bool) -> Result<(), GritsError> {
    let (path, has_blurb) = find_agent_file(root);

    if json {
        let output = serde_json::json!({
            "file": path.as_ref().map(|p| p.file_name().unwrap().to_string_lossy().to_string()),
            "has_blurb": has_blurb,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    } else {
        match (&path, has_blurb) {
            (Some(p), true) => {
                let name = p.file_name().unwrap().to_string_lossy();
                println!("{name} — grits blurb present");
            }
            (Some(p), false) => {
                let name = p.file_name().unwrap().to_string_lossy();
                println!("{name} — grits blurb NOT present");
                println!("  run: grits agents --add --force");
            }
            (None, _) => {
                println!("no agent file found (AGENTS.md, CLAUDE.md)");
                println!("  run: grits agents --add --force");
            }
        }
    }

    Ok(())
}

/// Add mode: create or append agent file with grits blurb.
fn run_add(root: &Path, force: bool, json: bool) -> Result<(), GritsError> {
    let (existing_path, has_blurb) = find_agent_file(root);

    if has_blurb {
        if json {
            let output = serde_json::json!({
                "action": "none",
                "reason": "blurb already present",
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        } else {
            let name = existing_path.unwrap().file_name().unwrap().to_string_lossy().to_string();
            println!("{name} — grits blurb already present (no changes)");
        }
        return Ok(());
    }

    let target_path = existing_path.unwrap_or_else(|| root.join("AGENTS.md"));
    let target_name = target_path.file_name().unwrap().to_string_lossy().to_string();
    let is_new = !target_path.exists();

    if !force {
        if json {
            let output = serde_json::json!({
                "action": "dry_run",
                "file": &target_name,
                "would_create": is_new,
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        } else {
            if is_new {
                println!("would create {target_name} with grits blurb");
            } else {
                println!("would append grits blurb to {target_name}");
            }
            println!("  run: grits agents --add --force");
        }
        return Ok(());
    }

    // Write the blurb
    if is_new {
        fs::write(&target_path, format!("{BLURB}\n"))
            .map_err(|e| GritsError::io(format!("failed to write {target_name}: {e}")))?;
    } else {
        let existing = fs::read_to_string(&target_path)
            .map_err(|e| GritsError::io(format!("failed to read {target_name}: {e}")))?;

        let mut content = existing;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
        content.push_str(BLURB);
        content.push('\n');

        fs::write(&target_path, content)
            .map_err(|e| GritsError::io(format!("failed to write {target_name}: {e}")))?;
    }

    if json {
        let output = serde_json::json!({
            "action": if is_new { "created" } else { "appended" },
            "file": &target_name,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    } else if is_new {
        println!("created {target_name} with grits blurb");
    } else {
        println!("appended grits blurb to {target_name}");
    }

    Ok(())
}

/// Remove mode: strip grits blurb from agent file.
fn run_remove(root: &Path, force: bool, json: bool) -> Result<(), GritsError> {
    let (existing_path, has_blurb) = find_agent_file(root);

    if !has_blurb {
        if json {
            let output = serde_json::json!({
                "action": "none",
                "reason": "no blurb found",
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        } else {
            println!("no grits blurb found in agent files");
        }
        return Ok(());
    }

    let path = existing_path.unwrap();
    let name = path.file_name().unwrap().to_string_lossy().to_string();

    if !force {
        if json {
            let output = serde_json::json!({
                "action": "dry_run",
                "file": &name,
                "would_remove": true,
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        } else {
            println!("would remove grits blurb from {name}");
            println!("  run: grits agents --remove --force");
        }
        return Ok(());
    }

    // Create backup
    let backup_path = path.with_extension("md.bak");
    fs::copy(&path, &backup_path)
        .map_err(|e| GritsError::io(format!("failed to create backup: {e}")))?;

    // Strip the blurb
    let content = fs::read_to_string(&path)
        .map_err(|e| GritsError::io(format!("failed to read {name}: {e}")))?;

    let stripped = strip_blurb(&content);

    fs::write(&path, stripped)
        .map_err(|e| GritsError::io(format!("failed to write {name}: {e}")))?;

    if json {
        let output = serde_json::json!({
            "action": "removed",
            "file": &name,
            "backup": backup_path.file_name().unwrap().to_string_lossy().to_string(),
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    } else {
        let backup_name = backup_path.file_name().unwrap().to_string_lossy();
        println!("removed grits blurb from {name} (backup: {backup_name})");
    }

    Ok(())
}

/// Find the first matching agent file and check if it contains the blurb.
fn find_agent_file(root: &Path) -> (Option<PathBuf>, bool) {
    for name in AGENT_FILES {
        let path = root.join(name);
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            let has_blurb = content.contains(START_MARKER);
            return (Some(path), has_blurb);
        }
    }
    (None, false)
}

/// Remove the grits blurb section (between markers, inclusive) from content.
fn strip_blurb(content: &str) -> String {
    let mut result = String::new();
    let mut in_blurb = false;

    for line in content.lines() {
        if line.trim() == START_MARKER {
            in_blurb = true;
            continue;
        }
        if line.trim() == END_MARKER {
            in_blurb = false;
            continue;
        }
        if !in_blurb {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Trim trailing whitespace but keep a final newline
    let trimmed = result.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}
