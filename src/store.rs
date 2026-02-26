use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::identity::AgentIdentity;
use crate::GritsError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEntry {
    pub id: String,
    pub agent: AgentIdentity,
    pub op: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub ts: String,
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Open or create the store at the given directory.
    /// Creates `.grits/intents.jsonl` inside `root` if it doesn't exist.
    pub fn open(root: &Path) -> Result<Self, GritsError> {
        let dir = root.join(".grits");
        fs::create_dir_all(&dir)
            .map_err(|e| GritsError::io(format!("failed to create .grits dir: {e}")))?;

        Ok(Self {
            path: dir.join("intents.jsonl"),
        })
    }

    /// Append a single entry to the JSONL file.
    pub fn append(&self, entry: &IntentEntry) -> Result<(), GritsError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| GritsError::io(format!("failed to open intents file: {e}")))?;

        let line = serde_json::to_string(entry)
            .map_err(|e| GritsError::io(format!("failed to serialize entry: {e}")))?;

        writeln!(file, "{line}")
            .map_err(|e| GritsError::io(format!("failed to write entry: {e}")))?;

        Ok(())
    }

    /// Read all entries from the JSONL file.
    pub fn read_all(&self) -> Result<Vec<IntentEntry>, GritsError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)
            .map_err(|e| GritsError::io(format!("failed to open intents file: {e}")))?;

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (i, line) in reader.lines().enumerate() {
            let line = line
                .map_err(|e| GritsError::io(format!("failed to read line {}: {e}", i + 1)))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry: IntentEntry = serde_json::from_str(trimmed)
                .map_err(|e| GritsError::io(format!("failed to parse line {}: {e}", i + 1)))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Return active claims: claims without a matching release (same id).
    pub fn active_claims(&self) -> Result<Vec<IntentEntry>, GritsError> {
        let entries = self.read_all()?;

        let released_ids: HashSet<String> = entries
            .iter()
            .filter(|e| e.op == "release")
            .map(|e| e.id.clone())
            .collect();

        let active: Vec<IntentEntry> = entries
            .into_iter()
            .filter(|e| e.op == "claim" && !released_ids.contains(&e.id))
            .collect();

        Ok(active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        (dir, store)
    }

    fn make_entry(id: &str, op: &str, file: &str, symbol: Option<&str>) -> IntentEntry {
        IntentEntry {
            id: id.to_string(),
            agent: AgentIdentity {
                type_: "test".to_string(),
                cwd: "/tmp".to_string(),
            },
            op: op.to_string(),
            file: file.to_string(),
            symbol: symbol.map(|s| s.to_string()),
            commit: None,
            ts: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn empty_store_returns_no_entries() {
        let (_dir, store) = temp_store();
        assert!(store.read_all().unwrap().is_empty());
        assert!(store.active_claims().unwrap().is_empty());
    }

    #[test]
    fn append_and_read_round_trips() {
        let (_dir, store) = temp_store();
        let entry = make_entry("gs-0001", "claim", "src/lib.rs", Some("foo"));
        store.append(&entry).unwrap();

        let entries = store.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "gs-0001");
        assert_eq!(entries[0].symbol.as_deref(), Some("foo"));
    }

    #[test]
    fn active_claims_excludes_released() {
        let (_dir, store) = temp_store();
        store.append(&make_entry("gs-0001", "claim", "a.rs", Some("foo"))).unwrap();
        store.append(&make_entry("gs-0002", "claim", "b.rs", Some("bar"))).unwrap();
        store.append(&make_entry("gs-0001", "release", "a.rs", Some("foo"))).unwrap();

        let active = store.active_claims().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "gs-0002");
    }
}
