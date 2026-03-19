use pan::crypto::{actor_id_from_pubkey, hash_event, sign, verify, HashInput};
use pan::types::EventType;

use ed25519_dalek::SigningKey;

/// Helper to create a deterministic keypair from a seed for testing.
fn keypair_from_seed(seed: &[u8; 32]) -> (SigningKey, [u8; 32]) {
    let signing_key = SigningKey::from_bytes(seed);
    let verifying_key = signing_key.verifying_key();
    (signing_key, *verifying_key.as_bytes())
}

// =============================================================================
// TEST VECTOR 1: Basic ActorRegistered event with empty content and no parents
// =============================================================================
#[test]
fn vector_01_actor_registered_minimal() {
    let input = HashInput {
        entity_id: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        event_type: EventType::ActorRegistered,
        timestamp: 1700000000000_i64,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let hash = hash_event(&input);

    assert_eq!(hash.len(), 64, "Hash must be 64 hex characters");
    // Hash is deterministic - same inputs always produce same output
    let hash2 = hash_event(&input);
    assert_eq!(hash, hash2, "Hash must be deterministic");
}

// =============================================================================
// TEST VECTOR 2: Event with content
// =============================================================================
#[test]
fn vector_02_with_content() {
    let input = HashInput {
        entity_id: "actor123",
        event_type: EventType::PresenceRecorded,
        timestamp: 1700000000000_i64,
        content: "Kitchen sink pipe replaced. Leak fixed under cabinet.",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let hash = hash_event(&input);
    assert_eq!(hash.len(), 64);

    // Different content must produce different hash
    let input2 = HashInput {
        content: "Different content",
        ..input
    };
    let hash2 = hash_event(&input2);
    assert_ne!(hash, hash2, "Different content must produce different hash");
}

// =============================================================================
// TEST VECTOR 3: Event with tags (verifies sorting)
// =============================================================================
#[test]
fn vector_03_with_tags_sorted() {
    let tags = vec![
        "plumbing".to_string(),
        "home_repair".to_string(),
        "urgent".to_string(),
    ];
    let input = HashInput {
        entity_id: "actor123",
        event_type: EventType::PresenceRecorded,
        timestamp: 1700000000000_i64,
        content: "Fixed the sink",
        tags: &tags,
        parent_hashes: &[],
        references_event: None,
    };
    let hash = hash_event(&input);

    // Same tags in different order must produce SAME hash (tags are sorted internally)
    let tags_reordered = vec![
        "urgent".to_string(),
        "home_repair".to_string(),
        "plumbing".to_string(),
    ];
    let input2 = HashInput {
        tags: &tags_reordered,
        ..input
    };
    let hash2 = hash_event(&input2);
    assert_eq!(hash, hash2, "Tag order must not affect hash (sorted internally)");
}

// =============================================================================
// TEST VECTOR 4: Event with single parent hash
// =============================================================================
#[test]
fn vector_04_with_parent() {
    let parent = "deadbeef".repeat(8);
    let parents = vec![parent.clone()];
    let input = HashInput {
        entity_id: "actor123",
        event_type: EventType::PresenceRecorded,
        timestamp: 1700000001000_i64,
        content: "Follow-up event",
        tags: &[],
        parent_hashes: &parents,
        references_event: None,
    };
    let hash = hash_event(&input);
    assert_eq!(hash.len(), 64);

    // Without parent must be different
    let input_no_parent = HashInput {
        parent_hashes: &[],
        ..input
    };
    let hash_no_parent = hash_event(&input_no_parent);
    assert_ne!(hash, hash_no_parent, "Parent hash must affect event hash");
}

// =============================================================================
// TEST VECTOR 5: ConfirmationRecorded with references_event
// =============================================================================
#[test]
fn vector_05_confirmation_with_reference() {
    let referenced = "cafebabe".repeat(8);
    let input = HashInput {
        entity_id: "actor123",
        event_type: EventType::ConfirmationRecorded,
        timestamp: 1700000002000_i64,
        content: "Confirmed the work was done",
        tags: &[],
        parent_hashes: &[],
        references_event: Some(&referenced),
    };
    let hash = hash_event(&input);
    assert_eq!(hash.len(), 64);

    // Without reference must be different (None gives 32 zero bytes)
    let input_no_ref = HashInput {
        references_event: None,
        ..input
    };
    let hash_no_ref = hash_event(&input_no_ref);
    assert_ne!(
        hash, hash_no_ref,
        "references_event must affect hash"
    );
}

// =============================================================================
// TEST VECTOR 6: PanNodePlaced event type
// =============================================================================
#[test]
fn vector_06_pan_node_placed() {
    let input = HashInput {
        entity_id: "node1234567890ab",
        event_type: EventType::PanNodePlaced,
        timestamp: 1700000000000_i64,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let hash = hash_event(&input);
    assert_eq!(hash.len(), 64);

    // Different event type must produce different hash
    let input2 = HashInput {
        event_type: EventType::ActorRegistered,
        ..input
    };
    let hash2 = hash_event(&input2);
    assert_ne!(hash, hash2, "Event type must affect hash");
}

// =============================================================================
// TEST VECTOR 7: Timestamp affects hash
// =============================================================================
#[test]
fn vector_07_timestamp_affects_hash() {
    let input = HashInput {
        entity_id: "actor123",
        event_type: EventType::ActorRegistered,
        timestamp: 1700000000000_i64,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let hash1 = hash_event(&input);

    let input2 = HashInput {
        timestamp: 1700000000001_i64,
        ..input
    };
    let hash2 = hash_event(&input2);

    assert_ne!(hash1, hash2, "Different timestamp must produce different hash");
}

// =============================================================================
// TEST VECTOR 8: Entity ID affects hash
// =============================================================================
#[test]
fn vector_08_entity_id_affects_hash() {
    let input = HashInput {
        entity_id: "actor_aaa",
        event_type: EventType::ActorRegistered,
        timestamp: 1700000000000_i64,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let hash1 = hash_event(&input);

    let input2 = HashInput {
        entity_id: "actor_bbb",
        ..input
    };
    let hash2 = hash_event(&input2);

    assert_ne!(hash1, hash2, "Different entity_id must produce different hash");
}

// =============================================================================
// TEST VECTOR 9: Unicode NFC normalization
// =============================================================================
#[test]
fn vector_09_unicode_nfc_normalization() {
    // é as single codepoint (U+00E9)
    let composed = "caf\u{00E9}";
    // é as e + combining acute accent (U+0065 U+0301)
    let decomposed = "cafe\u{0301}";

    let input1 = HashInput {
        entity_id: "actor123",
        event_type: EventType::PresenceRecorded,
        timestamp: 1700000000000_i64,
        content: composed,
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let hash1 = hash_event(&input1);

    let input2 = HashInput {
        content: decomposed,
        ..input1
    };
    let hash2 = hash_event(&input2);

    assert_eq!(
        hash1, hash2,
        "NFC-equivalent strings must produce same hash"
    );
}

// =============================================================================
// TEST VECTOR 10: actor_id_from_pubkey deterministic
// =============================================================================
#[test]
fn vector_10_actor_id_from_pubkey() {
    let seed = [1u8; 32];
    let (_, pubkey) = keypair_from_seed(&seed);

    let actor_id1 = actor_id_from_pubkey(&pubkey);
    let actor_id2 = actor_id_from_pubkey(&pubkey);

    assert_eq!(actor_id1.len(), 64, "actor_id must be 64 hex chars");
    assert_eq!(actor_id1, actor_id2, "actor_id must be deterministic");

    // Different pubkey must give different actor_id
    let seed2 = [2u8; 32];
    let (_, pubkey2) = keypair_from_seed(&seed2);
    let actor_id3 = actor_id_from_pubkey(&pubkey2);
    assert_ne!(actor_id1, actor_id3, "Different pubkey must give different actor_id");
}

// =============================================================================
// TEST VECTOR 11: Sign and verify round-trip
// =============================================================================
#[test]
fn vector_11_sign_verify_roundtrip() {
    let seed = [42u8; 32];
    let (signing_key, pubkey) = keypair_from_seed(&seed);

    let input = HashInput {
        entity_id: "actor123",
        event_type: EventType::ActorRegistered,
        timestamp: 1700000000000_i64,
        content: "",
        tags: &[],
        parent_hashes: &[],
        references_event: None,
    };
    let event_id_hex = hash_event(&input);
    let event_id_bytes: [u8; 32] = hex::decode(&event_id_hex)
        .expect("hash is valid hex")
        .try_into()
        .expect("hash is 32 bytes");

    let signature = sign(&signing_key, &event_id_bytes);
    assert_eq!(signature.len(), 128, "Signature must be 128 hex chars");

    let result = verify(&pubkey, &event_id_bytes, &signature);
    assert!(result.is_ok(), "Valid signature must verify");
}

// =============================================================================
// TEST VECTOR 12: Full event lifecycle (hash, sign, verify)
// =============================================================================
#[test]
fn vector_12_full_lifecycle() {
    // Create keypair
    let seed = [99u8; 32];
    let (signing_key, pubkey) = keypair_from_seed(&seed);
    let actor_id = actor_id_from_pubkey(&pubkey);

    // Create event with all fields populated
    let tags = vec!["test".to_string(), "vector".to_string()];
    let parent = "0123456789abcdef".repeat(4);
    let parents = vec![parent];
    let reference = "fedcba9876543210".repeat(4);

    let input = HashInput {
        entity_id: &actor_id,
        event_type: EventType::ConfirmationRecorded,
        timestamp: 1700000000000_i64,
        content: "Full lifecycle test with Unicode: 日本語 emoji: 🎉",
        tags: &tags,
        parent_hashes: &parents,
        references_event: Some(&reference),
    };

    // Hash
    let event_id = hash_event(&input);
    assert_eq!(event_id.len(), 64);

    // Sign
    let event_id_bytes: [u8; 32] = hex::decode(&event_id)
        .expect("valid hex")
        .try_into()
        .expect("32 bytes");
    let signature = sign(&signing_key, &event_id_bytes);

    // Verify
    let result = verify(&pubkey, &event_id_bytes, &signature);
    assert!(result.is_ok(), "Lifecycle signature must verify");

    // Verify with wrong key fails
    let seed2 = [100u8; 32];
    let (_, wrong_pubkey) = keypair_from_seed(&seed2);
    let wrong_result = verify(&wrong_pubkey, &event_id_bytes, &signature);
    assert!(wrong_result.is_err(), "Wrong key must fail verification");

    // Verify with tampered message fails
    let tampered = [0u8; 32];
    let tampered_result = verify(&pubkey, &tampered, &signature);
    assert!(tampered_result.is_err(), "Tampered message must fail verification");
}
