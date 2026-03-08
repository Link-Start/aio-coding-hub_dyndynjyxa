//! Usage: SQLite migration v29->v30 - Add OAuth columns to providers table.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v29_to_v30(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 30;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Add OAuth columns to providers table (idempotent — skip if column already exists).
    // Guard: in some test fixtures the providers table may not exist yet.
    let has_providers_table: bool = tx
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='providers'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if has_providers_table {
        let alter_statements = [
            "ALTER TABLE providers ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'api_key'",
            "ALTER TABLE providers ADD COLUMN oauth_provider_type TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_access_token TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_refresh_token TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_id_token TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_token_uri TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_client_id TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_client_secret TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_expires_at INTEGER",
            "ALTER TABLE providers ADD COLUMN oauth_email TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_last_refreshed_at INTEGER",
            "ALTER TABLE providers ADD COLUMN oauth_last_error TEXT",
            "ALTER TABLE providers ADD COLUMN oauth_refresh_lead_s INTEGER NOT NULL DEFAULT 3600",
        ];

        for stmt in &alter_statements {
            match tx.execute_batch(stmt) {
                Ok(()) => {}
                Err(e) if e.to_string().contains("duplicate column name") => {
                    // Column already exists (e.g. dev DB from a prior run) — skip.
                }
                Err(e) => {
                    return Err(format!("failed to alter providers table: {e}"));
                }
            }
        }
    }

    // Record migration (idempotent — skip if row already exists from a prior partial run).
    // Ensure schema_migrations table exists (may be absent if DB was created before v26).
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
    )
    .map_err(|e| format!("failed to create schema_migrations table: {e}"))?;
    let now = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, ?)",
        [VERSION, now],
    )
    .map_err(|e| format!("failed to insert schema_migrations row for v{VERSION}: {e}"))?;

    // Update user_version
    super::set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;

    Ok(())
}
