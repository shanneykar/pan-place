# PAN — Physical Anchor Network
## CLAUDE.md — Layer-0 Build Context
### Single source of truth. All design decisions locked. No ambiguity.

---

## WHAT THIS FILE IS

This is the context file for Claude Code sessions building PAN layer-0.
Read this entire file before writing any code. Every decision here is final.
If something is marked "future" or "not layer-0", do not build it.
Do not stub it. Do not add columns or fields "for later."

---

## PROJECT PHILOSOPHY

PAN is an open protocol for verifying real-world interactions tied to physical locations.
It answers: did real people meet at a real place and did something real happen?

Core beliefs:
- Reality is the source of truth
- Trust emerges from patterns of confirmed activity, not from rules or ratings
- Silence is rejection — no explicit reject/dispute mechanism
- Events are immutable and append-only — no updates, no deletes, no revocation
- The protocol records. It does not adjudicate.
- Physical presence is the foundation. Confirmations are the structure.
- Node trust is earned, not granted — a QR on a park bench starts at zero

---

## WHAT LAYER-0 BUILDS

A Rust/Axum server that:
1. Registers Actors (keypair + phone double-hash)
2. Registers PAN Nodes (physical anchors at coordinates)
3. Writes Events to per-entity storage with Ed25519 signatures
4. Stores events in JSONL (source of truth) + SQLite (index/query)
5. Exposes 5 REST endpoints

That is the complete scope. Nothing else.

---

## LOCKED DESIGN DECISIONS

| Decision | Choice | Reason |
|----------|--------|--------|
| Language | Rust | Performance, memory safety |
| HTTP server | Axum 0.7 | Async, Tokio-native |
| Source of truth | JSONL per entity | Portable, P2P-ready, append-only |
| Query index | SQLite via sqlx | Fast lookups, deduplication |
| Hashing | BLAKE3 with domain separator + length prefixing | Frozen spec below |
| Signing | Ed25519 via ed25519-dalek v2 | Audited, widely used |
| Identity | Ed25519 keypair + phone double-hash | Keypair is root; phone is sybil gate |
| DAG parent field | `parent_hashes: Vec<String>` — array | Future-proofs for P2P/distributed |
| Layer-0 parent limit | Max 1 parent per event | Relaxed when distribution is built |
| Event content | Freeform `content` string + optional `tags` array | Human-readable, flexible |
| Confirmation model | Separate event referencing original by hash | One confirmer per confirmation event |
| Rejection model | None. Silence is rejection. | Unconfirmed events are weak signals |
| Event types | 4 only (see below) | Minimal primitives |
| Node at layer-0 | Record in DB with coordinates; events reference it | Node DAG is emergent from referencing events |

---

## FOUR EVENT TYPES — COMPLETE LIST

```rust
pub enum EventType {
    ActorRegistered,       // New keypair enters the network
    PanNodePlaced,         // New physical anchor registered
    PresenceRecorded,      // Actor taps PAN node — proves physical presence
    ConfirmationRecorded,  // "Yes this happened" — bilateral verification
}
```

No other event types exist in layer-0.
No rejection. No dispute. No revocation. No business registration.
No ownership transfer. No key recovery. No node archiving.

---

## DATA STRUCTURES

### Actor

```rust
pub struct Actor {
    pub actor_id: String,      // blake3(pubkey_bytes)[0..32].to_hex() — 64 hex chars
    pub pubkey: Vec<u8>,       // raw 32-byte Ed25519 public key
    pub phone_dhash: String,   // H(H(phone_number)) hex — sybil gate, never raw phone
    pub created_at: i64,       // millisecond epoch
}
```

`actor_id` derivation: `blake3::hash(&pubkey_bytes)` → first 32 bytes → hex encode → 64-char string.

### PanNode

```rust
pub struct PanNode {
    pub pan_id: String,         // blake3(lat || lon || placed_at)[0..8].to_hex() — 16 hex chars
    pub lat: f64,
    pub lon: f64,
    pub radius_miles: f64,      // default 1.0
    pub placed_at: i64,         // millisecond epoch
    pub node_type: NodeType,
    pub status: NodeStatus,
}

pub enum NodeType { Fixed, Ephemeral }
pub enum NodeStatus { Active, Archived }
```

### Event

