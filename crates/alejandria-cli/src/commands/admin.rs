//! Admin commands for API key management
//!
//! Implements P0-2: Multi-Key Support with Database Management
//!
//! Commands:
//! - generate-key: Create new API keys with optional expiration
//! - list-keys: List all API keys with filters
//! - revoke-key: Revoke a specific key by ID
//! - revoke-user: Revoke all keys for a specific user

use alejandria_storage::{api_keys, SqliteStore};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use std::env;

use crate::config::Config;

/// Generate a new API key
pub fn generate_key(
    user_id: &str,
    description: Option<&str>,
    expires_in_days: Option<i64>,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Get current system user as created_by (fallback to env USER or "cli")
    let created_by = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "cli".to_string());

    // Generate new API key via with_conn
    let (id, plaintext_key) = store
        .with_conn(|conn| {
            api_keys::create_api_key(conn, user_id, description, expires_in_days, &created_by)
        })
        .context("Failed to create API key")?;

    // Calculate expiration date for display
    let expires_at = expires_in_days.map(|days| Utc::now() + chrono::Duration::days(days));

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "user_id": user_id,
                "api_key": plaintext_key,
                "description": description,
                "created_at": Utc::now().to_rfc3339(),
                "expires_at": expires_at.map(|d| d.to_rfc3339()),
                "created_by": created_by,
            }))?
        );
    } else {
        println!("✅ API Key Generated Successfully");
        println!();
        println!("  ID:          {}", id);
        println!("  User ID:     {}", user_id);
        println!("  API Key:     {}", plaintext_key);

        if let Some(desc) = description {
            println!("  Description: {}", desc);
        }

        println!(
            "  Created:     {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(expires) = expires_at {
            println!("  Expires:     {}", expires.format("%Y-%m-%d %H:%M:%S UTC"));
        } else {
            println!("  Expires:     Never");
        }

        println!("  Created By:  {}", created_by);

        println!();
        println!("⚠️  IMPORTANT: Save this API key securely. It won't be shown again.");
        println!();
        println!("  To use this key, set the environment variable:");
        println!("  export ALEJANDRIA_API_KEY=\"{}\"", plaintext_key);
    }

    Ok(())
}

/// List all API keys with optional filters
pub fn list_keys(user_id: Option<&str>, include_revoked: bool, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // List API keys
    let keys = store
        .with_conn(|conn| {
            api_keys::list_api_keys(conn, include_revoked, true) // include_expired=true
        })
        .context("Failed to list API keys")?;

    // Filter by user_id if specified
    let filtered_keys: Vec<_> = if let Some(user) = user_id {
        keys.into_iter().filter(|k| k.username == user).collect()
    } else {
        keys
    };

    if json_output {
        let keys_json: Vec<_> = filtered_keys
            .iter()
            .map(|key| {
                json!({
                    "id": key.id,
                    "username": key.username,
                    "description": key.description,
                    "created_at": key.created_at.to_rfc3339(),
                    "expires_at": key.expires_at.map(|d| d.to_rfc3339()),
                    "last_used_at": key.last_used_at.map(|d| d.to_rfc3339()),
                    "revoked_at": key.revoked_at.map(|d| d.to_rfc3339()),
                    "usage_count": key.usage_count,
                    "status": key.status(),
                    "created_by": key.created_by,
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "total": filtered_keys.len(),
                "keys": keys_json,
            }))?
        );
    } else {
        if filtered_keys.is_empty() {
            println!("No API keys found.");
            return Ok(());
        }

        println!("API Keys ({} total):", filtered_keys.len());
        println!();

        for key in filtered_keys {
            let status_icon = match key.status() {
                "active" => "✅",
                "revoked" => "🚫",
                "expired" => "⏰",
                _ => "❓",
            };

            println!(
                "  {} {} (ID: {})",
                status_icon,
                key.status().to_uppercase(),
                key.id
            );
            println!("    Username:    {}", key.username);

            if let Some(desc) = &key.description {
                println!("    Description: {}", desc);
            }

            println!(
                "    Created:     {}",
                key.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!("    Created By:  {}", key.created_by);

            if let Some(expires_at) = key.expires_at {
                println!(
                    "    Expires:     {}",
                    expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }

            if let Some(last_used) = key.last_used_at {
                println!(
                    "    Last Used:   {}",
                    last_used.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }

            println!("    Usage Count: {}", key.usage_count);

            if let Some(revoked_at) = key.revoked_at {
                println!(
                    "    Revoked:     {}",
                    revoked_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }

            println!();
        }
    }

    Ok(())
}

/// Revoke a specific API key by ID
pub fn revoke_key(key_id: &str, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Revoke the key
    let result = store.with_conn(|conn| api_keys::revoke_api_key_by_id(conn, key_id));

    match result {
        Ok(()) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "success": true,
                        "key_id": key_id,
                        "message": "API key revoked successfully"
                    }))?
                );
            } else {
                println!("✅ API key {} revoked successfully", key_id);
            }
        }
        Err(e) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "success": false,
                        "key_id": key_id,
                        "error": format!("{}", e)
                    }))?
                );
            } else {
                println!("⚠️  Failed to revoke API key {}: {}", key_id, e);
            }
        }
    }

    Ok(())
}

/// Revoke all API keys for a specific user
pub fn revoke_user(user_id: &str, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Revoke all keys for user
    let count = store
        .with_conn(|conn| api_keys::revoke_api_keys_for_user(conn, user_id))
        .context("Failed to revoke API keys")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "user_id": user_id,
                "keys_revoked": count,
                "message": format!("{} API key(s) revoked for user {}", count, user_id)
            }))?
        );
    } else {
        if count > 0 {
            println!("✅ Revoked {} API key(s) for user {}", count, user_id);
        } else {
            println!("⚠️  No active API keys found for user {}", user_id);
        }
    }

    Ok(())
}
