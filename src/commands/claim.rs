use chrono::Utc;

use crate::conflict::check_conflicts;
use crate::id::generate_id;
use crate::identity::AgentIdentity;
use crate::store::{IntentEntry, Store};
use crate::{find_root, parse_target, GritsError};

pub fn run(target: &str, json: bool) -> Result<(), GritsError> {
    let (file, symbol) = parse_target(target);
    let agent = AgentIdentity::detect()?;
    let root = find_root()?;
    let store = Store::open(&root)?;

    let active = store.active_claims()?;
    let conflicts = check_conflicts(&file, symbol.as_deref(), &active);

    if !conflicts.is_empty() {
        let c = &conflicts[0];
        let who = format!("{} @ {}", c.agent.type_, c.agent.cwd);
        let what = match &c.symbol {
            Some(s) => format!("{}:{}", c.file, s),
            None => c.file.clone(),
        };
        return Err(GritsError::conflict(
            format!("{who} has an active claim on {what}"),
            "Use 'grits status' to see all claims, or pick a different symbol".to_string(),
        ));
    }

    let ts = Utc::now().to_rfc3339();
    let id = generate_id(&file, symbol.as_deref(), &agent.type_, &agent.cwd, &ts);

    let entry = IntentEntry {
        id: id.clone(),
        agent: agent.clone(),
        op: "claim".to_string(),
        file,
        symbol,
        commit: None,
        ts,
    };

    store.append(&entry)?;

    if json {
        let out = serde_json::json!({
            "id": id,
            "agent": { "type": agent.type_, "cwd": agent.cwd },
        });
        println!("{}", serde_json::to_string(&out).unwrap());
    } else {
        println!("{} (agent: {} @ {})", id, agent.type_, agent.cwd);
    }

    Ok(())
}