```rust
pub struct Event {
    pub event_id: String,            // BLAKE3 hash hex — canonical spec below
    pub entity_id: String,           // actor_id or pan_id — whose DAG this belongs to
    pub event_type: EventType,       // one of four types
    pub timestamp: i64,              // millisecond epoch
    pub content: String,             // freeform human-readable description
    pub tags: Vec<String>,           // optional category labels, e.g. ["plumbing", "home_repair"]
    pub parent_hashes: Vec<String>,  // DAG parents; empty = root; max 1 in layer-0
    pub references_event: Option<String>, // for confirmations: hash of event being confirmed
    pub signature: String,           // Ed25519 sig over event_id bytes, hex
    pub actor_id: String,            // who created and signed this event
}
```

**Key field notes:**
- `content` — always present, can be empty string for registration events
- `tags` — always present as array, can be empty. Sorted before hashing.
- `references_event` — only used by ConfirmationRecorded. NULL for other types. Included in hash.
- `signature` — NOT included in hash. Hash is what gets signed.
- `event_id` — NOT an input. Always recomputed server-side and compared to submitted value.

---

## CANONICAL HASH CONSTRUCTION — FROZEN

This specification is immutable. Any deviation breaks cross-implementation compatibility.
A change here is a hard fork. Do not modify.

```
event_id = BLAKE3(
    b"PAN\x00"                                  // 4 bytes — domain separator
    || u32_be(entity_id.len())                  // 4 bytes
    || entity_id.as_bytes()                     // UTF-8
    || u32_be(event_type_str.len())             // 4 bytes — event_type as snake_case
    || event_type_str.as_bytes()                // UTF-8
    || i64_be(timestamp)                        // 8 bytes — big-endian signed
    || u32_be(content.len())                    // 4 bytes
    || content.as_bytes()                       // UTF-8 (NFC-normalized before hashing)
    || u32_be(tags.len())                       // 4 bytes — number of tags
    || for each t in tags.sort():               // tags MUST be sorted before hashing
         u32_be(t.len()) || t.as_bytes()
    || u32_be(parent_hashes.len())              // 4 bytes — number of parents
    || for each h in parent_hashes.sort():      // parents MUST be sorted before hashing
         u32_be(h.len()) || h.as_bytes()
    || references_event_or_zero                 // 32 zero bytes if None, else:
                                                // u32_be(ref.len()) || ref.as_bytes()
)
```

**CRITICAL — references_event encoding:**
- If `references_event` is `None`: write 32 zero bytes (0x00 × 32)
- If `references_event` is `Some(hash_str)`: write `u32_be(hash_str.len()) || hash_str.as_bytes()`

**Invariants:**
- Signature is NOT in the hash — hash is what gets signed
- `event_type_str` is snake_case: `"actor_registered"`, `"pan_node_placed"`, `"presence_recorded"`, `"confirmation_recorded"`
- Tags are sorted lexicographically before hashing
- Parent hashes are sorted lexicographically before hashing
- Content MUST be NFC-normalized (use `unicode-normalization` crate) before hashing
- All length prefixes are u32 big-endian

**Signature:**
```
signature = Ed25519_Sign(actor_private_key, hex_decode(event_id))
```
Sign the raw 32 bytes of the event_id hash, not the hex string.

**Verification:**
```
Ed25519_Verify(actor_pubkey, hex_decode(event_id), hex_decode(signature)) == true
```

---

## STORAGE — TWO LAYERS

### Layer 1: JSONL (Source of Truth)

One file per entity. Append-only. Never modified after write.

```
data/
├── actors/
│   └── {actor_id}.jsonl       // one JSON line per event in this actor's DAG
├── nodes/
│   └── {pan_id}.jsonl         // one JSON line per event referencing this node
└── index/
    └── pan.db                 // SQLite index database
```

Each line in a JSONL file is a complete Event struct serialized as JSON.
Files are the portable unit — to sync an entity's history, send their file.

### Layer 2: SQLite (Index/Query)

SQLite is the query layer. It is rebuildable from JSONL files.
If the SQLite database is deleted, it can be reconstructed by replaying all JSONL files.

**Schema:**

