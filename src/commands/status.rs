use std::collections::BTreeMap;

use crate::store::Store;
use crate::{find_root, GritsError};

pub fn run(json: bool) -> Result<(), GritsError> {
    let root = find_root()?;
    let store = Store::open(&root)?;
    let active = store.active_claims()?;

    if json {
        let items: Vec<_> = active.iter().map(|c| {
            serde_json::json!({
                "id": c.id,
                "agent": { "type": c.agent.type_, "cwd": c.agent.cwd },
                "file": c.file,
                "symbol": c.symbol,
                "ts": c.ts,
            })
        }).collect();
        println!("{}", serde_json::to_string(&serde_json::json!({ "claims": items })).unwrap());
        return Ok(());
    }

    if active.is_empty() {
        println!("no active claims");
        return Ok(());
    }

    // Group by agent key (type @ cwd)
    let mut by_agent: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for c in &active {
        let key = format!("{} @ {}", c.agent.type_, c.agent.cwd);
        by_agent.entry(key).or_default().push(c);
    }

    println!("{} active claim(s)", active.len());
    for (agent_key, claims) in &by_agent {
        for c in claims {
            let what = match &c.symbol {
                Some(s) => format!("{}:{}", c.file, s),
                None => c.file.clone(),
            };
            println!("  {}: {} [{}] ({})", agent_key, what, c.id, c.ts);
        }
    }

    Ok(())
}
