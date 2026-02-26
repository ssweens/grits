use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

use crate::GritsError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentIdentity {
    #[serde(rename = "type")]
    pub type_: String,
    pub cwd: String,
}

/// Known agent process names mapped to their type string.
const KNOWN_AGENTS: &[(&str, &str)] = &[
    ("claude", "claude"),
    ("codex", "codex"),
    ("cursor", "cursor"),
    ("Cursor", "cursor"),
    ("windsurf", "windsurf"),
];

impl AgentIdentity {
    /// Detect agent identity by walking the process tree.
    ///
    /// Walks parent processes looking for a known agent name.
    /// Falls back to "human" if no agent is found (interactive terminal use).
    pub fn detect() -> Result<Self, GritsError> {
        let cwd = std::env::current_dir()
            .map_err(|e| GritsError::io(format!("failed to get cwd: {e}")))?
            .to_string_lossy()
            .to_string();

        let type_ = detect_agent_type();

        Ok(Self { type_, cwd })
    }
}

fn detect_agent_type() -> String {
    let sys = System::new_all();
    let pid = Pid::from_u32(std::process::id());

    let mut current = pid;
    loop {
        let Some(process) = sys.process(current) else {
            break;
        };

        let name = process.name().to_string_lossy();
        for &(pattern, agent_type) in KNOWN_AGENTS {
            if name.contains(pattern) {
                return agent_type.to_string();
            }
        }

        let Some(parent) = process.parent() else {
            break;
        };
        if parent == current {
            break; // avoid infinite loop at PID 0/1
        }
        current = parent;
    }

    "human".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_uses_process_tree() {
        let identity = AgentIdentity::detect().unwrap();
        // In a test context, should detect "claude" if run from Claude Code,
        // or "human" if run from a plain terminal
        assert!(!identity.type_.is_empty());
        assert!(!identity.cwd.is_empty());
    }
}
