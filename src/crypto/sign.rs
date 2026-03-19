use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::error::PanError;

/// Sign the raw 32 bytes of event_id hash with the given private key.
/// Returns hex-encoded signature (128 hex chars = 64 bytes).
pub fn sign(private_key: &SigningKey, event_id_bytes: &[u8; 32]) -> String {
    let signature = private_key.sign(event_id_bytes);
    hex::encode(signature.to_bytes())
}

/// Verify an Ed25519 signature.
/// - pubkey: raw 32-byte Ed25519 public key
/// - message: the raw bytes that were signed (event_id hash bytes)
/// - signature_hex: hex-encoded signature (128 chars)
pub fn verify(pubkey: &[u8], message: &[u8], signature_hex: &str) -> Result<(), PanError> {
    let pubkey_array: [u8; 32] = pubkey
        .try_into()
        .map_err(|_| PanError::InvalidSignature)?;

    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_array).map_err(|_| PanError::InvalidSignature)?;

    let sig_bytes = hex::decode(signature_hex).map_err(|_| PanError::InvalidSignature)?;

    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| PanError::InvalidSignature)?;

    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify(message, &signature)
        .map_err(|_| PanError::InvalidSignature)
}

/// Generate a new random keypair.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let (signing_key, verifying_key) = generate_keypair();
        let message = blake3::hash(b"test message");
        let message_bytes: [u8; 32] = *message.as_bytes();

        let signature = sign(&signing_key, &message_bytes);
        assert_eq!(signature.len(), 128);

        let result = verify(verifying_key.as_bytes(), &message_bytes, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_invalid_signature() {
        let (_, verifying_key) = generate_keypair();
        let message = blake3::hash(b"test message");
        let message_bytes = message.as_bytes();

        let bad_sig = "00".repeat(64);
        let result = verify(verifying_key.as_bytes(), message_bytes, &bad_sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_wrong_message() {
        let (signing_key, verifying_key) = generate_keypair();
        let message = blake3::hash(b"test message");
        let message_bytes: [u8; 32] = *message.as_bytes();

        let signature = sign(&signing_key, &message_bytes);

        let wrong_message = blake3::hash(b"different message");
        let result = verify(
            verifying_key.as_bytes(),
            wrong_message.as_bytes(),
            &signature,
        );
        assert!(result.is_err());
    }
}
