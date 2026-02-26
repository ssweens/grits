use crate::conflict::check_conflicts;
use crate::store::Store;
use crate::{find_root, parse_target, GritsError};

pub fn run(target: &str, json: bool) -> Result<(), GritsError> {
    let (file, symbol) = parse_target(target);
    let root = find_root()?;
    let store = Store::open(&root)?;

    let active = store.active_claims()?;
    let conflicts = check_conflicts(&file, symbol.as_deref(), &active);

    if conflicts.is_empty() {
        if json {
            println!(r#"{{"status":"clear"}}"#);
        } else {
            println!("clear");
        }
        Ok(())
    } else {
        if json {
            let items: Vec<_> = conflicts.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "agent": { "type": c.agent.type_, "cwd": c.agent.cwd },
                    "file": c.file,
                    "symbol": c.symbol,
                    "ts": c.ts,
                })
            }).collect();
            let out = serde_json::json!({
                "status": "conflict",
                "conflicts": items,
            });
            println!("{}", serde_json::to_string(&out).unwrap());
        } else {
            println!("CONFLICT: {} active claim(s)", conflicts.len());
            for c in &conflicts {
                let who = format!("{} @ {}", c.agent.type_, c.agent.cwd);
                let what = match &c.symbol {
                    Some(s) => format!("{}:{}", c.file, s),
                    None => c.file.clone(),
                };
                println!("  {} → {} (since {})", who, what, c.ts);
            }
        }
        Err(GritsError::conflict(
            format!("{} conflict(s) found", conflicts.len()),
            "Use 'grits status' to see all claims, or pick a different symbol".to_string(),
        ))
    }
}
