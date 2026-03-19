use std::path::PathBuf;

use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

use pan::crypto::{actor_id_from_pubkey, generate_keypair, hash_event, sign, HashInput};
use pan::types::EventType;

use crate::client::PanClient;

// ── disk paths ────────────────────────────────────────────────────────────────

pub fn pan_dir() -> PathBuf {
    dirs::home_dir()
        .expect("cannot determine home directory")
        .join(".pan")
}

pub fn identity_path() -> PathBuf {
    pan_dir().join("identity.json")
}

pub fn last_event_path() -> PathBuf {
    pan_dir().join("last_event.json")
}

// ── structs ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct Identity {
    pub actor_id: String,
    /// Hex-encoded raw 32-byte Ed25519 public key.
    pub pubkey: String,
    /// Hex-encoded 32-byte Ed25519 signing (secret) key — never leaves disk.
    pub secret_key: String,
    pub phone_dhash: String,
    pub server: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct LastEvent {
    pub event_id: String,
}

// ── disk I/O ──────────────────────────────────────────────────────────────────

pub fn load_identity() -> Result<Identity> {
    let path = identity_path();
    if !path.exists() {
        anyhow::bail!("no identity found. Run: pan-cli identity create");
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let identity: Identity = serde_json::from_str(&content)
        .with_context(|| "parsing identity.json")?;
    Ok(identity)
}

pub fn save_identity(identity: &Identity) -> Result<()> {
    let dir = pan_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    let content = serde_json::to_string_pretty(identity)?;
    std::fs::write(identity_path(), content)?;
    Ok(())
}

pub fn load_last_event() -> Option<String> {
    let path = last_event_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let last: LastEvent = serde_json::from_str(&content).ok()?;
    Some(last.event_id)
}

pub fn save_last_event(event_id: &str) -> Result<()> {
    let content = serde_json::to_string_pretty(&LastEvent {
        event_id: event_id.to_string(),
    })?;
    std::fs::write(last_event_path(), content)?;
    Ok(())
}

// ── crypto helpers ────────────────────────────────────────────────────────────

/// H(H(phone_bytes)) — the double-hash sybil gate.
pub fn hash_phone(phone: &str) -> String {
    let inner = blake3::hash(phone.trim().as_bytes());
    let outer = blake3::hash(inner.as_bytes());
    hex::encode(outer.as_bytes())
}

pub fn signing_key_from_identity(identity: &Identity) -> Result<SigningKey> {
    let bytes = hex::decode(&identity.secret_key)
        .with_context(|| "decoding secret_key")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("secret_key must be 32 bytes"))?;
    Ok(SigningKey::from_bytes(&arr))
}

// ── commands ──────────────────────────────────────────────────────────────────

pub async fn create(phone_arg: Option<&str>, server_arg: Option<&str>) -> Result<()> {
    let existing = identity_path();
    if existing.exists() {
        // In non-interactive mode (flags provided), skip the confirm prompt.
        if phone_arg.is_none() {
            let confirm = dialoguer::Confirm::new()
                .with_prompt("~/.pan/identity.json already exists. Overwrite?")
                .default(false)
                .interact()?;
            if !confirm {
                anyhow::bail!("aborted");
            }
        }
    }

    // 1. Generate keypair.
    let (signing_key, verifying_key) = generate_keypair();
    let pubkey_bytes = verifying_key.as_bytes().to_vec();
    let pubkey_hex = hex::encode(&pubkey_bytes);
    let secret_hex = hex::encode(signing_key.as_bytes());

    // 2. Derive actor_id.
    let actor_id = actor_id_from_pubkey(&pubkey_bytes);

    // 3. Phone number — flag or interactive prompt.
    let phone: String = match phone_arg {
        Some(p) => p.to_string(),
        None => dialoguer::Password::new()
            .with_prompt("Enter phone number (used once, never stored raw)")
            .interact()?,
    };

    // 4. Compute phone_dhash.
    let phone_dhash = hash_phone(&phone);

    // 5. Server URL — flag or interactive prompt.
    let server: String = match server_arg {
        Some(s) => s.to_string(),
        None => dialoguer::Input::new()
            .with_prompt("Server URL")
            .default("http://127.0.0.1:3000".to_string())
            .interact_text()?,
    };

    let created_at = chrono::Utc::now().timestamp_millis();

    // 6. Sign actor_id raw bytes.
    let id_bytes: [u8; 32] = hex::decode(&actor_id)
        .unwrap()
        .try_into()
        .unwrap();
    let signature = sign(&signing_key, &id_bytes);

    // 7. POST /actors.
    let client = PanClient::new(&server);
    let body = serde_json::json!({
        "actor_id": actor_id,
        "pubkey": pubkey_hex,
        "phone_dhash": phone_dhash,
        "signature": signature,
        "created_at": created_at
    });
    let _resp = client.post("/actors", &body).await
        .with_context(|| "registering actor with server")?;

    // 8. Save identity.
    let identity = Identity {
        actor_id: actor_id.clone(),
        pubkey: pubkey_hex,
        secret_key: secret_hex,
        phone_dhash,
        server,
        created_at,
    };
    save_identity(&identity)?;

    // 9. Compute ActorRegistered event_id and store as last_event (DAG root).
    let reg_event_id = hash_event(&HashInput {
        entity_id: &actor_id,
        event_type: EventType::ActorRegistered,
        timestamp: created_at,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    });
    save_last_event(&reg_event_id)?;

    println!("Identity created. Actor ID: {}", actor_id);
    Ok(())
}

pub fn show() -> Result<()> {
    let identity = load_identity()?;
    println!("Actor ID : {}", identity.actor_id);
    println!("Pubkey   : {}", identity.pubkey);
    println!("Server   : {}", identity.server);
    Ok(())
}
