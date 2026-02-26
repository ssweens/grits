use chrono::Utc;

use crate::identity::AgentIdentity;
use crate::store::{IntentEntry, Store};
use crate::{find_root, GritsError};

pub fn run(id: &str, commit: &str, json: bool) -> Result<(), GritsError> {
    let root = find_root()?;
    let store = Store::open(&root)?;
    let agent = AgentIdentity::detect()?;

    let active = store.active_claims()?;
    let claim = active.iter().find(|e| e.id == id);

    let claim = match claim {
        Some(c) => c,
        None => {
            return Err(GritsError::invalid_input(
                format!("no active claim with id '{id}'"),
            ));
        }
    };

    let entry = IntentEntry {
        id: id.to_string(),
        agent,
        op: "release".to_string(),
        file: claim.file.clone(),
        symbol: claim.symbol.clone(),
        commit: Some(commit.to_string()),
        ts: Utc::now().to_rfc3339(),
    };

    store.append(&entry)?;

    if json {
        let out = serde_json::json!({
            "id": id,
            "released": true,
            "commit": commit,
        });
        println!("{}", serde_json::to_string(&out).unwrap());
    } else {
        println!("released {} (commit: {})", id, commit);
    }

    Ok(())
}
