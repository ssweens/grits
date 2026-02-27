use std::fs;
use std::path::Path;
use std::process::Command;

use crate::GritsError;

/// Languages grits supports for AST-aware merging via mergiraf.
const MERGIRAF_EXTENSIONS: &[&str] = &["*.rs", "*.ts", "*.tsx", "*.js", "*.jsx", "*.py", "*.go"];

const GITIGNORE_CONTENT: &str = "*.lock\n*.tmp\n";

pub const GITATTRIBUTES_HEADER: &str = "# grits: AST-aware merging via mergiraf";

pub fn run(json: bool) -> Result<(), GritsError> {
    let root = crate::find_root()?;
    let grits_dir = root.join(".grits");

    // Create .grits/ directory (idempotent)
    fs::create_dir_all(&grits_dir)
        .map_err(|e| GritsError::io(format!("failed to create .grits/: {e}")))?;

    // Write .grits/.gitignore
    fs::write(grits_dir.join(".gitignore"), GITIGNORE_CONTENT)
        .map_err(|e| GritsError::io(format!("failed to write .grits/.gitignore: {e}")))?;

    let mut steps: Vec<String> = vec!["  [+] .grits/.gitignore".to_string()];

    // Check for mergiraf on PATH
    let mergiraf_available = detect_mergiraf();

    if mergiraf_available {
        configure_merge_driver(&root)?;
        steps.push("  [+] .git/config — mergiraf merge driver".to_string());

        configure_diff3(&root)?;
        steps.push("  [+] .git/config — merge.conflictStyle = diff3".to_string());

        write_gitattributes(&root)?;
        let exts: Vec<&str> = MERGIRAF_EXTENSIONS.to_vec();
        steps.push(format!("  [+] .gitattributes — mergiraf for {}", exts.join(", ")));
    } else {
        steps.push("  [!] mergiraf not found — install with: cargo install --locked mergiraf".to_string());
        steps.push("      Then re-run: grits init".to_string());
    }

    if json {
        let output = serde_json::json!({
            "initialized": true,
            "mergiraf": mergiraf_available,
            "gitattributes": mergiraf_available,
            "path": ".grits/",
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    } else {
        println!("Initialized grits in .grits/");
        for step in &steps {
            println!("{step}");
        }
        if !mergiraf_available {
            println!();
        }
        println!();
        println!("Next steps:");
        println!("  grits agents --add   # inject workflow guidance into AGENTS.md");
        println!("  grits claim <file>:<symbol>  # start coordinating");
    }

    Ok(())
}

/// Check if mergiraf is available on PATH.
fn detect_mergiraf() -> bool {
    Command::new("mergiraf")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Configure the mergiraf merge driver in .git/config.
fn configure_merge_driver(root: &Path) -> Result<(), GritsError> {
    let run_git = |args: &[&str]| -> Result<(), GritsError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| GritsError::io(format!("failed to run git config: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GritsError::io(format!("git config failed: {stderr}")));
        }
        Ok(())
    };

    run_git(&["config", "merge.mergiraf.name", "mergiraf"])?;
    run_git(&["config", "merge.mergiraf.driver", "mergiraf merge --git %O %A %B -s %S -x %X -y %Y -p %P"])?;

    Ok(())
}

/// Set merge.conflictStyle = diff3 in .git/config.
fn configure_diff3(root: &Path) -> Result<(), GritsError> {
    let output = Command::new("git")
        .args(["config", "merge.conflictStyle", "diff3"])
        .current_dir(root)
        .output()
        .map_err(|e| GritsError::io(format!("failed to run git config: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GritsError::io(format!("git config failed: {stderr}")));
    }
    Ok(())
}

/// Write mergiraf mappings to .gitattributes (append if exists, skip if already present).
fn write_gitattributes(root: &Path) -> Result<(), GritsError> {
    let path = root.join(".gitattributes");

    // Check if already configured
    if path.exists() {
        let content = fs::read_to_string(&path)
            .map_err(|e| GritsError::io(format!("failed to read .gitattributes: {e}")))?;
        if content.contains("merge=mergiraf") {
            return Ok(());
        }
    }

    let mut block = String::new();
    // Add newline separator if appending to existing file
    if path.exists() {
        let existing = fs::read_to_string(&path)
            .map_err(|e| GritsError::io(format!("failed to read .gitattributes: {e}")))?;
        if !existing.is_empty() && !existing.ends_with('\n') {
            block.push('\n');
        }
        block.push('\n');
    }

    block.push_str(GITATTRIBUTES_HEADER);
    block.push('\n');
    for ext in MERGIRAF_EXTENSIONS {
        block.push_str(&format!("{ext} merge=mergiraf\n"));
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| GritsError::io(format!("failed to open .gitattributes: {e}")))?;

    std::io::Write::write_all(&mut file, block.as_bytes())
        .map_err(|e| GritsError::io(format!("failed to write .gitattributes: {e}")))?;

    Ok(())
}
