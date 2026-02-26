use sha2::{Digest, Sha256};

/// Generate a grits ID from the claim parameters.
///
/// Format: `gs-XXXX` where XXXX is 4 bytes of SHA-256 encoded as base36.
pub fn generate_id(file: &str, symbol: Option<&str>, agent_type: &str, cwd: &str, ts: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file.as_bytes());
    if let Some(s) = symbol {
        hasher.update(s.as_bytes());
    }
    hasher.update(agent_type.as_bytes());
    hasher.update(cwd.as_bytes());
    hasher.update(ts.as_bytes());

    let hash = hasher.finalize();
    let bytes = &hash[..4];
    let num = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

    format!("gs-{}", base36_encode(num))
}

fn base36_encode(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }

    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();

    while n > 0 {
        result.push(CHARS[(n % 36) as usize]);
        n /= 36;
    }

    result.reverse();
    String::from_utf8(result).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_has_gs_prefix() {
        let id = generate_id("src/lib.rs", Some("foo"), "claude", "/repo", "2026-01-01T00:00:00Z");
        assert!(id.starts_with("gs-"));
    }

    #[test]
    fn same_inputs_produce_same_id() {
        let a = generate_id("a.rs", Some("x"), "claude", "/r", "t1");
        let b = generate_id("a.rs", Some("x"), "claude", "/r", "t1");
        assert_eq!(a, b);
    }

    #[test]
    fn different_inputs_produce_different_ids() {
        let a = generate_id("a.rs", Some("x"), "claude", "/r", "t1");
        let b = generate_id("a.rs", Some("y"), "claude", "/r", "t1");
        assert_ne!(a, b);
    }
}