```sql
CREATE TABLE actors (
    actor_id    TEXT PRIMARY KEY,
    pubkey      BLOB NOT NULL,
    phone_dhash TEXT NOT NULL UNIQUE,
    created_at  INTEGER NOT NULL
);

CREATE TABLE pan_nodes (
    pan_id        TEXT PRIMARY KEY,
    lat           REAL NOT NULL,
    lon           REAL NOT NULL,
    radius_miles  REAL NOT NULL DEFAULT 1.0,
    placed_at     INTEGER NOT NULL,
    node_type     TEXT NOT NULL DEFAULT 'fixed',
    status        TEXT NOT NULL DEFAULT 'active'
);

CREATE TABLE events (
    event_id          TEXT PRIMARY KEY,
    entity_id         TEXT NOT NULL,
    event_type        TEXT NOT NULL,
    timestamp         INTEGER NOT NULL,
    content           TEXT NOT NULL DEFAULT '',
    tags              TEXT NOT NULL DEFAULT '[]',   -- JSON array of strings
    references_event  TEXT,                          -- NULL or event_id hash
    signature         TEXT NOT NULL,
    actor_id          TEXT NOT NULL
);

CREATE TABLE event_parents (
    event_id     TEXT NOT NULL REFERENCES events(event_id),
    parent_hash  TEXT NOT NULL,
    PRIMARY KEY (event_id, parent_hash)
);

-- Indexes for common queries
CREATE INDEX idx_events_entity_id ON events(entity_id);
CREATE INDEX idx_events_actor_id ON events(actor_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_event_type ON events(event_type);
CREATE INDEX idx_events_references ON events(references_event);
CREATE INDEX idx_event_parents_parent ON event_parents(parent_hash);
```

**Write path:** Every event write goes to BOTH JSONL and SQLite atomically.
JSONL append first, then SQLite insert. If SQLite fails, the JSONL line is the truth
and SQLite can be rebuilt.

**Read path:** All queries go through SQLite. JSONL is only read for full entity export.

---

## FIVE API ENDPOINTS — LAYER-0 SCOPE

### POST /actors

Register a new actor.

```json
{
  "actor_id": "hex64",
  "pubkey": "hex64 — raw 32-byte Ed25519 pubkey",
  "phone_dhash": "hex — H(H(phone))",
  "signature": "hex128 — Ed25519 sig over hex_decode(actor_id)",
  "created_at": 1234567890000
}
```

Validations:
1. `actor_id == blake3(hex_decode(pubkey))[0..32].to_hex()`
2. `phone_dhash` not already registered (UNIQUE constraint)
3. Signature valid: `Ed25519_Verify(pubkey, hex_decode(actor_id), hex_decode(signature))`
4. `created_at` is plausible: > 2020-01-01 epoch ms, < now + 5 minutes
5. `actor_id` not already registered

Response 201: `{ "actor_id": "...", "created_at": ... }`

Also writes an `ActorRegistered` event as the first entry in this actor's JSONL and SQLite.

### POST /nodes

Register a new PAN node.

```json
{
  "lat": 35.6762,
  "lon": 139.6503,
  "radius_miles": 1.0,
  "node_type": "fixed",
  "actor_id": "hex64",
  "signature": "hex128 — sig over blake3(lat_f64_be || lon_f64_be || placed_at_i64_be)"
}
```

Validations:
1. `actor_id` exists
2. `lat` in [-90, 90], `lon` in [-180, 180]
3. `placed_at` set to `now_ms()` server-side
4. Signature valid over `blake3(lat_f64_be || lon_f64_be || placed_at_i64_be)`
5. Derive `pan_id = blake3(lat_f64_be || lon_f64_be || placed_at_i64_be)[0..8].to_hex()`

Response 201: `{ "pan_id": "...", "placed_at": ... }`

Also writes a `PanNodePlaced` event.

### POST /events

Write event to an entity's DAG.

```json
{
  "event_id": "hex64",
  "entity_id": "actor_id or pan_id",
  "event_type": "presence_recorded",
  "timestamp": 1234567890000,
  "content": "Kitchen sink pipe replaced. Leak fixed under cabinet.",
  "tags": ["plumbing", "home_repair"],
  "parent_hashes": ["hex64"],
  "references_event": "hex64 or null",
  "signature": "hex128",
  "actor_id": "hex64"
}
```

