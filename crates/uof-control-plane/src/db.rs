//! Database connection and migration handling.
//!
//! This module provides the database pool and runs migrations on startup.

use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Create a PostgreSQL connection pool.
///
/// # Arguments
///
/// * `database_url` - PostgreSQL connection URL, e.g., "postgres://user:pass@localhost/uof"
/// * `max_connections` - Maximum number of connections in the pool
///
/// # Errors
///
/// Returns an error if connection to the database fails.
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("failed to create database pool")?;

    Ok(pool)
}

/// Run database migrations.
///
/// Executes all SQL migration files in order.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    // Check if migrations already ran by looking for the agents table
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'agents')"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if table_exists {
        tracing::info!("Database already initialized, skipping migrations");
        return Ok(());
    }

    let migration_sql = include_str!("../sql/migrations/001_initial.sql");

    // Process line by line, collecting statements
    let mut current_stmt = String::new();

    for line in migration_sql.lines() {
        let trimmed = line.trim();

        // Skip single-line comments
        if trimmed.starts_with("--") {
            continue;
        }

        current_stmt.push_str(trimmed);
        current_stmt.push('\n');

        // If line ends with ';', we have a complete statement
        if trimmed.ends_with(';') {
            let final_stmt = current_stmt.trim().to_string();
            if !final_stmt.is_empty() {
                // Remove trailing semicolon for execution
                let exec_stmt = final_stmt.trim_end_matches(';');
                if !exec_stmt.trim().is_empty() {
                    tracing::debug!("Running: {}", &exec_stmt[..exec_stmt.len().min(80)]);
                    sqlx::query(exec_stmt)
                        .execute(pool)
                        .await
                        .context(format!("failed to execute: {}", &exec_stmt[..exec_stmt.len().min(50)]))?;
                }
            }
            current_stmt.clear();
        }
    }

    tracing::info!("Database migrations completed successfully");

    Ok(())
}

/// Check database connectivity.
pub async fn health_check(pool: &PgPool) -> Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .context("database health check failed")?;

    Ok(())
}