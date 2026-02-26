use crate::store::IntentEntry;

/// Check if a target file:symbol conflicts with any active claims.
///
/// Returns the list of conflicting active claims.
///
/// Conflict rules:
/// - Same file + same symbol → conflict
/// - Same file + one has symbol=None (whole file claim) → conflict
/// - Different symbols in same file → no conflict
/// - Different files → no conflict
pub fn check_conflicts<'a>(
    file: &str,
    symbol: Option<&str>,
    active_claims: &'a [IntentEntry],
) -> Vec<&'a IntentEntry> {
    active_claims
        .iter()
        .filter(|claim| is_conflict(file, symbol, claim))
        .collect()
}

fn is_conflict(file: &str, symbol: Option<&str>, claim: &IntentEntry) -> bool {
    if claim.file != file {
        return false;
    }

    // Either side claims the whole file → conflict
    if symbol.is_none() || claim.symbol.is_none() {
        return true;
    }

    // Same symbol → conflict
    symbol == claim.symbol.as_deref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::AgentIdentity;

    fn claim(file: &str, symbol: Option<&str>) -> IntentEntry {
        IntentEntry {
            id: "gs-test".to_string(),
            agent: AgentIdentity { type_: "claude".to_string(), cwd: "/repo".to_string() },
            op: "claim".to_string(),
            file: file.to_string(),
            symbol: symbol.map(|s| s.to_string()),
            commit: None,
            ts: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn same_file_same_symbol_conflicts() {
        let claims = vec![claim("a.rs", Some("foo"))];
        let conflicts = check_conflicts("a.rs", Some("foo"), &claims);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn same_file_different_symbols_no_conflict() {
        let claims = vec![claim("a.rs", Some("foo"))];
        let conflicts = check_conflicts("a.rs", Some("bar"), &claims);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn different_files_no_conflict() {
        let claims = vec![claim("a.rs", Some("foo"))];
        let conflicts = check_conflicts("b.rs", Some("foo"), &claims);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn whole_file_claim_conflicts_with_symbol() {
        let claims = vec![claim("a.rs", None)];
        let conflicts = check_conflicts("a.rs", Some("foo"), &claims);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn symbol_claim_conflicts_with_whole_file_check() {
        let claims = vec![claim("a.rs", Some("foo"))];
        let conflicts = check_conflicts("a.rs", None, &claims);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn whole_file_vs_whole_file_conflicts() {
        let claims = vec![claim("a.rs", None)];
        let conflicts = check_conflicts("a.rs", None, &claims);
        assert_eq!(conflicts.len(), 1);
    }
}
