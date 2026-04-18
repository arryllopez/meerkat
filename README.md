# Meerkat

[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Status](https://img.shields.io/badge/status-pre--alpha-orange)](https://github.com/arryllopez/meerkat)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/arryllopez/meerkat/pulls)
[![GitHub Stars](https://img.shields.io/github/stars/arryllopez/meerkat?style=social)](https://github.com/arryllopez/meerkat)
[![Discussions](https://img.shields.io/badge/GitHub-Discussions-purple?logo=github)](https://github.com/arryllopez/meerkat/discussions)

Real-time collaborative scene editing inside Blender — multiplayer object sync, live transforms, shared presence, and (soon) turn-based collaborative mesh modeling.

<p align="center">
  <img src="cursor_tracking-ezgif.com-video-to-gif-converter.gif" alt="Meerkat demo — real-time object sync between two Blender instances">
</p>

<p align="center">
  <img width="1200" height="300" alt="Meerkat banner" src="https://github.com/user-attachments/assets/1d96cfae-ddbe-475f-aaa3-78047050e43e" />
</p>

<h3 align="center">Alpha in progress — <a href="https://github.com/arryllopez/meerkat/discussions">follow along or join the discussion</a></h3>

---

## Why Meerkat?

Blender has no built-in real-time collaboration. Teams juggle `.blend` file versions over chat or cloud sync — hoping nobody overwrites each other's work. Meerkat makes the session live.

---

## What works today

- Real-time object lifecycle: create, delete, rename synced across peers.
- Live transforms: position, rotation, scale throttled at 30Hz.
- Camera and light property sync.
- Presence: connected users panel, selection highlights, remote cursor overlays.
- Password-gated sessions.
- Full state sync + reconnect + save-scene workflow.

---

## Alpha goal (v0.1) — Figma for Blender scene layouts

Real-time collaborative **scene layout** in Blender. Place, arrange, parent, and modify shared primitives, images, and drawings together. Think Figma, but for a 3D scene: you and your teammates block out a set, arrange references, tweak lighting, sketch in grease pencil, all live. Mesh-level editing is not part of alpha — that's the v0.2 headline. External asset library import (`.blend` linking) is also post-alpha, tracked in [issue #15](https://github.com/arryllopez/meerkat/issues/15).

**Who it's for:** layout teams, set dressers, level designers, architectural blockouts, storyboarders, anyone whose workflow is object-level.

### Alpha roadmap

**Phase 1 — Object type coverage** (biggest remaining chunk)

All Blender object data types relevant to scene layout, not just mesh primitives.

| Type | Alpha | Notes |
|------|:-----:|-------|
| Mesh | ✅ | Already synced; extend to full primitive creation params (segments, subdivisions, radii) |
| Camera | ✅ | Already synced |
| Light | ✅ | Already synced |
| Light Probe | ☐ | Reflection / irradiance probes |
| Object (empty) | ☐ | Plain empties (plain axes, arrows, image) |
| Collection | ☐ | Full hierarchy sync (create, nest, move, rename) |
| Curve | ☐ | Bezier + NURBS curves (NURBS *surfaces* out of alpha scope) |
| Text | ☐ | 3D text objects with font + geometry params |
| Metaball | ☐ | Implicit surfaces |
| Grease Pencil (v2 + v3) | ☐ | 2D/3D sketching + annotation |
| Image (reference) | ☐ | Empty-image objects and background references |
| Volume | ☐ | VDB volume objects |
| Speaker | ☐ | Audio objects |
| Material | ☐ | Material assignments + basic properties (shader node graph sync is post-alpha) |
| Texture | ☐ | Texture slots and references |
| Node Groups | ☐ | Geometry nodes + material node groups (compositing nodes post-alpha) |
| Geometry Nodes | ☐ | Node graph state sync |
| Scene settings | ☐ | Render, frame range, units |
| World | ☐ | HDRI, background color, environment |

**Phase 2 — Scene structure**
- [ ] Parenting (object-to-object, object-to-collection).
- [ ] Modifier stack sync (subsurf, mirror, bevel, array, solidify, boolean).

**Partial / planned (may slip to v0.2)**
- Armature (rigging)
- Particles (cache sync)
- Physics
- VSE (video sequencer)

**Out of alpha scope**
- External asset library import / `.blend` linking — semi-implemented, unstable. Tracked in [#15](https://github.com/arryllopez/meerkat/issues/15).
- Concurrent mesh-level editing — v0.2 headline (see post-alpha).
- Actions, NLA strips (animation — post-alpha).
- Compositing nodes.
- NURBS surfaces.

**Phase 3 — Polish**
- [ ] Reconnect UX (no stuck states, clear error surfacing).
- [ ] Rate limits and payload size guards.
- [ ] Session membership edge cases (purposeful disconnect vs retry logic, idle sessions).

**Phase 4 — Deployment**
- [ ] Docker + hosted relay.
- [ ] Launch demo video: 4-person scene layout session.

## Post-alpha roadmap

### v0.2 — Concurrent mesh editing

The headline feature after alpha. A team of 4 can collaboratively model a dragon, car, or chair — in real time, on the same mesh, simultaneously. Selection-granular locks with last-write-wins semantics let peers work on disjoint regions in parallel: extrude, loop cut, bevel, move verts, all concurrently. You see your teammate's geometry grow live next to yours.

Design borrows from two places: **Rust's ownership model** (exclusive mutable borrows, shared reads, RAII drop) and **Google Docs collaboration** (last-write-wins selection, per-user undo stack with cascade-delete on dependents).

**Ownership model**
- Stable per-vertex / per-edge / per-face IDs that survive topology ops.
- Ownership table: `{ element_id → user_id }`, server-arbitrated.
- Selection = ownership. Last-write-wins on overlap, per-element (not per-selection).
- Shared-read borrows: peers can reference owned elements for snapping and bridge targets without claiming (`&T` to the owner's `&mut T`).

**Active-operator guard**
- While a Blender operator is running (G, R, S, E, loop cut, bevel), the owner's lock is protected.
- Preemption attempts queued server-side, applied on op exit.
- Prevents mid-drag ownership theft and the desync that would follow.

**Edit flow**
- Vertex transforms streamed at 30Hz on owned elements (live sculpt feel).
- Topology ops broadcast with stable IDs, not full snapshots.
- Client-side ownership pre-check fails unowned-touching ops locally.
- Optional "full-mesh lock" escalation for global ops (loop cut across body, whole-mesh proportional edit). Modeled on Rust's `unsafe {}` — opt-in, rare.

**Undo (Google Docs model)**
- Per-user undo stack (hook Blender's built-in).
- Undo emits an inverse op broadcast like any edit.
- Cascade-delete on topology: removing an element drops dependents (Blender's BMesh enforces this natively). Matches Google Docs behavior — you undo your sentence, the friend's italic on it goes with it.
- Peers' dangling ops (referencing elements an undo deleted) drop silently on the peer's client.

**Peer rendering**
- Owned elements tinted with owner's user color.
- Hover tooltip: "Bob — 12s ago".
- Preemption click transfers color and control atomically.

**Resilience**
- Disconnect releases all locks (bookkeeping only; LWW makes grace periods unnecessary).
- Full mesh snapshot fallback on reconnect or desync detection.

### Later

- CRDT concurrent editing of the *same* element (no locks, automatic convergence on overlapping edits). Research-grade territory.
- Full operational-transform undo (transforms inverse ops against intervening peer ops, preserves peer contributions on overlapping regions).
- Material and shader-node sync.
- Animation sync (keyframes, markers, playback).
- Sculpt-brush stroke streaming for high-poly workflows.
- In-scene comments, snapshot timeline, in-plugin chat.

---

## Architecture

<img width="1507" height="674" alt="Meerkat Architecture Diagram" src="https://github.com/user-attachments/assets/7e35ad55-39a7-4034-b3b6-aa603eee2b75" />

Two components:

- **Rust backend** (`tokio` + `axum`) — WebSocket sessions, object ID/transform diffing, relay. Transmits only IDs and transforms, not mesh data, so bandwidth stays minimal.
- **Python Blender plugin** — Hooks Blender's depsgraph update handlers to capture local changes and apply incoming remote deltas.

---

## Requirements

| Dependency | Purpose |
|------------|---------|
| Blender 4.0+ | Plugin host |
| Python 3.10+ | Bundled with Blender |
| Rust 1.75+ | Backend server (if self-hosting) |

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
cargo clippy        # Lint
```

**Plugin development:**
```bash
# Symlink plugin into Blender's addons directory for live reloading
ln -s $(pwd)/blender_plugin ~/.config/blender/4.x/scripts/addons/meerkat
```

---

## Contributing

Contributions welcome — especially networking, Blender Python API, and conflict resolution strategies.

1. Fork the repository
2. Create your feature branch (`git checkout -b feat/your-feature`)
3. Commit your changes
4. Open a Pull Request

Have a question or idea? [Start a discussion](https://github.com/arryllopez/meerkat/discussions).

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=arryllopez/meerkat&type=Date)](https://star-history.com/#arryllopez/meerkat&Date)

---

## License

Licensed under the **GNU General Public License v3.0**.

- Use, modify, and distribute freely.
- Derivative work must also be open-source under GPLv3.
- No proprietary forks.

See the [LICENSE](LICENSE) file for full details.
