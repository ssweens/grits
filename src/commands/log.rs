use crate::store::Store;
use crate::{find_root, parse_target, GritsError};

pub fn run(target: Option<&str>, agent: Option<&str>, json: bool) -> Result<(), GritsError> {
    if target.is_none() && agent.is_none() {
        return Err(GritsError::invalid_input(
            "provide a file:symbol target or --agent flag".to_string(),
        ));
    }

    let root = find_root()?;
    let store = Store::open(&root)?;
    let entries = store.read_all()?;

    let filtered: Vec<_> = entries
        .iter()
        .filter(|e| {
            if let Some(t) = target {
                let (file, symbol) = parse_target(t);
                if e.file != file {
                    return false;
                }
                if let Some(ref s) = symbol
                    && e.symbol.as_deref() != Some(s.as_str())
                {
                    return false;
                }
            }
            if let Some(a) = agent
                && e.agent.type_ != a
            {
                return false;
            }
            true
        })
        .collect();

    if json {
        let items: Vec<_> = filtered.iter().map(|e| {
            serde_json::json!({
                "id": e.id,
                "agent": { "type": e.agent.type_, "cwd": e.agent.cwd },
                "op": e.op,
                "file": e.file,
                "symbol": e.symbol,
                "commit": e.commit,
                "ts": e.ts,
            })
        }).collect();
        println!("{}", serde_json::to_string(&serde_json::json!({ "entries": items })).unwrap());
        return Ok(());
    }

    if filtered.is_empty() {
        println!("no entries found");
        return Ok(());
    }

    for e in &filtered {
        let what = match &e.symbol {
            Some(s) => format!("{}:{}", e.file, s),
            None => e.file.clone(),
        };
        let commit_str = match &e.commit {
            Some(c) => format!(" (commit: {})", c),
            None => String::new(),
        };
        println!(
            "{} {} {} {}{}",
            e.ts, e.agent.type_, e.op, what, commit_str
        );
    }

    Ok(())
}
