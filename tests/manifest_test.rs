//! Validate mcp-server.toml parses, passes SDK validation, has the right tool
//! count, and gates the high-impact writes.

use adk_mcp_sdk::manifest::ServerManifest;
use std::path::Path;

fn manifest() -> ServerManifest {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("mcp-server.toml");
    ServerManifest::from_file(&path).expect("manifest should parse")
}

#[test]
fn manifest_parses_and_validates() {
    let m = manifest();
    assert!(m.validate().is_empty(), "validation errors: {:?}", m.validate());
    assert_eq!(m.server_id, "mcp_assessment");
    assert_eq!(m.domain, "education");
    assert_eq!(m.tools.len(), 23, "expected 23 declared tools");
}

#[test]
fn high_impact_writes_are_gated() {
    let m = manifest();
    for name in ["set_assessment_status", "submit_attempt"] {
        let t = m.tools.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name} present"));
        assert!(t.requires_approval, "{name} must require approval");
    }
}

#[test]
fn submit_attempt_is_external_write() {
    use adk_mcp_sdk::risk::RiskClass;
    let m = manifest();
    let t = m.tools.iter().find(|t| t.name == "submit_attempt").unwrap();
    assert_eq!(t.risk_class, RiskClass::ExternalWrite);
}

#[test]
fn analytics_reads_are_read_only() {
    use adk_mcp_sdk::risk::RiskClass;
    let m = manifest();
    for name in ["list_items", "items_for_standard", "assessment_blueprint", "item_analysis", "standard_mastery", "get_attempt", "audit_log"] {
        let t = m.tools.iter().find(|t| t.name == name).unwrap();
        assert_eq!(t.risk_class, RiskClass::ReadOnly, "{name} should be read_only");
    }
}