Validations (in order):
1. `actor_id` exists in actors table
2. `entity_id` exists (actors OR pan_nodes)
3. All `parent_hashes` exist in events table
4. `parent_hashes` has at most 1 entry (layer-0 limit)
5. Recompute `event_id` from all fields — must match submitted value (HashMismatch if not)
6. Signature valid: `Ed25519_Verify(actor_pubkey, hex_decode(event_id), hex_decode(signature))`
7. `timestamp` > all parent event timestamps (TimestampNotForward)
8. If `event_type` is `confirmation_recorded`: `references_event` must be set and must exist
9. If `event_type` is `presence_recorded`: event must reference a valid pan_id in content or entity_id
10. Tags: all entries must be non-empty strings, max 10 tags, each max 50 chars
11. Content: max 2000 chars

If `event_id` already exists: return 200 `{ "event_id": "...", "status": "duplicate" }`. Not an error.

Response 201: `{ "event_id": "...", "status": "written" }`

### GET /actors/{actor_id}/events

Full event history for this actor.

Response 200:
```json
{
  "actor_id": "...",
  "events": [ /* Event objects, ordered by timestamp asc */ ]
}
```

Returns events where `entity_id == actor_id` OR `actor_id == actor_id`.

### GET /nodes/{pan_id}/events

All events touching this node.

Query params:
- `from` — millisecond epoch, optional
- `to` — millisecond epoch, optional
- `event_type` — optional filter

Response 200:
```json
{
  "pan_id": "...",
  "events": [ /* Event objects, ordered by timestamp asc */ ]
}
```

Returns events where `entity_id == pan_id` OR event references this pan_id.

---

## FILE STRUCTURE

```
pan/
├── Cargo.toml
├── CLAUDE.md                    ← this file
├── data/                        ← created at runtime
│   ├── actors/                  ← JSONL files, one per actor
│   ├── nodes/                   ← JSONL files, one per node
│   └── index/
│       └── pan.db               ← SQLite database
├── src/
│   ├── main.rs                  ← Axum server bootstrap
│   ├── lib.rs                   ← pub mod declarations
│   ├── error.rs                 ← PanError enum, IntoResponse impl
│   │
│   ├── types/
│   │   ├── mod.rs
│   │   ├── actor.rs             ← Actor struct
│   │   ├── node.rs              ← PanNode, NodeType, NodeStatus
│   │   └── event.rs             ← Event, EventType (4 variants)
│   │
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── hash.rs              ← hash_event() — canonical BLAKE3 spec
│   │   ├── sign.rs              ← sign(), verify() — Ed25519 wrappers
│   │   └── identity.rs          ← actor_id_from_pubkey()
│   │
│   ├── store/
│   │   ├── mod.rs               ← PanStore struct combining both layers
│   │   ├── jsonl.rs             ← append_event(), read_entity_events()
│   │   ├── sqlite.rs            ← SqlitePool init, migrations
│   │   ├── actors.rs            ← insert_actor, get_actor, phone_dhash_exists
│   │   ├── nodes.rs             ← insert_node, get_node, node_exists
│   │   └── events.rs            ← insert_event, get_events_*, event_exists
│   │
│   └── api/
│       ├── mod.rs               ← router() fn
│       ├── actors.rs            ← POST /actors
│       ├── nodes.rs             ← POST /nodes
│       ├── events.rs            ← POST /events
│       └── query.rs             ← GET endpoints
│
└── tests/
    ├── hash_vectors.rs          ← deterministic hash tests — MUST PASS FIRST
    └── e2e_lifecycle.rs         ← full lifecycle integration test
```

---

## ERROR HANDLING

```rust
pub enum PanError {
    ActorNotFound(String),           // 404
    NodeNotFound(String),            // 404
    EventNotFound(String),           // 404
    InvalidSignature,                // 400
    InvalidHash,                     // 400
    HashMismatch { computed, submitted }, // 400
    PhoneDhashAlreadyRegistered,     // 409
    ActorAlreadyExists(String),      // 409
    ParentNotFound(String),          // 400
    TooManyParents,                  // 400 — layer-0: max 1 parent
    InvalidTag(String),              // 400
    ContentTooLong,                  // 400
    TimestampNotForward,             // 400
    MissingReference,                // 400 — confirmation without references_event
    ReferenceNotFound(String),       // 400
    InvalidCoordinates,              // 400
    StorageError(String),            // 500
    DatabaseError(sqlx::Error),      // 500
}
```

