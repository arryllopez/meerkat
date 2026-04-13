# Meerkat

[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Status](https://img.shields.io/badge/status-alpha%20soon-orange)](https://github.com/arryllopez/meerkat)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/arryllopez/meerkat/pulls)
[![GitHub Stars](https://img.shields.io/github/stars/arryllopez/meerkat?style=social)](https://github.com/arryllopez/meerkat)
[![Discussions](https://img.shields.io/badge/GitHub-Discussions-purple?logo=github)](https://github.com/arryllopez/meerkat/discussions)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10+-blue?logo=python&logoColor=white)](https://www.python.org/)
[![Blender](https://img.shields.io/badge/Blender-4.0+-orange?logo=blender&logoColor=white)](https://www.blender.org/)
[![Tokio](https://img.shields.io/badge/Tokio-async-brightgreen?logo=rust)](https://tokio.rs/)
[![Axum](https://img.shields.io/badge/Axum-web-lightgrey?logo=rust)](https://github.com/tokio-rs/axum)
[![WebSocket](https://img.shields.io/badge/WebSocket-realtime-blue)](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](https://github.com/arryllopez/meerkat)
[![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](https://github.com/arryllopez/meerkat)
[![Windows](https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white)](https://github.com/arryllopez/meerkat)

Real-time collaborative editing inside Blender — multiplayer scene editing, live transforms, and shared sessions.

<p align="center">
  <img src="cursor_tracking-ezgif.com-video-to-gif-converter.gif" alt="Meerkat Demo — real-time object sync between two Blender instances">
</p>

<p align="center">
  <img width="1200" height="300" alt="Lawrence Arryl Lopez (1)" src="https://github.com/user-attachments/assets/1d96cfae-ddbe-475f-aaa3-78047050e43e" />

</p>

<h3 align="center">Alpha dropping soon — <a href="https://github.com/arryllopez/meerkat/discussions">join the discussion</a></h3>

---

## Why Meerkat?

Blender has no built-in real-time collaboration. If you're working with a team, you're juggling `.blend` file versions over chat or cloud sync — hoping nobody overwrites each other's work.

Meerkat fixes that.

| Feature | Meerkat | Manual File Sync | Proprietary Alternatives |
|---------|:-------:|:----------------:|:------------------------:|
| Real-time transform sync | ✅ | ❌ | Partial |
| Conflict resolution | ✅ | ❌ | Partial |
| Presence indicators | ✅ | ❌ | Some |
| Open-source | ✅ | ✅ | ❌ |
| Works inside Blender | ✅ | ❌ | ❌ |
| Cloud relay (optional) | ✅ | ❌ | ✅ |
| Peer-to-peer option | ✅ | ❌ | ❌ |

---

## Features

- **Multiplayer Scene Editing** — Multiple artists editing the same scene simultaneously
- **Live Transforms** — Object position, rotation, and scale synced in real-time
- **Shared Sessions** — Host or join a session directly from the Blender UI panel
- **Conflict Resolution** — Handles simultaneous edits gracefully without overwriting work
- **Presence Indicators** — See who's in the session and what they're selecting
- **Peer-to-Peer Option** — Direct connections without a relay server when on the same network
- **Cloud Relay** — Optional hosted relay for remote teams (no port forwarding required)

---


## Roadmap

- [x] **Phase 1 (Weeks 1-2): Rust server foundation** - WebSocket server, session state, event handlers, and broadcast infrastructure.
- [x] **Phase 2 (Weeks 2-3): Blender plugin skeleton** - addon structure, connect/disconnect flow, and initial full state sync.
- [x] **Phase 3 (Weeks 3-4): Object lifecycle sync** - create/delete/sync for primitives, cameras, lights, and asset references.
- [x] **Phase 4 (Weeks 4-5): Transform and property sync** - 30Hz throttled transforms, property updates, and name syncing.
- [x] **Phase 5 (Weeks 5-6): Presence and resilience** - users panel, selection highlights, robust full sync, reconnect handling, and scene export.
- [ ] **Phase 6 (Weeks 6-7): Observability and durability** - file-backed event log, crash recovery, metrics endpoint, and benchmarking.
- [ ] **Phase 7 (Weeks 7-8): Polish and deployment** - Docker, cloud deployment (WSS), rate limits, packaging, ADRs, and launch demo.

Detailed implementation checklist: `CLAUDE.MD` (see **Implementation Phases**).

---

## Reliability and Scale Notes (Cloud Hosting)

### Glaring backend issues to fix

- [x] **Silent message drops under load**: `mpsc::channel(32)` plus `try_send` currently drops messages when queues fill (`backend/meerkat-server/src/websocket.rs`, `backend/meerkat-server/src/handlers/helpers.rs`). (this is addressed by just dropping the connections that backlog)
- [x] **Broadcast cost scales with total connections**: fanout iterates all `connection_meta`, not just peers in the session (`backend/meerkat-server/src/handlers/helpers.rs`).
- [x] **Re-join can leave stale membership**: a connection can overwrite `connection_meta` without full prior cleanup (`backend/meerkat-server/src/handlers/join_session.rs`).
- [x] **Object ID clobber risk**: `CreateObject` inserts directly; duplicate IDs can overwrite state (`backend/meerkat-server/src/handlers/create_object.rs`).
- [x] **Runtime `expect` in hot paths**: serialization `expect(...)` can crash server process during malformed data conditions (multiple handler files + `websocket.rs`).
- [x] **Sessions are never reclaimed**: empty sessions are retained forever, growing memory over time (`backend/meerkat-server/src/handlers/leave_session.rs`).

### Blender plugin behavior to fix

- [x] **Data-loss risk on connect/sync**: plugin currently removes all scene objects during connect/full-state sync instead of only Meerkat-managed objects (`blender_plugin/operators.py`, `blender_plugin/event_handlers.py`).
- [x] **User identity mismatch edge case**: local `user_id` inferred by matching `display_name`; duplicate names can break echo suppression (`blender_plugin/event_handlers.py`).
- [x] **Reconnect errors are swallowed**: broad `except Exception: pass` hides failures and complicates debugging (`blender_plugin/websocket_client.py`).
- [ ] **Blender version gate too strict**: addon currently declares Blender `5.0.0` minimum (`blender_plugin/__init__.py`).
- [ ] **High-volume debug printing**: per-event payload printing adds overhead in active sessions (`blender_plugin/event_handlers.py`).

### What Meerkat already covers in Blender

- [x] Real-time multiplayer object lifecycle (create/delete/update) inside Blender.
- [x] Live transform, object naming, and selected camera/light property synchronization.
- [x] Presence features: connected users, selected-object highlights, and remote cursor overlays.
- [x] Shared asset library placement with hierarchy support and missing-asset placeholders.
- [x] Full state sync + reconnect foundation + save-scene workflow.

### High-impact additions for the ecosystem

- [ ] Collection, parenting, and outliner hierarchy sync.
- [ ] Object locking / edit intent indicators to prevent collisions.
- [ ] Material and shader-node parameter synchronization.
- [ ] Animation sync (keyframes, markers, playback state).
- [ ] In-scene review tools (comments, pins, annotations).
- [ ] Snapshot/version timeline (checkpoint, rollback, compare).
- [ ] In plugin chat feature 
- [ ] Maybe gRPC communication in the backend 

### Cost and concurrency strategy (self-hosted cloud)

- [x] Use per-session connection indexes (`session_id -> connection_ids`) so broadcast work is O(session size).
- [ ] Add protocol-level backpressure/coalescing (especially transform/cursor updates) and guaranteed handling for critical events.
- [ ] Enforce guardrails: message rate limits, payload size limits, max users/session, max active sessions, idle TTL.
- [ ] Replace panic paths with recoverable errors and server-side error events.
- [ ] Add production metrics: active sessions/connections, queue depth, drop counts, latency percentiles, msg/sec.



### First improvement set (next branch scope)

- [ ] Backend reliability hardening:
  1) Backpressure policy with coalescing for high-rate updates.
  2) No-drop handling for critical events (create/delete/join/leave).
  3) Remove panic-prone `expect` paths from runtime message flow.

---

## Architecture

<img width="1507" height="674" alt="Meerkat Architecture Diagram" src="https://github.com/user-attachments/assets/7e35ad55-39a7-4034-b3b6-aa603eee2b75" />

Meerkat is split into two components:

- **Rust backend** (`tokio` + `axum`) — Handles WebSocket sessions, object ID/transform diffing, and relay logic. Only transmits object IDs and transforms rather than full mesh data, keeping bandwidth minimal.
- **Python Blender plugin** — Hooks into Blender's depsgraph update handlers to capture and broadcast local changes, and applies incoming remote deltas to the scene.

---

## Requirements

### Runtime

| Dependency | Purpose |
|------------|---------|
| Blender 4.0+ | Plugin host |
| Python 3.10+ | Bundled with Blender |
| Rust 1.75+ | Backend server (if self-hosting) |

### Build (source installs only)

- Rust 1.75+
- Python 3.10+
- Blender 4.0+ (for plugin testing)

---

## Installation

> **Alpha not yet released.** Instructions will be finalized for the first release. Watch the repo or [join the discussion](https://github.com/arryllopez/meerkat/discussions) to be notified.

**From source (backend):**
```bash
git clone https://github.com/arryllopez/meerkat.git
cd meerkat
cargo build --release
```

**Plugin (Blender):**
```
# Coming soon — will be installable via Blender's Add-on preferences
Edit → Preferences → Add-ons → Install → select meerkat.zip
```

---

## Usage

```bash
# Start the relay server (self-hosted)
./meerkat-server

# Or connect to the hosted relay — configured directly in the Blender panel
```

Inside Blender, open the **Meerkat** side panel (`N` key → Meerkat tab):

| Action | Description |
|--------|-------------|
| Host Session | Start a new collaborative session |
| Join Session | Connect to an existing session by ID |
| Leave Session | Disconnect from the current session |
| View Peers | See who's currently connected |

---

## Development

```bash
cargo build         # Build backend binary
cargo test          # Run unit/integration tests
cargo clippy        # Lint (enforced via pre-commit)
```

**Plugin development:**
```bash
# Symlink plugin into Blender's addons directory for live reloading
ln -s $(pwd)/plugin ~/.config/blender/4.x/scripts/addons/meerkat
```

---

## Contributing

Contributions are welcome — especially around networking, Blender Python API expertise, and conflict resolution strategies.

1. Fork the repository
2. Create your feature branch (`git checkout -b feat/your-feature`)
3. Commit your changes
4. Open a Pull Request

Have a question or idea? [Start a discussion](https://github.com/arryllopez/meerkat/discussions).

---

## Featured In

> Know a place Meerkat should be listed? [Open an issue](https://github.com/arryllopez/meerkat/issues) or submit a PR.

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=arryllopez/meerkat&type=Date)](https://star-history.com/#arryllopez/meerkat&Date)

---

## License

Licensed under the **GNU General Public License v3.0**.

- You can use, modify, and distribute this software freely.
- Any derivative work must also be open-source under GPLv3.
- No proprietary forks.

See the [LICENSE](LICENSE) file for full details.
