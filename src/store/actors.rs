use sqlx::{Row, SqlitePool};

use crate::error::PanError;
use crate::types::Actor;

pub async fn insert_actor(pool: &SqlitePool, actor: &Actor) -> Result<(), PanError> {
    sqlx::query(
        "INSERT INTO actors (actor_id, pubkey, phone_dhash, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&actor.actor_id)
    .bind(&actor.pubkey)
    .bind(&actor.phone_dhash)
    .bind(actor.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_actor(pool: &SqlitePool, actor_id: &str) -> Result<Actor, PanError> {
    let row = sqlx::query(
        "SELECT actor_id, pubkey, phone_dhash, created_at FROM actors WHERE actor_id = ?",
    )
    .bind(actor_id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Err(PanError::ActorNotFound(actor_id.to_string())),
        Some(row) => Ok(Actor {
            actor_id: row.get("actor_id"),
            pubkey: row.get("pubkey"),
            phone_dhash: row.get("phone_dhash"),
            created_at: row.get("created_at"),
        }),
    }
}

pub async fn actor_exists(pool: &SqlitePool, actor_id: &str) -> Result<bool, PanError> {
    let row = sqlx::query("SELECT 1 FROM actors WHERE actor_id = ?")
        .bind(actor_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn phone_dhash_exists(pool: &SqlitePool, phone_dhash: &str) -> Result<bool, PanError> {
    let row = sqlx::query("SELECT 1 FROM actors WHERE phone_dhash = ?")
        .bind(phone_dhash)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}