All error responses: `{ "error": "error_code", "message": "human readable" }`

---

## SECURITY NOTES

- Server NEVER stores private keys
- Server NEVER stores raw phone numbers
- `phone_dhash` arrives pre-computed from client
- All signatures verified server-side before any write
- `event_id` always recomputed server-side and compared to submitted value
- JSONL files are append-only — no mechanism to modify or delete lines
- SQLite is rebuildable from JSONL — it is a cache, not the authority

---

## BUILD ORDER — STRICT SEQUENCE

### Session 1 — Types + Crypto (no I/O, no network)
- `types/actor.rs`, `types/node.rs`, `types/event.rs`
- `crypto/identity.rs` — `actor_id_from_pubkey()`
- `crypto/hash.rs` — `hash_event()` implementing canonical spec
- `crypto/sign.rs` — `sign()` and `verify()` wrappers
- `tests/hash_vectors.rs` — ALL vectors must pass

**GATE: Do not proceed to session 2 until hash vectors pass.**

### Session 2 — Storage (both layers)
- `store/jsonl.rs` — append event, read entity file
- `store/sqlite.rs` — pool init, run migration SQL
- `store/actors.rs`, `store/nodes.rs`, `store/events.rs` — CRUD
- `store/mod.rs` — PanStore struct combining JSONL + SQLite
- Integration test: write actor + event, read back from both stores, verify match

**GATE: Do not proceed to session 3 until round-trip storage test passes.**

### Session 3 — API
- `api/actors.rs` — POST /actors
- `api/nodes.rs` — POST /nodes
- `api/events.rs` — POST /events
- `api/query.rs` — GET endpoints
- `tests/e2e_lifecycle.rs` — full lifecycle test
- `main.rs` — Axum server bootstrap

**GATE: All tests pass. Layer-0 is complete.**

---

## GUARDRAILS — THINGS CLAUDE CODE MUST NOT DO

1. Do not add event types beyond the four specified
2. Do not add mutable fields or update/delete operations on events
3. Do not build trust computation, scoring, or ranking
4. Do not build peer gossip, sync, or federation
5. Do not build NFC/QR parsing
6. Do not build phone SMS verification
7. Do not build a web frontend
8. Do not add fields "for future use"
9. Do not use localStorage or any browser storage API
10. Do not add dependencies not listed in Cargo.toml
11. Do not change the hash construction spec
12. Do not use `axum::Server` — use `tokio::net::TcpListener` + `axum::serve`
13. Do not use `/:param` route syntax — use `/{param}`
14. Do not store raw GPS coordinates in confirmation events
15. Do not allow more than 1 parent hash per event in layer-0 validation
16. Tags and content are part of the hash — do not make them mutable metadata

---

## WHAT IS EXPLICITLY NOT IN LAYER-0

- Trust signals / computation
- Discovery queries / area search
- Peer gossip / distributed sync
- LanceDB (semantic search) — future
- DuckDB (analytics) — future
- Key recovery flow
- Business entity type
- Node archiving
- Offline queue
- Any web frontend
- Mobile app

---

## CLI CLIENT — Session 4

### What We Are Building

A command-line client binary `pan-cli` that talks to the PAN layer-0 server.
It generates keypairs locally, signs events locally, and sends them to the server.
Private keys never leave the local machine.

This is the reference client. If someone wants to build a mobile app or web client,
this CLI proves every flow works and shows exactly how to use the API.

---

### Binary Setup

Add to Cargo.toml:

```toml
[[bin]]
name = "pan-cli"
path = "src/cli/main.rs"

[dependencies]
# Add these to existing dependencies
clap = { version = "4", features = ["derive"] }
dirs = "5"
reqwest = { version = "0.12", features = ["json"] }
dialoguer = "0.11"
```

---

### Local Identity Storage

The CLI stores identity on disk. One file. Never uploaded.

```
~/.pan/
├── identity.json        # keypair + actor_id + server URL
```

`identity.json` structure:
```json
{
  "actor_id": "hex64",
  "pubkey": "hex64",
  "secret_key": "hex64",
  "phone_dhash": "hex64",
  "server": "http://127.0.0.1:3000",
  "created_at": 1234567890000
}
```

