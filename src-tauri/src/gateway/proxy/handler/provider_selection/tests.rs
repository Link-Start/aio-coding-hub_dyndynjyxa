use super::resolve_session_bound_provider_id;
use crate::circuit_breaker;
use crate::{providers, session_manager};
use std::collections::HashMap;

fn ids(items: &[providers::ProviderForGateway]) -> Vec<i64> {
    items.iter().map(|p| p.id).collect()
}

fn insert_provider(db: &crate::db::Db, name: &str, enabled: bool) -> providers::ProviderSummary {
    providers::upsert(
        db,
        None,
        "claude",
        name,
        vec!["https://example.com".to_string()],
        "order",
        None, // auth_mode
        Some("k"),
        enabled,
        1.0,
        Some(100),
        None,
        None,
        None,
        Some("fixed"),
        Some("00:00:00"),
        None,
        None,
        None,
        None,
        None, // note
    )
    .expect("insert provider")
}

#[test]
fn resolve_session_bound_provider_id_skips_disabled_bound_provider() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    providers::set_enabled(&db, id1, false).expect("disable provider 1");

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        None,
        &mut enabled,
        Some(&order),
    );

    // Disabled provider must NOT be re-inserted; fall through to next enabled provider
    assert_eq!(selected, None);
    assert_eq!(ids(&enabled), vec![id2]);
}

#[test]
fn resolve_session_bound_provider_id_skips_insertion_when_forced_provider_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    providers::set_enabled(&db, id1, false).expect("disable provider 1");

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        Some(id2),
        &mut enabled,
        Some(&order),
    );

    assert_eq!(selected, None);
    assert_eq!(ids(&enabled), vec![id2]);
}

#[test]
fn resolve_session_bound_provider_id_does_not_insert_when_reuse_disabled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    providers::set_enabled(&db, id1, false).expect("disable provider 1");

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        false,
        None,
        &mut enabled,
        Some(&order),
    );

    assert_eq!(selected, None);
    assert_eq!(ids(&enabled), vec![id2]);
}

#[test]
fn resolve_session_bound_provider_id_clears_stale_binding_when_bound_provider_not_in_candidates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    // Simulate a mode/provider list that no longer contains the bound provider.
    let mut candidates =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    candidates.retain(|p| p.id == id2);
    assert_eq!(ids(&candidates), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        None,
        &mut candidates,
        Some(&order),
    );

    // Must NOT re-insert the stale provider; reuse should fall through.
    assert_eq!(selected, None);
    assert_eq!(ids(&candidates), vec![id2]);
    assert_eq!(session.get_bound_provider("claude", "sess_1", now), None);
}
