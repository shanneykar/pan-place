use crate::types::EventType;
use unicode_normalization::UnicodeNormalization;

/// Input for computing event_id hash.
/// Does not include signature (signature is over the hash).
pub struct HashInput<'a> {
    pub entity_id: &'a str,
    pub event_type: EventType,
    pub timestamp: i64,
    pub content: &'a str,
    pub tags: &'a [String],
    pub parent_hashes: &'a [String],
    pub references_event: Option<&'a str>,
}

/// Compute the canonical event_id hash per the frozen spec.
///
/// ```text
/// event_id = BLAKE3(
///     b"PAN\x00"                                  // 4 bytes — domain separator
///     || u32_be(entity_id.len())
///     || entity_id.as_bytes()
///     || u32_be(event_type_str.len())
///     || event_type_str.as_bytes()
///     || i64_be(timestamp)
///     || u32_be(content.len())                    // content is NFC-normalized
///     || content.as_bytes()
///     || u32_be(tags.len())                       // number of tags
///     || for each t in tags.sort():
///          u32_be(t.len()) || t.as_bytes()
///     || u32_be(parent_hashes.len())
///     || for each h in parent_hashes.sort():
///          u32_be(h.len()) || h.as_bytes()
///     || references_event_or_zero                 // 32 zero bytes if None
/// )
/// ```
pub fn hash_event(input: &HashInput) -> String {
    let mut hasher = blake3::Hasher::new();

    // Domain separator: b"PAN\x00" (4 bytes)
    hasher.update(b"PAN\x00");

    // entity_id
    let entity_bytes = input.entity_id.as_bytes();
    hasher.update(&(entity_bytes.len() as u32).to_be_bytes());
    hasher.update(entity_bytes);

    // event_type as snake_case string
    let event_type_str = input.event_type.as_str();
    let event_type_bytes = event_type_str.as_bytes();
    hasher.update(&(event_type_bytes.len() as u32).to_be_bytes());
    hasher.update(event_type_bytes);

    // timestamp as i64 big-endian
    hasher.update(&input.timestamp.to_be_bytes());

    // content (NFC-normalized)
    let content_normalized: String = input.content.nfc().collect();
    let content_bytes = content_normalized.as_bytes();
    hasher.update(&(content_bytes.len() as u32).to_be_bytes());
    hasher.update(content_bytes);

    // tags (sorted, then length-prefixed)
    let mut sorted_tags: Vec<&String> = input.tags.iter().collect();
    sorted_tags.sort();
    hasher.update(&(sorted_tags.len() as u32).to_be_bytes());
    for tag in sorted_tags {
        let tag_bytes = tag.as_bytes();
        hasher.update(&(tag_bytes.len() as u32).to_be_bytes());
        hasher.update(tag_bytes);
    }

    // parent_hashes (sorted, then length-prefixed)
    let mut sorted_parents: Vec<&String> = input.parent_hashes.iter().collect();
    sorted_parents.sort();
    hasher.update(&(sorted_parents.len() as u32).to_be_bytes());
    for parent in sorted_parents {
        let parent_bytes = parent.as_bytes();
        hasher.update(&(parent_bytes.len() as u32).to_be_bytes());
        hasher.update(parent_bytes);
    }

    // references_event: 32 zero bytes if None, else length-prefixed string
    match input.references_event {
        None => {
            hasher.update(&[0u8; 32]);
        }
        Some(ref_str) => {
            let ref_bytes = ref_str.as_bytes();
            hasher.update(&(ref_bytes.len() as u32).to_be_bytes());
            hasher.update(ref_bytes);
        }
    }

    let hash = hasher.finalize();
    hex::encode(hash.as_bytes())
}

/// Hash for node placement signature verification.
/// pan_id = blake3(lat_f64_be || lon_f64_be || placed_at_i64_be)[0..8].to_hex()
pub fn hash_node_placement(lat: f64, lon: f64, placed_at: i64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&lat.to_be_bytes());
    hasher.update(&lon.to_be_bytes());
    hasher.update(&placed_at.to_be_bytes());
    *hasher.finalize().as_bytes()
}

/// Derive pan_id from coordinates and timestamp.
/// pan_id = blake3(lat || lon || placed_at)[0..8].to_hex() — 16 hex chars
pub fn derive_pan_id(lat: f64, lon: f64, placed_at: i64) -> String {
    let hash = hash_node_placement(lat, lon, placed_at);
    hex::encode(&hash[0..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_event_deterministic() {
        let input = HashInput {
            entity_id: "abc123",
            event_type: EventType::ActorRegistered,
            timestamp: 1700000000000,
            content: "",
            tags: &[],
            parent_hashes: &[],
            references_event: None,
        };
        let h1 = hash_event(&input);
        let h2 = hash_event(&input);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_pan_id_length() {
        let pan_id = derive_pan_id(35.6762, 139.6503, 1700000000000);
        assert_eq!(pan_id.len(), 16);
    }
}
