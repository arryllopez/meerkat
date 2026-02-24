# MEERKAT
## Real-Time Collaborative Scene Layout for Blender
**Product Requirements Document — MVP v2.0**
**February 2026**

> *"Every 3D object is an actor. Animators are directors. Meerkat puts all the directors on the same stage."*

---

## 1. Executive Summary

Meerkat is a real-time collaborative scene layout system for Blender. It enables multiple users to edit a shared 3D scene simultaneously — placing objects, positioning cameras, adjusting lights, and arranging assets from a shared library together in real-time.

This is a distributed systems project disguised as a 3D plugin. The goal is to build a stable, demonstrable, technically impressive collaboration engine — not a production-ready collaborative 3D platform.

### 1.1 Origin

Born from a real experience: collaborating on a 3D animation over voice chat, where one person described positions, sizes, and lighting directions while the other manually placed everything. Meerkat eliminates that friction.

### 1.2 Design Philosophy

Meerkat follows the same architecture as Figma: shared asset libraries live locally on every client. The network only carries lightweight events — object references, transforms, and presence data. Heavy geometry and materials never touch the wire.

---

## 2. Problem Statement

Blender is a single-user tool. Teams working on scene layout today collaborate by passing `.blend` files manually, using version control, working sequentially, or describing scene changes verbally over calls.

This introduces merge conflicts, context switching, miscommunication, and workflow friction that scales linearly with team size. A simple instruction like "move it 2 units to the left" becomes a back-and-forth that wastes minutes per adjustment.

Meerkat enables real-time collaborative scene layout through a centralized authoritative Rust backend and synchronized Blender client plugins.

---

## 3. Target User

**Primary:** Small animation teams (2–5 people) where one or more people are responsible for scene composition — placing props, setting up cameras, positioning lights, and arranging the spatial layout of a shot or sequence.

**Specific workflow:** Art director + scene builder working simultaneously. The art director moves a camera, the scene builder adjusts prop placement, both see changes instantly.

**Secondary:** Solo animators collaborating with remote teammates on scene blocking and previsualization.

---

## 4. Product Goals (MVP)

### 4.1 Primary Goal
Enable 2+ users to collaboratively lay out the same Blender scene in real time — placing shared library assets, cameras, and lights, and transforming them with immediate visibility to all participants.

### 4.2 Secondary Goals
- Demonstrate low-latency sync (<250ms perceived delay)
- Avoid scene corruption or state divergence
- Provide deterministic state resolution
- Be stable and polished enough for live demo
- Be architecturally sound enough to serve as a credible portfolio project

---

## 5. Strict Non-Goals (Scope Control)

The following are explicitly **OUT OF SCOPE** for MVP:
- Vertex-level mesh editing or sync
- Material, shader, or texture syncing
- Modifier syncing
- Animation keyframe syncing
- Rigging or armature syncing
- Voice or text chat
- Cloud persistence or database storage
- Session replay or undo history sync
- Offline mode
- CRDT-based merge algorithms
- Authentication or access control
- Multi-scene or multi-file support

> **Conflict resolution strategy: Last Write Wins (LWW) only. No merge attempts. Every feature not listed in sections 6–7 is out of scope.**

---

## 6. System Architecture

### 6.1 High-Level Design

```
Blender Client A (Python Plugin)
    ↕ WebSocket (WSS in production)
Rust Authoritative Server (tokio + axum)
    ↕ WebSocket (WSS in production)
Blender Client B (Python Plugin)
    ↕ WebSocket (WSS in production)
Blender Client N...
```

The server is authoritative. Clients do not attempt to resolve conflicts locally. All state mutations flow through the server, which broadcasts canonical state to all connected clients.

### 6.2 Asset Library Model

All collaborators share the same `.blend` asset file(s) locally. When a user places an asset, the network message contains only the asset identifier and transform — not the mesh data. Receiving clients instantiate the asset from their local library copy.

```
Shared Asset Library (assets.blend)
├─ dragon_character
├─ oak_tree
├─ hero_chair
├─ street_lamp
└─ building_facade_01

Network message: { type: "asset_ref", asset_id: "oak_tree", transform: { pos, rot, scale } }

Heavy geometry NEVER touches the wire.
```

---

## 7. Core Functional Requirements

### 7.1 Session Management

Users connect to the server via WebSocket. Users join a named session (room ID) via the Blender UI panel. On join, the client receives a full snapshot of the current scene state (`FULL_STATE_SYNC`). The server tracks all active sessions independently.

**Session Persistence:** Sessions survive user disconnection. A session and its state persist on the server as long as the server is running. Rejoining restores the full scene. Sessions are destroyed only when explicitly closed by the host or when the server shuts down (with event log enabling recovery on restart).

**Acceptance Criteria:**
- Client A creates/joins session `"shot-01"`
- Client B joins session `"shot-01"`
- Both see an identical scene
- Both disconnect and rejoin later — scene state is preserved

### 7.2 Asset Library System

Collaborators share a common asset library — one or more `.blend` files containing named objects. All participants must have the same asset library files stored locally.

**Requirements:**
- Plugin configuration includes a file path to one or more asset library `.blend` files
- UI panel displays a dropdown of available assets from loaded libraries
- When a user places an asset, the plugin sends `CREATE_OBJECT` with type `"asset_ref"`, the `asset_id` (object name in the library), the library filename, and the transform
- Receiving clients look up the `asset_id` in their local copy of the library, **link** it into the scene (not append — linked objects retain a live reference to the source file so asset updates propagate automatically on library reload), assign the synced UUID, and apply the transform
- If a client is missing the asset library or the `asset_id` is not found, display a placeholder bounding box with the object name and log a warning in the UI panel

**Acceptance Criteria:**
- User A selects `"dragon_character"` from the asset dropdown → all other users see the full dragon model appear at the correct position within 250ms
- Transform sync on asset objects works identically to primitives
- A client missing the asset file sees a labeled placeholder box instead of the model

### 7.3 Object Creation Sync

**Supported Object Types (MVP):**

| Category | Types |
|---|---|
| Primitives | Cube, Sphere, Cylinder |
| Cameras | Perspective Camera |
| Lights | Point Light, Sun Light (Directional) |
| Asset References | Any named object from the shared asset library |

**When a user creates an object, the plugin must:**
1. Generate a UUID for the object
2. Capture object type (`primitive`, `camera`, `light`, or `asset_ref`)
3. Capture transform (position, rotation, scale)
4. Capture type-specific properties (see section 8.1)
5. For asset references: capture `asset_id` and library filename
6. Send `CREATE_OBJECT` event to server

**Server must:**
- Validate the event
- Store object in canonical state
- Broadcast creation event to all other clients in the session

**Other clients must:**
- Instantiate the correct object type (for `asset_ref`: append from local library)
- Assign the same UUID
- Apply transform and type-specific properties

**Acceptance Criteria:**
- User A creates a point light → User B sees it appear within 250ms with correct position and properties
- User A places `"oak_tree"` from the asset library → User B sees the full tree model appear with correct transform

### 7.4 Object Deletion Sync

When a user deletes an object, the client sends a `DELETE_OBJECT` event with the object's UUID. The server removes the object from canonical state and broadcasts the deletion. Other clients delete the matching object. If the UUID doesn't exist locally, the event is silently ignored.

**Acceptance Criteria:**
- Deletion propagates to all clients without orphan objects
- Deleting an already-deleted object does not cause errors

### 7.5 Transform Synchronization

