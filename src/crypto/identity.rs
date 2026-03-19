/// Derive actor_id from a 32-byte Ed25519 public key.
/// actor_id = blake3(pubkey_bytes)[0..32].to_hex() — 64 hex chars
pub fn actor_id_from_pubkey(pubkey: &[u8]) -> String {
    let hash = blake3::hash(pubkey);
    hex::encode(hash.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_id_length() {
        let pubkey = [0u8; 32];
        let actor_id = actor_id_from_pubkey(&pubkey);
        assert_eq!(actor_id.len(), 64);
    }

    #[test]
    fn test_actor_id_deterministic() {
        let pubkey = [42u8; 32];
        let id1 = actor_id_from_pubkey(&pubkey);
        let id2 = actor_id_from_pubkey(&pubkey);
        assert_eq!(id1, id2);
    }
}