`secret_key` is the Ed25519 signing key. Stored locally only.
The CLI reads this file for every command that needs signing.
If file doesn't exist, most commands fail with: "No identity found. Run: pan-cli identity create"

---

### Commands

#### `pan-cli identity create`

Interactive. Generates everything needed to register.

```
1. Generate Ed25519 keypair
2. Derive actor_id from pubkey
3. Prompt: "Enter phone number (used once, never stored raw):"
4. Compute phone_dhash = blake3(blake3(phone_bytes))
5. Prompt: "Server URL [http://127.0.0.1:3000]:"
6. Sign actor_id with private key
7. POST /actors with { actor_id, pubkey, phone_dhash, signature, created_at }
8. On 201: save identity.json to ~/.pan/
9. Print: "Identity created. Actor ID: <id>"
```

If `~/.pan/identity.json` already exists, warn and ask to confirm overwrite.

#### `pan-cli identity show`

Prints actor_id, pubkey, server URL. Never prints secret_key.

#### `pan-cli node place --lat <f64> --lon <f64> [--radius <f64>] [--type fixed|ephemeral]`

```
1. Load identity
2. Set placed_at = now_ms()
3. Compute node_hash = blake3(lat_f64_be || lon_f64_be || placed_at_i64_be)
4. Sign node_hash with private key
5. POST /nodes with { lat, lon, radius_miles, node_type, actor_id, signature }
6. Print: "Node placed. PAN ID: <pan_id>"
```

Default radius: 1.0. Default type: fixed.

#### `pan-cli presence --node <pan_id>`

```
1. Load identity
2. GET node info (optional — just to verify node exists, but the server validates anyway)
3. Build PresenceRecorded event:
   - entity_id = actor_id (this goes on the actor's DAG)
   - content = "Presence at node <pan_id>"
   - tags = []
   - references_event = None
   - parent_hashes = [] (or last known event hash for this actor — see DAG chaining below)
4. Compute event_id via hash_event()
5. Sign event_id
6. POST /events
7. Print: "Presence recorded. Event: <event_id>"
```

#### `pan-cli event create --content <string> [--tags <comma,separated>] [--entity <id>]`

```
1. Load identity
2. Build event:
   - entity_id = --entity flag, or default to own actor_id
   - event_type = ConfirmationRecorded if a future confirm references it,
     but at creation time it's just a ConfirmationRecorded with no references_event.
     WAIT — this needs thought. See "Event creation model" below.
3. Compute event_id
4. Sign
5. POST /events
6. Print: "Event created. Event ID: <event_id>\nShare this ID with the other party to confirm."
```

#### `pan-cli confirm --event <event_id_hash>`

```
1. Load identity
2. Build ConfirmationRecorded event:
   - entity_id = own actor_id
   - content = "" (or optional --content flag)
   - tags = [] (or optional --tags flag)
   - references_event = the event_id being confirmed
   - parent_hashes = [] or last known event
3. Compute event_id
4. Sign
5. POST /events
6. Print: "Confirmed. Event: <event_id>"
```

#### `pan-cli history actor [--id <actor_id>]`

```
1. Load identity
2. If no --id flag, use own actor_id
3. GET /actors/<actor_id>/events
4. Print formatted table of events: timestamp, type, content, tags, confirmations count
```

#### `pan-cli history node --id <pan_id> [--from <epoch_ms>] [--to <epoch_ms>] [--type <event_type>]`

```
1. GET /nodes/<pan_id>/events with query params
2. Print formatted table
```

---

### Event Creation Model — Important

There's a subtlety here. In PAN, a "work event" and its "confirmation" are two separate events:

1. **Actor A creates the work event** — type: `ConfirmationRecorded`, content describes the work, tags categorize it, `references_event` is None. This is the claim.

2. **Actor B confirms it** — type: `ConfirmationRecorded`, content can be empty, `references_event` points to Actor A's event hash. This is the confirmation.

WAIT. This breaks the model. A `ConfirmationRecorded` without a `references_event` is a claim, not a confirmation. The server validates that `ConfirmationRecorded` must have `references_event` set.