Transforms include position (x, y, z), rotation (Euler x, y, z), and scale (x, y, z). These apply to all object types equally.

**Requirements:**
- Plugin detects transform changes via polling/diffing at a fixed interval
- Sends `UPDATE_TRANSFORM` event with the object UUID and full transform
- Server overwrites the object's transform in canonical state (LWW)
- Server broadcasts updated transform to all other clients
- Receiving clients apply transform immediately without re-emitting an event (echo suppression)

**Throttling Constraint:** Transform updates are throttled client-side to a maximum of 30 updates per second per object. The server may additionally coalesce rapid updates from the same client for the same object.

**Acceptance Criteria:**
- User A moves an object → User B sees smooth, near-real-time movement
- Rapid dragging does not flood the server or cause jitter on other clients

### 7.6 Object Property Synchronization

| Object Type | Synced Properties |
|---|---|
| Camera | Focal length (mm), sensor width, clip start, clip end |
| Point Light | Color (RGB), power (watts), radius |
| Sun Light | Color (RGB), intensity, angle |
| Primitives | No additional properties beyond transform |
| Asset References | No additional properties beyond transform |

When a synced property changes, the plugin sends an `UPDATE_PROPERTIES` event. The server overwrites properties (LWW) and broadcasts to other clients. Property updates are throttled identically to transform updates.

**Acceptance Criteria:**
- User A changes a light's color → User B sees the change within 250ms
- User A adjusts camera focal length → User B sees the update

### 7.7 Object Naming / Labeling

Users can rename objects within the collaborative session. The plugin sends an `UPDATE_NAME` event with the UUID and new name string. The server overwrites the name (LWW) and broadcasts to all clients.

**Acceptance Criteria:**
- User A renames `"Cube.003"` to `"hero_chair"` → User B sees the name update in Blender's outliner

### 7.8 User Presence

**Server tracks:**
- Connected users per session
- User IDs (self-assigned display names on join)
- Join/leave events
- Currently selected object per user

**Clients display:**
- Connected users list in the Blender UI panel
- Each user is assigned a unique color on join

**Selection Presence (MVP target, deprioritize if time-constrained):**
- When a user selects an object, their selection is broadcast
- Other clients see a colored outline or bounding box on the selected object with the user's name label
- When the user deselects, the highlight is removed

**Acceptance Criteria:**
- User A joins → User B sees "User A" appear in the users panel
- User A disconnects → User B sees "User A" removed
- (Stretch) User A selects an object → User B sees a colored highlight

### 7.9 Scene Export

Any connected user can save the current collaborative scene state as a local `.blend` file at any time.

**When a user clicks "Save Scene":**
1. Plugin requests `FULL_STATE_SYNC` from server to ensure local state is current
2. On confirmation, plugin calls Blender's native save operator to write a `.blend` file to the user's chosen path
3. The saved file is a standalone snapshot — it has no ongoing connection to the session

The `.blend` file opens correctly in standalone Blender with no Meerkat plugin required.

**Acceptance Criteria:**
- User clicks Save Scene → receives a valid `.blend` file containing all objects, cameras, lights, transforms, names, and asset library objects
- The `.blend` file opens in standalone Blender without the Meerkat plugin
- Saving does not interrupt the active session

---

## 8. Data Model Specification

### 8.1 Object Model (Server-Side Canonical State)

```json
{
  "object_id": "uuid-v4",
  "name": "string",
  "type": "cube | sphere | cylinder | camera | point_light | sun_light | asset_ref",
  "asset_id": "string | null",
  "asset_library": "string | null",
  "transform": {
    "position": [x, y, z],
    "rotation": [x, y, z],
    "scale": [x, y, z]
  },
  "properties": {
    // Camera: focal_length, sensor_width, clip_start, clip_end
    // Point Light: color [r,g,b], power, radius
    // Sun Light: color [r,g,b], intensity, angle
  },
  "created_by": "user_id",
  "last_updated_by": "user_id",
  "last_updated_at": "unix_timestamp_ms"
}
```

### 8.2 Session Model

```json
{
  "session_id": "string (room name)",
  "objects": { "uuid": "ObjectModel" },
  "users": {
    "user_id": {
      "display_name": "string",
      "color": [r, g, b],
      "selected_object": "uuid | null",
      "connected_at": "timestamp"
    }
  },
  "event_log": []
}
```

### 8.3 Event Types

| Direction | Event | Payload |
|---|---|---|
| Client → Server | `CREATE_OBJECT` | object_id, name, type, asset_id?, transform, properties |
| Client → Server | `DELETE_OBJECT` | object_id |
| Client → Server | `UPDATE_TRANSFORM` | object_id, transform |
| Client → Server | `UPDATE_PROPERTIES` | object_id, properties |
| Client → Server | `UPDATE_NAME` | object_id, name |
| Client → Server | `SELECT_OBJECT` | object_id \| null |
| Client → Server | `JOIN_SESSION` | session_id, display_name |
| Client → Server | `LEAVE_SESSION` | (empty) |
| Server → Client | `FULL_STATE_SYNC` | Full session state snapshot |
| Server → Client | `OBJECT_CREATED` | Full object model, created_by |
| Server → Client | `OBJECT_DELETED` | object_id, deleted_by |
| Server → Client | `TRANSFORM_UPDATED` | object_id, transform, updated_by |
| Server → Client | `PROPERTIES_UPDATED` | object_id, properties, updated_by |
| Server → Client | `NAME_UPDATED` | object_id, name, updated_by |
| Server → Client | `USER_JOINED` | user_id, display_name, color |
| Server → Client | `USER_LEFT` | user_id |
| Server → Client | `USER_SELECTED` | user_id, object_id \| null |
| Server → Client | `ERROR` | code, message |

Every client→server event includes a `source_user_id` field. Clients ignore incoming events where `updated_by` matches their own ID to prevent echo loops.

---

## 9. Networking Protocol

**Transport:** WebSocket (WSS in production via TLS, WS on localhost during development)
**Format:** JSON (MVP). Binary protocol (MessagePack or Protobuf) is a post-MVP optimization.

The server is authoritative. Clients must never mutate local scene state without informing the server. If a client's local state diverges, it can request a `FULL_STATE_SYNC` to reset.

### 9.1 Message Envelope

```json
{
  "event_type": "string",
  "timestamp": 1708000000000,
  "source_user_id": "string",
  "payload": {}
}
```

---

## 10. Conflict Resolution (MVP)

### 10.1 Last Write Wins (LWW)

Every update carries a timestamp. The server overwrites previous state with the most recent update, then broadcasts canonical state. No merge attempts.

If two users transform the same object simultaneously, the most recent update wins. Simple, deterministic, sufficient for MVP.

### 10.2 Edge Case: CREATE/DELETE Race

If Client A creates object X and Client B sends DELETE for object X before receiving the CREATE (possible under network delay), the server treats DELETE of a nonexistent object as a no-op. The object persists.

---

## 11. Observability & Engineering Quality

### 11.1 Structured Logging

All server events logged with the `tracing` crate (Rust) using structured JSON format. Log fields: event type, session ID, user ID, object ID, timestamp, processing latency.

### 11.2 Metrics Endpoint

Expose an HTTP `/metrics` endpoint reporting:
- Active sessions count
- Active connections per session
- Messages processed per second
- Transform update throughput (messages/sec)
- Message propagation latency (p50, p95, p99)
- Event log size per session

### 11.3 Event Sourcing with Write-Ahead Log

