use crate::store::Store;
use crate::{find_root, parse_target, GritsError};

pub fn run(target: &str, json: bool) -> Result<(), GritsError> {
    let (file, symbol) = parse_target(target);
    let root = find_root()?;
    let store = Store::open(&root)?;
    let entries = store.read_all()?;

    // Find the most recent release matching file:symbol
    let release = entries
        .iter()
        .rev()
        .find(|e| {
            e.op == "release" && e.file == file && e.symbol.as_deref() == symbol.as_deref()
        });

    match release {
        Some(r) => {
            if json {
                let out = serde_json::json!({
                    "agent": { "type": r.agent.type_, "cwd": r.agent.cwd },
                    "file": r.file,
                    "symbol": r.symbol,
                    "commit": r.commit,
                    "ts": r.ts,
                });
                println!("{}", serde_json::to_string(&out).unwrap());
            } else {
                let commit_str = r.commit.as_deref().unwrap_or("unknown");
                println!("{} @ {}", r.agent.type_, r.agent.cwd);
                println!("commit: {}", commit_str);
                println!("ts: {}", r.ts);
            }
            Ok(())
        }
        None => {
            if json {
                println!(r#"{{"agent":null}}"#);
            } else {
                println!("no release history for {target}");
            }
            Ok(())
        }
    }
}