**Resolution:** The initial work event is NOT a `ConfirmationRecorded`. It needs to be a plain event that describes what happened. But we only have 4 event types and none of them is "work happened."

**Two options:**

**Option A:** Use `PresenceRecorded` for the initial work event (it doesn't require references_event). Overload its meaning: presence at a location = something happened here. Then `ConfirmationRecorded` references it.

**Option B:** The work description IS the `ConfirmationRecorded` event, created by Actor A, with `references_event` pointing to... what? There's nothing to reference yet.

**Correct answer:** The initial event that describes work is a `PresenceRecorded` event. It says "I was here, this happened." The confirmation event is a `ConfirmationRecorded` that references it. This keeps the validation clean and the types meaningful:
- PresenceRecorded = "something happened here" (claim)
- ConfirmationRecorded = "yes it did" (verification, always references another event)

Update the CLI commands accordingly:
- `pan-cli event create` → creates a `PresenceRecorded` event
- `pan-cli confirm` → creates a `ConfirmationRecorded` referencing it

---

### DAG Chaining

Each actor's events form a chain. The CLI should track the last event_id it created.

Store in `~/.pan/last_event.json`:
```json
{
  "event_id": "hex64"
}
```

On every event write, update this file. On next event creation, use it as the single parent_hash.
If file doesn't exist (first event after registration), parent_hashes is empty (DAG root).

The ActorRegistered event created during `identity create` is the root.
Store its event_id as the initial last_event.

---

### Output Formatting

Keep it minimal and readable. No fancy TUI.

For history output:
```
TIMESTAMP            TYPE                  CONTENT                              TAGS             REFS
2026-03-18 14:22:01  presence_recorded     Tapped node at Midtown Plumbing                       —
2026-03-18 14:35:12  presence_recorded     Kitchen sink pipe replaced           plumbing          —
2026-03-18 14:36:00  confirmation_recorded                                                       a1b2c3...
```

Truncate content at 40 chars. Show first 8 chars of reference hash.

---

### File Structure

```
src/cli/
├── main.rs          ← clap App setup, subcommand dispatch
├── identity.rs      ← create, show, load from disk
├── commands/
│   ├── mod.rs
│   ├── node.rs      ← place
│   ├── presence.rs  ← presence
│   ├── event.rs     ← create
│   ├── confirm.rs   ← confirm
│   └── history.rs   ← actor history, node history
└── client.rs        ← HTTP client wrapper around reqwest, talks to server
```

---

### Error Handling

- Server returns error JSON: `{ "error": "code", "message": "human text" }`
- CLI prints: `Error: <message>` and exits with code 1
- Network errors: `Error: could not connect to server at <url>`
- Missing identity: `Error: no identity found. Run: pan-cli identity create`

---

### What NOT To Build

- No interactive mode / REPL
- No config file beyond identity.json and last_event.json
- No key export / import
- No multi-identity support
- No TUI or color output (keep it simple, pipe-friendly)
- No offline queue

---

### Test Plan

Write `tests/cli_integration.rs` or test manually with a running server.

Manual smoke test sequence:
```bash
# Terminal 1: start server
cargo run --bin pan

# Terminal 2: CLI commands
pan-cli identity create
pan-cli identity show
pan-cli node place --lat 35.2271 --lon -80.8431
pan-cli presence --node <pan_id from above>
pan-cli event create --content "Fixed kitchen sink" --tags plumbing,home_repair
pan-cli confirm --event <event_id from above>   # needs second identity — see below
pan-cli history actor
pan-cli history node --id <pan_id>
```

To test confirmation with two actors:
```bash
# Move first identity aside
mv ~/.pan ~/.pan-actor1

# Create second identity
pan-cli identity create

# Confirm the event from actor 1
pan-cli confirm --event <event_id>

# Check both histories
pan-cli history actor
mv ~/.pan ~/.pan-actor2
mv ~/.pan-actor1 ~/.pan
pan-cli history actor
```

---

### Session 4 Gate

All commands work. The manual smoke test sequence above completes without errors.
The full flow — register, place node, presence, create event, confirm with second actor, query histories — produces correct results.

Commit: `"session 4: CLI client, full protocol flow verified"`

---
*Compiled from PAN design sessions, March 2026.*
*All decisions final. Hash spec frozen. Event model locked.*