Every state-mutating event is appended to an ordered event log per session. On server crash, state is reconstructed by replaying the event log. Periodic compaction: snapshot canonical state and truncate the log.

### 11.4 Property-Based Testing

Use `proptest` (Rust) to generate random sequences of `CREATE`, `DELETE`, `UPDATE_TRANSFORM`, and `UPDATE_PROPERTIES` operations. Assert that server state converges correctly regardless of operation ordering. Assert that replaying the event log produces identical state to live state.

### 11.5 Latency Benchmarking

A benchmark harness that spawns N simulated WebSocket clients, sends transform updates at configurable rates, and measures end-to-end propagation latency. Results output as a table for the README with p50, p95, and p99 latency numbers.

---

## 12. Performance Requirements

| Metric | Requirement |
|---|---|
| Transform propagation latency | <250ms perceived |
| Concurrent users per session | Minimum 5 |
| Transform update frequency | Capped at 30Hz per object per client |
| Server message throughput | Minimum 1,000 messages/second aggregate |
| Full state sync on join | <500ms for scenes with up to 200 objects |
| Crash recovery from event log | <5 seconds for 10,000 events |
| Stability | No crashes during concurrent edits |

---

## 13. Deployment Strategy

### 13.1 Overview

During development (Phases 1–6), the server runs locally via `cargo run` on `localhost:8080` with plain `ws://`. The cloud deployment happens in Phase 7 as part of the polish and launch work.

For MVP launch, the Meerkat server is deployed to a cloud platform (Fly.io preferred, GCP Cloud Run as alternative) with TLS termination and WSS support. A public hosted server URL is hardcoded as the default in the Blender plugin so users can connect with zero setup. Self-hosted Docker remains available for teams that want to run their own server.

### 13.2 Hosted Public Server

