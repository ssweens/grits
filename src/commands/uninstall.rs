use std::fs;
use std::path::Path;
use std::process::Command;

use crate::GritsError;
use super::init::GITATTRIBUTES_HEADER;

pub fn run(json: bool) -> Result<(), GritsError> {
    let root = crate::find_root()?;
    let mut steps: Vec<String> = Vec::new();

    // Remove .grits/ directory
    let grits_dir = root.join(".grits");
    if grits_dir.exists() {
        fs::remove_dir_all(&grits_dir)
            .map_err(|e| GritsError::io(format!("failed to remove .grits/: {e}")))?;
        steps.push("  [-] .grits/".to_string());
    }

    // Unset mergiraf merge driver from .git/config
    let unset_results = [
        ("merge.mergiraf.name", "merge.mergiraf.name"),
        ("merge.mergiraf.driver", "merge.mergiraf.driver"),
        ("merge.conflictStyle", "merge.conflictStyle"),
    ];

    for (key, label) in &unset_results {
        if git_config_unset(&root, key) {
            steps.push(format!("  [-] .git/config — {label}"));
        }
    }

    // Strip mergiraf lines from .gitattributes
    let attrs_path = root.join(".gitattributes");
    if attrs_path.exists() {
        let content = fs::read_to_string(&attrs_path)
            .map_err(|e| GritsError::io(format!("failed to read .gitattributes: {e}")))?;

        if content.contains("merge=mergiraf") {
            let cleaned = strip_mergiraf_lines(&content);
            if cleaned.trim().is_empty() {
                fs::remove_file(&attrs_path)
                    .map_err(|e| GritsError::io(format!("failed to remove .gitattributes: {e}")))?;
                steps.push("  [-] .gitattributes (removed — was empty)".to_string());
            } else {
                fs::write(&attrs_path, cleaned)
                    .map_err(|e| GritsError::io(format!("failed to write .gitattributes: {e}")))?;
                steps.push("  [-] .gitattributes — mergiraf entries".to_string());
            }
        }
    }

    // Strip agent blurb from AGENTS.md / CLAUDE.md
    let (agent_path, has_blurb) = super::agents::find_agent_file(&root);
    if has_blurb {
        let path = agent_path.unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(&path)
            .map_err(|e| GritsError::io(format!("failed to read {name}: {e}")))?;

        let stripped = super::agents::strip_blurb(&content);
        if stripped.trim().is_empty() {
            fs::remove_file(&path)
                .map_err(|e| GritsError::io(format!("failed to remove {name}: {e}")))?;
            steps.push(format!("  [-] {name} (removed — was empty)"));
        } else {
            fs::write(&path, stripped)
                .map_err(|e| GritsError::io(format!("failed to write {name}: {e}")))?;
            steps.push(format!("  [-] {name} — grits blurb"));
        }
    }

    if json {
        let output = serde_json::json!({
            "uninstalled": true,
            "steps": steps.len(),
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    } else if steps.is_empty() {
        println!("nothing to uninstall");
    } else {
        println!("Uninstalled grits:");
        for step in &steps {
            println!("{step}");
        }
    }

    Ok(())
}

/// Unset a git config key. Returns true if it was set (and got unset).
fn git_config_unset(root: &Path, key: &str) -> bool {
    // Check if the key exists first
    let check = Command::new("git")
        .args(["config", "--get", key])
        .current_dir(root)
        .output();

    match check {
        Ok(output) if output.status.success() => {
            // Key exists, unset it
            let _ = Command::new("git")
                .args(["config", "--unset", key])
                .current_dir(root)
                .output();
            true
        }
        _ => false,
    }
}

/// Remove the mergiraf header and all `*.ext merge=mergiraf` lines from .gitattributes.
fn strip_mergiraf_lines(content: &str) -> String {
    let mut result = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == GITATTRIBUTES_HEADER || trimmed.ends_with("merge=mergiraf") {
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    // Trim trailing blank lines but keep a final newline
    let trimmed = result.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}