- **URL format:** `wss://meerkat.<yourdomain>.com`
- Hardcoded as the default `server_url` value in the Blender plugin preferences
- Users who want zero setup: open Blender, install the addon, enter room name, click Connect
- TLS provided by the platform (Fly.io via Let's Encrypt, auto-renewed)
- Free tier hosting is sufficient for MVP scale (~5 concurrent sessions, ~20 concurrent users)

### 13.3 Self-Hosted Option

```bash
docker-compose up
```

- Exposes port 8080 by default
- Users configure a custom server URL in plugin preferences to point at their own instance
- TLS termination is their own responsibility (nginx reverse proxy or platform-level)

### 13.4 Abuse Prevention (Public Server)

| Limit | Value | Enforcement |
|---|---|---|
| Max users per session | 10 | Server rejects `JOIN_SESSION` with `ERROR` when at capacity |
| Max active sessions | 20 | Server rejects `JOIN_SESSION` for new session names when global limit is hit |
| Connection rate limit | 10 new connections/second | axum middleware (tower rate limiter) |
| Message rate limit | 100 messages/second per connection | Per-connection token bucket on the server |

### 13.5 Deployment as Resume Signal

The deployment is a deliberate engineering credential: *"Containerized Rust WebSocket server deployed to Fly.io with TLS, structured logging, metrics endpoint, and WebSocket connection lifecycle management."* This differentiates the project from a toy localhost demo.

---

## 14. Technical Stack

| Component | Technology |
|---|---|
| Blender Plugin | Python 3.x (Blender's embedded Python) |
| WebSocket Client | `asyncio` + `websockets` library |
| Backend Server | Rust (`tokio` + `axum` + `tokio-tungstenite`) |
| State Storage | In-memory `HashMap` + file-backed event log |
| Serialization | JSON (`serde_json`) |
| Logging | `tracing` + `tracing-subscriber` (JSON format) |
| Testing | `cargo test` + `proptest` |
| Protocol | JSON over WebSocket |
| Containerization | Docker + Docker Compose (self-hosted option) |
| Cloud Deployment | Fly.io or GCP Cloud Run (TLS via Let's Encrypt) |
| Benchmarking | Custom Rust harness with simulated clients |

---

## 15. Architecture Decision Records

These should be written as short markdown files in `/docs/decisions/` in the repository.

- **ADR-001: Why LWW over CRDT** — LWW is simple, deterministic, and sufficient for scene layout where conflicts are infrequent and the cost of a wrong resolution is low. CRDTs add complexity that doesn't justify itself for MVP.
- **ADR-002: Why Rust over Go** — Rust's ownership model prevents data races at compile time. tokio provides a mature async runtime. The type system catches protocol-level bugs early.
- **ADR-003: Why JSON over binary protocol** — Human-readable, easy to debug, trivially parsed in Rust and Python. Bandwidth overhead is negligible at MVP scale.
- **ADR-004: Why authoritative server over P2P** — Single server eliminates split-brain, simplifies conflict resolution, provides single source of truth for late joiners.
- **ADR-005: Transform throttling strategy** — Client-side 30Hz cap. Math: 5 users × 30 updates/sec × ~200 bytes = ~30KB/s, trivially handled.
- **ADR-006: Why shared asset libraries over mesh sync** — Follows Figma's architecture. Network carries only identifiers and transforms. Heavy geometry stays local. Keeps messages tiny regardless of asset complexity.
- **ADR-007: Why Fly.io over self-hosted VPS** — Fly.io provides TLS termination, global anycast routing, and zero-config WebSocket support. One-command deploy (`flyctl deploy`). Free tier covers MVP scale. No ops overhead.

---

## 16. Implementation Phases

> **Convention:** Every step is written as a concrete engineering action. "Implement X" means write the code, run the tests, confirm it works. No step is considered done until it compiles and the acceptance criteria passes locally.

---

### Phase 1 — Rust Server Foundation
**Weeks 1–2 | Goal: A working WebSocket server that routes events between clients and maintains authoritative scene state in memory.**

#### 1.1 Project Setup
- [ ] Run `cargo new meerkat-server` and set up Cargo workspace with `meerkat-server` crate
- [ ] Add dependencies to `Cargo.toml`:
  ```toml
  tokio = { version = "1", features = ["full"] }
  axum = { version = "0.7", features = ["ws"] }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  uuid = { version = "1", features = ["v4"] }
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["json"] }
  dashmap = "5"
  ```
- [ ] Set up `tracing-subscriber` with JSON format in `main.rs`
- [ ] Confirm `cargo build` succeeds

#### 1.2 Data Model (Types)
- [ ] Define `Transform` struct: `position: [f64; 3]`, `rotation: [f64; 3]`, `scale: [f64; 3]`
- [ ] Define `ObjectType` enum: `Cube`, `Sphere`, `Cylinder`, `Camera`, `PointLight`, `SunLight`, `AssetRef`
- [ ] Define `ObjectProperties` enum covering per-type property structs (camera, point_light, sun_light)
- [ ] Define `SceneObject` struct matching the data model in section 8.1
- [ ] Define `User` struct: `display_name`, `color: [u8; 3]`, `selected_object: Option<Uuid>`, `connected_at`
- [ ] Define `Session` struct: `session_id`, `objects: HashMap<Uuid, SceneObject>`, `users: HashMap<String, User>`, `event_log: Vec<LogEntry>`
- [ ] Define `ServerState` as `Arc<DashMap<String, Session>>`
- [ ] All structs `#[derive(Serialize, Deserialize, Clone, Debug)]`

#### 1.3 Message Protocol
- [ ] Define `MessageEnvelope` struct: `event_type: String`, `timestamp: u64`, `source_user_id: String`, `payload: serde_json::Value`
- [ ] Define typed payload structs for every client→server event (section 8.3)
- [ ] Define typed payload structs for every server→client event
- [ ] Write a `parse_client_message(raw: &str) -> Result<ClientEvent>` function that deserializes the envelope and dispatches to the correct payload type
- [ ] Unit test: round-trip serialize/deserialize for each event type

#### 1.4 WebSocket Server & Connection Handling
- [ ] Set up axum router with a single `/ws` route that upgrades to WebSocket
- [ ] Implement `handle_connection(socket, state)` async function:
  - Assign a unique connection ID
  - Run a message receive loop
  - On each message: parse → dispatch to handler → broadcast result
  - On disconnect: run cleanup (see 1.6)
- [ ] Bind server to `0.0.0.0:8080`, log startup with `tracing::info!`

#### 1.5 Session Event Handlers
Implement one handler function per client→server event. Each handler receives `(event_payload, source_user_id, state)` and returns a result to broadcast.

- [ ] **`handle_join_session`**: Create session if not exists. Add user with assigned color (pick from a fixed palette by index). Send `FULL_STATE_SYNC` to the joining client only. Broadcast `USER_JOINED` to all others.
- [ ] **`handle_leave_session`**: Remove user from session. Broadcast `USER_LEFT` to remaining clients.
- [ ] **`handle_create_object`**: Validate required fields. Insert `SceneObject` into session state. Append to event log. Broadcast `OBJECT_CREATED` to all other clients.
- [ ] **`handle_delete_object`**: If UUID exists, remove it. If not, no-op. Broadcast `OBJECT_DELETED`. Append to event log.
- [ ] **`handle_update_transform`**: LWW check: only overwrite if incoming `timestamp > last_updated_at`. Update `transform`, `last_updated_by`, `last_updated_at`. Broadcast `TRANSFORM_UPDATED` to all other clients. Append to event log.
- [ ] **`handle_update_properties`**: Same LWW logic as transform. Update `properties` field. Broadcast `PROPERTIES_UPDATED`.
- [ ] **`handle_update_name`**: Overwrite name. Broadcast `NAME_UPDATED`.
- [ ] **`handle_select_object`**: Update `user.selected_object`. Broadcast `USER_SELECTED`.

#### 1.6 Disconnect Cleanup
- [ ] On WebSocket close or error: look up which session the user was in, call `handle_leave_session` cleanup, broadcast `USER_LEFT`
- [ ] Session state persists after all users disconnect (session is NOT deleted)

#### 1.7 Broadcast Infrastructure
- [ ] Implement `broadcast_to_session(session_id, message, exclude_user_id, state)`:
  - Iterate over all connections in the session except `exclude_user_id`
  - Send serialized message to each via their `tokio::sync::mpsc::Sender<String>` channel
- [ ] Store per-connection sender in a `DashMap<String, Sender<String>>` keyed by connection ID
- [ ] Map connection ID → (session_id, user_id) in a separate lookup map

#### 1.8 Structured Logging
- [ ] Log every received event: `event_type`, `session_id`, `source_user_id`, `object_id` (if applicable)
- [ ] Log every broadcast: `session_id`, `event_type`, `recipient_count`
- [ ] Log session create/destroy events
- [ ] Log connection open/close with connection ID and remote IP

#### 1.9 Basic Event Log
- [ ] Append every state-mutating event (CREATE, DELETE, UPDATE_*) as a `LogEntry { timestamp, event_type, payload }` to `session.event_log`
- [ ] No persistence yet — in-memory `Vec` only (file-backed comes in Phase 6)

#### 1.10 Phase 1 Acceptance Test
- [ ] Write an integration test using `tokio-tungstenite` that:
  1. Spins up the server in a test runtime
  2. Connects two clients
  3. Client A joins session `"test-01"`
  4. Client B joins session `"test-01"`, receives `FULL_STATE_SYNC`
  5. Client A sends `CREATE_OBJECT` (cube)
  6. Assert Client B receives `OBJECT_CREATED` with matching UUID and transform
  7. Client A sends `DELETE_OBJECT`
  8. Assert Client B receives `OBJECT_DELETED`
  9. Client A disconnects; assert Client B receives `USER_LEFT`

---

### Phase 2 — Blender Plugin Skeleton
**Weeks 2–3 | Goal: A Blender addon that connects to the server, joins a session, and receives a full state sync.**

#### 2.1 Addon File Structure
```
blender_plugin/
├── __init__.py          # bl_info, register(), unregister()
├── operators.py         # All bpy.types.Operator subclasses
├── panels.py            # All bpy.types.Panel subclasses
├── preferences.py       # AddonPreferences
├── websocket_client.py  # Background thread WebSocket logic
├── event_handlers.py    # Incoming server event → Blender action
├── state.py             # Plugin-local state (connected session, object map, user map)
└── utils.py             # UUID helpers, transform helpers
```

- [ ] Create all files with stub content
- [ ] Write `bl_info` dict in `__init__.py` with correct name, version, blender minimum version
- [ ] Implement `register()` and `unregister()` that register/unregister all classes

#### 2.2 Addon Preferences
- [ ] Define `MeerkatPreferences(bpy.types.AddonPreferences)`:
  - `server_url: StringProperty` — default `"wss://meerkat.<yourdomain>.com"` (update in Phase 7; use `"ws://localhost:8080"` during development)
  - `asset_library_path: StringProperty` — file path selector
- [ ] Expose preferences in Blender's addon preferences panel

#### 2.3 Plugin State
- [ ] Define a module-level `PluginState` dataclass in `state.py`:
  - `connected: bool`
  - `session_id: str`
  - `user_id: str` — generated once at plugin load (UUID4)
  - `display_name: str`
  - `object_map: dict[str, bpy.types.Object]` — meerkat_id → Blender object
  - `users: dict[str, dict]` — user_id → display_name, color
  - `is_applying_remote_update: bool` — echo suppression flag
- [ ] Initialize `PluginState` as a singleton on module load

#### 2.4 WebSocket Background Thread
- [ ] Implement `WebSocketClient` class in `websocket_client.py`:
  - Runs in a `threading.Thread` as a daemon
  - Contains an `asyncio` event loop
  - On `connect(url)`: open WebSocket connection
  - On `send(message_dict)`: serialize to JSON, send on the socket
  - On `receive loop`: deserialize JSON, put message onto a `queue.Queue` for main thread pickup
  - On disconnect or error: set `connected = False`, log the error
- [ ] Implement `disconnect()`: cleanly close the WebSocket

#### 2.5 Main Thread Dispatcher
- [ ] Register a `bpy.app.timers` function that fires every 50ms
- [ ] On each tick: drain the incoming message queue, call the appropriate handler from `event_handlers.py` for each message
- [ ] This is the ONLY place Blender objects are created/modified (required for thread safety)

#### 2.6 Connect / Disconnect Operators
- [ ] `MEERKAT_OT_connect`: reads server URL, room name, display name from the UI panel → starts `WebSocketClient` → sends `JOIN_SESSION` → sets `PluginState.connected = True`
- [ ] `MEERKAT_OT_disconnect`: sends `LEAVE_SESSION` → closes `WebSocketClient` → sets `PluginState.connected = False`
- [ ] Both operators show error reports on failure (wrong URL, server unreachable)

#### 2.7 UI Panel
- [ ] Register `MEERKAT_PT_main_panel` as a panel in the 3D Viewport N-panel, category `"Meerkat"`
- [ ] Panel layout (when disconnected):
  - Label: "Meerkat Collaboration"
  - Field: Server URL (reads from preferences)
  - Field: Room Name
  - Field: Display Name
  - Button: Connect
- [ ] Panel layout (when connected):
  - Label: `"● Connected: <room-name>"` (green indicator via `alert=True` or icon)
  - Button: Disconnect
  - Separator
  - Section: "Users" (populated in Phase 5)
  - Separator
  - Section: "Objects" — asset dropdown and creation operators (populated in Phase 3)

#### 2.8 FULL_STATE_SYNC Handler
- [ ] Implement `handle_full_state_sync(payload)` in `event_handlers.py`:
  - Set `is_applying_remote_update = True`
  - For each object in `payload["objects"]`: call the appropriate object creation function
  - For each user in `payload["users"]`: update `PluginState.users`
  - Set `is_applying_remote_update = False`
- [ ] On FULL_STATE_SYNC, first clear all existing Meerkat-managed objects from the scene (identified by `meerkat_id` custom property)

#### 2.9 Asset Library Configuration
- [ ] In addon preferences: file path selector for asset library `.blend` file(s)
- [ ] Implement `load_asset_library(path) -> list[str]`: uses `bpy.data.libraries` to peek at object names in the file without fully loading it (use `bpy.data.libraries.load(path)` context manager, read `lib.data_blocks` linked object names)
- [ ] Store loaded asset names in `PluginState.asset_library_objects: list[str]`

#### 2.10 Phase 2 Acceptance Test
- [ ] Start server locally (`cargo run`)
- [ ] Install plugin in Blender
- [ ] Connect to `ws://localhost:8080`, join session `"test"`
- [ ] Confirm server logs show `JOIN_SESSION` received
- [ ] Confirm plugin UI shows connected state
- [ ] Disconnect and reconnect — confirm state is restored via `FULL_STATE_SYNC` (empty scene at this point)

---

### Phase 3 — Object Lifecycle
**Weeks 3–4 | Goal: Creating, deleting, and syncing all object types (primitives, cameras, lights, asset refs) across clients.**

#### 3.1 UUID Tagging
- [ ] Every Meerkat-managed object gets a custom property `obj["meerkat_id"] = str(uuid4())` set at creation time
- [ ] Implement `get_meerkat_id(obj) -> str | None` helper
- [ ] Implement `find_object_by_meerkat_id(uuid) -> bpy.types.Object | None` that searches `bpy.data.objects`

#### 3.2 Build CREATE_OBJECT Payload
- [ ] Implement `build_transform(obj) -> dict`: extract `location`, `rotation_euler`, `scale` from a Blender object, return as `{"position": [...], "rotation": [...], "scale": [...]}`
- [ ] Implement `build_create_payload(obj, object_type, asset_id=None, library=None) -> dict`: assemble the full `CREATE_OBJECT` payload

#### 3.3 Primitive Creation Operators
- [ ] `MEERKAT_OT_add_cube`: calls `bpy.ops.mesh.primitive_cube_add()`, tags the result with a UUID, sends `CREATE_OBJECT` with `type: "cube"`
- [ ] `MEERKAT_OT_add_sphere`: same for UV sphere
- [ ] `MEERKAT_OT_add_cylinder`: same for cylinder
- [ ] All three operators appear as buttons in the Meerkat panel under "Add Object"

#### 3.4 Camera Creation Operator
- [ ] `MEERKAT_OT_add_camera`: calls `bpy.ops.object.camera_add()`, tags with UUID, sends `CREATE_OBJECT` with `type: "camera"` and initial properties (focal_length, sensor_width, clip_start, clip_end from the newly created camera data)

#### 3.5 Light Creation Operators
- [ ] `MEERKAT_OT_add_point_light`: calls `bpy.ops.object.light_add(type='POINT')`, tags with UUID, sends `CREATE_OBJECT` with `type: "point_light"` and color, energy, shadow_soft_size
- [ ] `MEERKAT_OT_add_sun_light`: same for `type='SUN'`, sends color, energy, angle

#### 3.6 OBJECT_CREATED Receive Handler
- [ ] Implement `handle_object_created(payload)` in `event_handlers.py`:
  - Set `is_applying_remote_update = True`
  - Dispatch to the correct creation function by `payload["type"]`
  - Assign `meerkat_id` custom property with `payload["object_id"]`
  - Apply transform from payload
  - Apply type-specific properties from payload
  - Register object in `PluginState.object_map`
  - Set `is_applying_remote_update = False`

#### 3.7 Object Deletion — Send
- [ ] Implement `detect_and_send_deletions()` called from the main timer tick:
  - Compare `PluginState.object_map` keys against objects currently in `bpy.data.objects`
  - Any UUID in the map with no matching Blender object → send `DELETE_OBJECT` and remove from map
- [ ] Guard with `if not is_applying_remote_update:` to prevent echo loops

#### 3.8 OBJECT_DELETED Receive Handler
- [ ] Implement `handle_object_deleted(payload)`:
  - Set `is_applying_remote_update = True`
  - Find the object by `meerkat_id`
  - If found: `bpy.data.objects.remove(obj, do_unlink=True)`
  - If not found: log warning and return (no-op, no error)
  - Remove from `PluginState.object_map`
  - Set `is_applying_remote_update = False`

#### 3.9 Asset Library UI & Placement
- [ ] Add an asset dropdown (`EnumProperty` populated from `PluginState.asset_library_objects`) to the Meerkat panel
- [ ] `MEERKAT_OT_place_asset` operator:
  1. Read selected asset name from the enum property
  2. Call `bpy.ops.wm.link(filepath=..., directory=..., filename=asset_name)` to link from the local library file (linked objects retain a live reference — when the source `.blend` is updated and the library is reloaded, the asset updates automatically without re-placing)
  3. Find the newly linked object (search for it by name in `bpy.data.objects`)
  4. Tag with UUID
  5. Send `CREATE_OBJECT` with `type: "asset_ref"`, `asset_id: asset_name`, `asset_library: library_filename`

#### 3.10 OBJECT_CREATED: asset_ref Handler
- [ ] In `handle_object_created`, for `type == "asset_ref"`:
  - Call `bpy.ops.wm.link(filepath=..., directory=..., filename=payload["asset_id"])`
  - Find linked object by name
  - Assign `meerkat_id`
  - Apply transform

#### 3.11 Missing Asset Placeholder
- [ ] If `bpy.ops.wm.link` fails or the asset_id is not found in the local library:
  - Create a wireframe cube mesh object as a placeholder
  - Set its name to `f"[MISSING] {asset_id}"`
  - Add a text annotation in the 3D viewport (or use object name as the label)
  - Log a warning to the Meerkat panel status area
  - Still assign the `meerkat_id` so the object is tracked

#### 3.12 Phase 3 Acceptance Test
- [ ] Open two Blender instances, both connected to `ws://localhost:8080`, same session
- [ ] Add a cube in Client A → confirm it appears in Client B within 250ms
- [ ] Add a camera in Client A → confirm it appears in Client B
- [ ] Add a point light in Client A → confirm it appears in Client B
- [ ] Place an asset from the shared library in Client A → confirm it appears in Client B
- [ ] Delete the cube in Client A → confirm it disappears in Client B
- [ ] Delete an object in Client B → confirm it disappears in Client A

---

### Phase 4 — Transform & Property Sync
**Weeks 4–5 | Goal: Smooth, throttled, echo-suppressed real-time sync of object positions and type-specific properties.**

#### 4.1 Transform Poller
- [ ] Implement `poll_transforms()` called from the 33ms timer tick (30Hz)
- [ ] Maintain `transform_cache: dict[str, dict]` — `meerkat_id → last_sent_transform`
- [ ] For each object in `PluginState.object_map`:
  - If `is_applying_remote_update`: skip
  - Read current transform via `build_transform(obj)`
  - Compare to `transform_cache[meerkat_id]` (element-wise, with a small epsilon for float comparison)
  - If changed AND throttle allows: send `UPDATE_TRANSFORM`, update cache

#### 4.2 Client-Side Throttle
- [ ] Maintain `last_transform_send_time: dict[str, float]` — `meerkat_id → timestamp`
- [ ] Before sending `UPDATE_TRANSFORM`: check `time.monotonic() - last_send_time[id] >= (1/30)`
- [ ] If too soon: skip this tick (the next tick will pick up the latest transform)

#### 4.3 TRANSFORM_UPDATED Receive Handler
- [ ] Implement `handle_transform_updated(payload)`:
  - Set `is_applying_remote_update = True`
  - Find object by `meerkat_id`
  - If not found: log warning and return
  - Set `obj.location`, `obj.rotation_euler`, `obj.scale` from payload
  - Update `transform_cache` so the poller doesn't immediately re-send
  - Set `is_applying_remote_update = False`

#### 4.4 Camera Property Poller
- [ ] Maintain `camera_property_cache: dict[str, dict]` — `meerkat_id → last_sent_properties`
- [ ] For each camera object in `object_map`:
  - Read `obj.data.lens` (focal_length), `obj.data.sensor_width`, `obj.data.clip_start`, `obj.data.clip_end`
  - If changed: send `UPDATE_PROPERTIES`, update cache
- [ ] Same 30Hz throttle as transforms

#### 4.5 Light Property Poller
- [ ] For each point light in `object_map`:
  - Read `obj.data.color[0:3]`, `obj.data.energy` (power), `obj.data.shadow_soft_size` (radius)
  - If changed: send `UPDATE_PROPERTIES`
- [ ] For each sun light:
  - Read `obj.data.color[0:3]`, `obj.data.energy` (intensity), `obj.data.angle`
  - If changed: send `UPDATE_PROPERTIES`

#### 4.6 PROPERTIES_UPDATED Receive Handler
- [ ] Implement `handle_properties_updated(payload)`:
  - Set `is_applying_remote_update = True`
  - Find object by `meerkat_id`
  - Dispatch to the correct property setter based on object type:
    - Camera: set `obj.data.lens`, `sensor_width`, `clip_start`, `clip_end`
    - Point Light: set `obj.data.color`, `obj.data.energy`, `obj.data.shadow_soft_size`
    - Sun Light: set `obj.data.color`, `obj.data.energy`, `obj.data.angle`
  - Update property cache
  - Set `is_applying_remote_update = False`

#### 4.7 Object Name Poller & Sync
- [ ] Maintain `name_cache: dict[str, str]` — `meerkat_id → last_sent_name`
- [ ] For each object in `object_map`: if `obj.name != name_cache[id]`: send `UPDATE_NAME`, update cache
- [ ] `handle_name_updated(payload)`: set `obj.name = payload["name"]`, update cache

#### 4.8 Server-Side Transform Coalescing (Optional but recommended)
- [ ] On the Rust server: maintain a per-session, per-object "pending broadcast" queue
- [ ] When multiple `UPDATE_TRANSFORM` events arrive for the same `object_id` from the same `source_user_id` within a single event loop tick: drop all but the latest before broadcasting
- [ ] This prevents burst flooding when a client sends several transforms in rapid succession

#### 4.9 Phase 4 Acceptance Test
- [ ] Client A: grab a cube and move it around continuously
- [ ] Client B: observe the cube moving in near-real-time, no jitter, no freezing
- [ ] Client A: change a point light's color → Client B sees color change within 250ms
- [ ] Client A: change a camera's focal length → Client B sees the update
- [ ] Client A: rename an object → Client B sees new name in outliner
- [ ] Verify server logs show ≤30 transform updates/second per object

---

### Phase 5 — Presence, Export & Full State
**Weeks 5–6 | Goal: User presence panel, visual selection highlights, scene export, and robust session persistence.**

#### 5.1 USER_JOINED / USER_LEFT Handlers
- [ ] `handle_user_joined(payload)`: add user to `PluginState.users`, trigger panel redraw (`bpy.ops.wm.redraw_timer(type='DRAW_WIN', iterations=1)` or tag regions)
- [ ] `handle_user_left(payload)`: remove user from `PluginState.users`, trigger redraw

#### 5.2 Users Sub-Panel
- [ ] Add a "Users" collapsible section to `MEERKAT_PT_main_panel`
- [ ] For each user in `PluginState.users`:
  - Draw a colored square icon using the user's assigned color (use a `UILayout.prop` with a color swatch, or draw a label with custom icon)
  - Display `display_name` next to it
- [ ] If the user is the local client: show `"(you)"` suffix

#### 5.3 Color Assignment
- [ ] Server assigns color from a fixed palette on `JOIN_SESSION`: e.g., 10 visually distinct colors in a hardcoded array, pick by modular index of current user count
- [ ] Color is included in `USER_JOINED` broadcast so all clients show the same color for each user

#### 5.4 Selection Broadcast — Send
- [ ] Register a `bpy.app.handlers.depsgraph_update_post` handler (or poll in the timer)
- [ ] Detect active object change: compare `bpy.context.view_layer.objects.active` to `PluginState.last_selected`
- [ ] If changed and not `is_applying_remote_update`: send `SELECT_OBJECT` with the `meerkat_id` of the newly selected object (or `null` if nothing is selected)
- [ ] Update `PluginState.last_selected`

#### 5.5 Selection Presence — Visual Highlight
- [ ] Implement a `bpy.types.SpaceView3D` draw handler registered with `bpy.types.SpaceView3D.draw_handler_add`
- [ ] In the draw callback:
  - For each user in `PluginState.users` who has a `selected_object`:
    - Find the Blender object by `meerkat_id`
    - Draw a colored bounding box outline using `gpu` and `gpu_extras.batch` (immediate mode drawing)
    - Draw the user's display name near the object using `blf`
- [ ] Clean up draw handler on disconnect and plugin unregister

#### 5.6 USER_SELECTED Receive Handler
- [ ] `handle_user_selected(payload)`: update `PluginState.users[user_id]["selected_object"]`; the draw handler picks this up on the next frame

#### 5.7 Full State Sync — Robust Handling
- [ ] On `FULL_STATE_SYNC` receive:
  1. Set `is_applying_remote_update = True`
  2. Find all objects in the scene with `"meerkat_id"` custom property → delete them
  3. Clear `PluginState.object_map`, `transform_cache`, `property_cache`, `name_cache`
  4. Recreate all objects from the snapshot
  5. Repopulate user list
  6. Set `is_applying_remote_update = False`
- [ ] This is safe to call at any time, including on reconnect

#### 5.8 Session Persistence & Reconnect
- [ ] Server already persists session state after disconnect (from Phase 1)
- [ ] Client: implement auto-reconnect in `WebSocketClient`:
  - On disconnect: wait 2 seconds, attempt reconnect up to 5 times with exponential backoff (2s, 4s, 8s, 16s, 30s)
  - On successful reconnect: re-send `JOIN_SESSION` → receive `FULL_STATE_SYNC` → restore local state
  - Show reconnecting status in the UI panel

#### 5.9 Save Scene Operator
- [ ] `MEERKAT_OT_save_scene`:
  1. Send a `FULL_STATE_SYNC` request to server (add a client→server `REQUEST_STATE_SYNC` event type, or just re-join the session)
  2. Wait for the sync to complete (use a flag `PluginState.sync_complete`)
  3. Call `bpy.ops.wm.save_as_mainfile('INVOKE_DEFAULT')` to open the file browser
- [ ] The saved `.blend` opens standalone — no Meerkat hooks are embedded in the file

#### 5.10 Phase 5 Acceptance Test
- [ ] Client A and B join the same session
- [ ] Client A disconnects → Client B sees Client A disappear from the users panel
- [ ] Client A reconnects → Client B sees Client A reappear; Client A gets full scene state back
- [ ] Client A selects a cube → Client B sees a colored bounding box around that cube labeled with Client A's name
- [ ] User clicks Save Scene → a `.blend` file is saved; open it in a fresh Blender without the addon → all objects are present

---

### Phase 6 — Observability & Durability
**Weeks 6–7 | Goal: File-backed event log with crash recovery, metrics endpoint, property-based tests, and latency benchmarks.**

#### 6.1 File-Backed Event Log
- [ ] On `JOIN_SESSION` (session create): open a file `./data/<session_id>.log` for append
- [ ] Serialize every state-mutating event as a newline-delimited JSON entry and `fsync` after each write
- [ ] Keep the `session.event_log` in-memory `Vec` as a hot copy; the file is the durable copy
- [ ] On server startup: scan `./data/` for `.log` files and replay each to reconstruct session state

#### 6.2 Crash Recovery Replay
- [ ] Implement `replay_event_log(log_entries: Vec<LogEntry>) -> Session`:
  - Process each entry in order using the same handler logic as live events
  - Skip validation that requires a live client (e.g., user presence)
  - Return the reconstructed `Session`
- [ ] Acceptance test: start server, create objects, kill the server process hard (`kill -9`), restart — all objects should reappear

#### 6.3 Log Compaction
- [ ] After every 1,000 events per session, trigger compaction:
  1. Serialize the current canonical state snapshot to `./data/<session_id>.snapshot.json`
  2. Truncate `./data/<session_id>.log` (keep it open for new appends)
  3. On replay startup: load snapshot first, then replay only events after the snapshot timestamp
- [ ] Log compaction event with `tracing::info!`

#### 6.4 Metrics HTTP Endpoint
- [ ] Add an axum route `GET /metrics` (on a separate port `8081` or same port, different path)
- [ ] Implement `MetricsState` with atomic counters:
  - `active_sessions: AtomicUsize`
  - `active_connections: AtomicUsize`
  - `messages_received_total: AtomicU64`
  - `transform_updates_total: AtomicU64`
- [ ] Track latency using a sliding window histogram (or a simple `VecDeque<u64>` of the last 1,000 timestamps)
- [ ] Compute p50, p95, p99 propagation latency by embedding `sent_at` in transform update payloads and computing `received_at - sent_at` on broadcast
- [ ] Return metrics as JSON: `{ active_sessions, active_connections, msg_per_sec, p50_ms, p95_ms, p99_ms }`

#### 6.5 Property-Based Tests
- [ ] Add `proptest = "1"` to `[dev-dependencies]`
- [ ] Write a strategy that generates random sequences of `(event_type, object_id, payload)` tuples
- [ ] Property 1: After replaying any sequence, `session.objects` equals the state from applying events live
- [ ] Property 2: LWW is monotonic — applying an older update after a newer one does not regress state
- [ ] Property 3: DELETE of nonexistent UUID is always a no-op (no panic, no state change)
- [ ] Run with `cargo test` as part of CI

#### 6.6 Latency Benchmark Harness
- [ ] Create a `benches/` crate or a standalone `benchmark` binary
- [ ] CLI args: `--clients N`, `--rate HZ`, `--duration SECONDS`
- [ ] Spawn N `tokio-tungstenite` clients, all join the same session
- [ ] Each client sends `UPDATE_TRANSFORM` at `HZ` for one object
- [ ] Embed `sent_at: unix_timestamp_ms` in the payload
- [ ] One designated "observer" client receives all broadcasts and computes `received_at - sent_at`
- [ ] After duration, print results:
  ```
  Clients: 5 | Rate: 30Hz | Duration: 10s
  Messages sent:    1500
  Messages received: 7350
  Latency p50: 4ms | p95: 12ms | p99: 28ms
  ```
- [ ] Include these numbers in `README.md`

#### 6.7 Phase 6 Acceptance Test
- [ ] Kill the server while 2 clients are connected with objects in the scene
- [ ] Restart the server
- [ ] Both clients reconnect (via Phase 5 auto-reconnect)
- [ ] Both receive `FULL_STATE_SYNC` with all objects restored from event log
- [ ] `/metrics` endpoint returns valid JSON with non-zero counts
- [ ] `cargo test` passes all property-based tests
- [ ] Benchmark outputs a latency table

---

### Phase 7 — Polish, Deployment & Demo
**Weeks 7–8 | Goal: Production-quality deployment on a public cloud server with TLS, all edge cases handled, and a demo-ready product.**

#### 7.1 Error Handling & Graceful Degradation
- [ ] All WebSocket send failures on the server: log and remove the dead connection, do not panic
- [ ] Malformed client messages: send `ERROR` event back to the offending client, continue serving others
- [ ] Plugin: all server errors surface as Blender `self.report({'ERROR'}, message)` in operators
- [ ] Plugin: if `FULL_STATE_SYNC` payload is malformed, log the error and request it again
- [ ] Server: if event log write fails, log the error but continue serving (degrade gracefully, don't crash)

#### 7.2 Transform Interpolation (Receiving Client)
- [ ] Instead of snapping objects to new transforms on receive, interpolate over the next 3–5 frames
- [ ] Maintain `pending_transforms: dict[str, (target_transform, steps_remaining)]` in plugin state
- [ ] On each timer tick: for each object with a pending transform, lerp current → target by `1/steps_remaining`, decrement counter
- [ ] This smooths jitter caused by network irregularity and makes the demo feel polished

#### 7.3 Docker
- [ ] Write `Dockerfile` for the Rust server using a multi-stage build:
  ```dockerfile
  FROM rust:1.76 AS builder
  WORKDIR /app
  COPY . .
  RUN cargo build --release

  FROM debian:bookworm-slim
  COPY --from=builder /app/target/release/meerkat-server /usr/local/bin/
  EXPOSE 8080 8081
  CMD ["meerkat-server"]
  ```
- [ ] Write `docker-compose.yml`:
  ```yaml
  services:
    meerkat:
      build: .
      ports:
        - "8080:8080"   # WebSocket
        - "8081:8081"   # Metrics
      volumes:
        - ./data:/app/data
  ```
- [ ] Test locally: `docker-compose up`, connect Blender plugin, confirm everything works

#### 7.4 Connection Limits & Rate Limiting
- [ ] Add `tower` and `tower-http` dependencies to the server
- [ ] Implement per-IP rate limiting middleware using `tower::ServiceBuilder` and a token bucket:
  - Max 10 new WebSocket upgrades per second per IP
- [ ] In `handle_join_session`: reject with `ERROR` if session already has 10 users
- [ ] In `handle_join_session`: reject with `ERROR` if total active sessions ≥ 20 (new session names only; existing sessions can still be rejoined)
- [ ] Per-connection message rate: track message count per connection per second; if > 100 msg/sec, send `ERROR` and close the connection

#### 7.5 Fly.io Deployment
- [ ] Install `flyctl` CLI
- [ ] Run `flyctl auth login`
- [ ] Run `flyctl launch` in the project root:
  - App name: `meerkat-server` (or your chosen name)
  - Region: closest to you (e.g., `iad` for US East)
  - No database needed
- [ ] Edit generated `fly.toml`:
  ```toml
  [http_service]
    internal_port = 8080
    force_https = true   # Fly.io handles TLS, forwards as HTTP internally

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.ports]]
    port = 80
    handlers = ["http"]
  ```
- [ ] Run `flyctl deploy` — Fly.io builds the Docker image, deploys, provisions TLS via Let's Encrypt
- [ ] Run `flyctl status` — confirm the app is running
- [ ] Test WSS: use `websocat wss://<your-app>.fly.dev` to confirm WebSocket upgrade works
- [ ] Confirm `/metrics` endpoint is accessible at `https://<your-app>.fly.dev/metrics`

#### 7.6 Update Plugin Default Server URL
- [ ] In `preferences.py`, change the default `server_url` from `"ws://localhost:8080"` to `"wss://<your-app>.fly.dev"`
- [ ] Confirm the plugin connects to the live server from Blender with no configuration needed beyond installing the addon

#### 7.7 Blender Addon Packaging
- [ ] Create a `package.sh` (or `Makefile` target) that zips the `blender_plugin/` directory:
  ```bash
  zip -r meerkat-blender-addon.zip blender_plugin/
  ```
- [ ] Test install: Blender → Edit → Preferences → Add-ons → Install → select the `.zip`
- [ ] Confirm the addon installs cleanly, shows up in the addon list, and connects to the live server

#### 7.8 Architecture Decision Records
- [ ] Create `/docs/decisions/` directory
- [ ] Write one `.md` file per ADR (ADR-001 through ADR-007) with format:
  ```markdown
  # ADR-00N: Title
  **Status:** Accepted
  **Date:** 2026-02

  ## Context
  ## Decision
  ## Consequences
  ```

#### 7.9 Demo Video
- [ ] Set up two Blender instances side-by-side on screen, both connected to the live `wss://` server
- [ ] Record a screen capture showing:
  1. Both clients join the session
  2. User A places an asset from the shared library → appears in User B instantly
  3. User A moves the object → User B sees smooth real-time movement
  4. User B adds a camera → User A sees it appear
  5. User B changes the camera's focal length → User A sees the update
  6. User A adds a point light, changes its color → User B sees the color change
  7. User A selects an object → User B sees the colored highlight
  8. User B saves the scene as a `.blend` file
- [ ] Export a 15–30 second hero GIF from the recording
- [ ] Export the full walkthrough video

#### 7.10 README
- [ ] Hero GIF at the top of `README.md`
- [ ] One-paragraph project description (lead with "real-time distributed state sync engine in Rust")
- [ ] Architecture diagram (ASCII or image) showing Client → WSS → Rust Server → broadcast
- [ ] Feature table (what's synced, what's not)
- [ ] Quick start:
  ```bash
  # Option 1: Use the public hosted server
  # Install blender_addon.zip in Blender, enter room name, connect.

  # Option 2: Self-hosted
  docker-compose up
  # Then set server URL to ws://localhost:8080 in addon preferences
  ```
- [ ] Latency benchmark results table (from Phase 6)
- [ ] Link to ADRs in `/docs/decisions/`
- [ ] Link to the demo video

#### 7.11 Phase 7 Acceptance Test — End-to-End Against Live Server
- [ ] Two people on separate machines (or two Blender instances on the same machine) connect to `wss://<your-app>.fly.dev`, room `"demo"`
- [ ] All Phase 3, 4, and 5 acceptance tests pass against the live hosted server (not localhost)
- [ ] `/metrics` returns non-zero data
- [ ] Connection limits work: attempt to join with 11 simultaneous clients → 11th is rejected gracefully
- [ ] Server survives 5 minutes of continuous transform spam from 5 clients with no crashes

---

## 17. Risks and Mitigations

### 17.1 Technical Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Blender API has no clean transform change callback | High | Poll via timer (33ms), diff against cached state, emit on change |
| Infinite echo loops | High | `is_applying_remote_update` flag + `updated_by` field check |
| Transform spam overwhelming server | Medium | Client-side 30Hz cap + server-side per-object coalescing |
| State divergence under concurrent CREATE/DELETE | Medium | Server authoritative. DELETE of nonexistent = no-op. FULL_STATE_SYNC reset. |
| Blender Python GIL threading limitations | Medium | WebSocket on background thread, `bpy.app.timers` for UI dispatch |
| Asset library mismatch between clients | Medium | Validate `asset_id` on receive. Placeholder bounding box + warning if missing. |
| Event log grows unbounded | Low | Periodic compaction: snapshot state, truncate log |

### 17.2 Product Risks

| Risk | Mitigation |
|---|---|
| "Just a toy" perception | Asset library system enables real models in demo. Cameras and lights make it visually compelling. |
| Demo doesn't feel real-time | Transform smoothing/interpolation. Test with artificial latency. |
| Server requires manual setup | Docker Compose one-command. Blender add-on as `.zip` drag-and-drop. Public hosted server as default. |
| Asset library distribution friction | Document workflow clearly. All collaborators share the same `.blend` library file. |

### 17.3 Resume-Signaling Risks

| Risk | Mitigation |
|---|---|
| "Blender plugin" reads as hobbyist | Lead with: "real-time distributed state sync engine in Rust" |
| Rust without depth looks like hype-chasing | Prepare to discuss ownership, async, tokio, axum vs warp, serde |
| Solo project lacks collaboration signal | ADRs, clean commits, open issues, contributing guide |

---

## 18. Success Criteria

MVP is complete when:
- [ ] 2+ users can join the same session
- [ ] Users can place assets from a shared library and all participants see them
- [ ] Users can place cameras and lights and sync their properties
- [ ] Users can place primitive objects (cubes, spheres, cylinders)
- [ ] Transform sync is smooth and near-real-time for all object types
- [ ] Object naming syncs correctly
- [ ] User presence panel shows connected users
- [ ] Any user can save the scene as a `.blend` file
- [ ] Sessions persist across disconnection/reconnection
- [ ] Server recovers state from event log after crash
- [ ] No scene duplication, no orphan objects, no crashes
- [ ] Latency benchmark produces measurable p50/p95/p99 numbers
- [ ] Property-based tests pass for randomized operation sequences
- [ ] Server is deployed to Fly.io with TLS; plugin default URL is the live WSS endpoint
- [ ] Demo is reproducible via public hosted server OR `docker-compose up` + addon install in under 3 minutes
- [ ] README contains hero demo GIF, architecture diagram, latency table, and quick start

---

## 19. Future Extensions (Post-MVP)

- `.blend` file import to bootstrap a session from an existing scene
- Viewport camera sync (see through another user's eyes)
- Object locking (prevent simultaneous edits to same object)
- Text chat / spatial annotations pinned to objects
- Built-in asset library sharing (upload/download within Meerkat)
- Hot-reload asset library without restarting session
- Mesh-level delta sync (topology-aware diffing)
- CRDT-based conflict resolution
- Cloud persistence (PostgreSQL or SQLite)
- Session history and undo/redo sync
- Binary protocol (MessagePack or Protobuf)
- Horizontal server scaling
- Authentication and permissions
- Scene diff visualization

---

*The origin story is real. The problem is real. The architecture is sound. Now ship it.*
