# Meerkat

[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Status](https://img.shields.io/badge/status-pre--alpha-orange)](https://github.com/arryllopez/meerkat)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/arryllopez/meerkat/pulls)
[![GitHub Stars](https://img.shields.io/github/stars/arryllopez/meerkat?style=social)](https://github.com/arryllopez/meerkat)
[![Discussions](https://img.shields.io/badge/GitHub-Discussions-purple?logo=github)](https://github.com/arryllopez/meerkat/discussions)

Real-time collaborative scene editing inside Blender — multiplayer object sync, live transforms, shared presence, and (later) turn-based collaborative mesh modeling.

<p align="center">
  <img src="cursor_tracking-ezgif.com-video-to-gif-converter.gif" alt="Meerkat demo — real-time object sync between two Blender instances">
</p>

<p align="center">
  <img width="1200" height="300" alt="Meerkat banner" src="https://github.com/user-attachments/assets/1d96cfae-ddbe-475f-aaa3-78047050e43e" />
</p>

<h3 align="center">Alpha in progress — <a href="https://github.com/arryllopez/meerkat/discussions">follow along or join the discussion</a></h3>

---

## Why Meerkat?

Blender has no built-in real-time collaboration. Teams juggle `.blend` file versions over chat or cloud sync, hoping nobody overwrites each other's work. Meerkat makes the session live, like Figma did for design.

---

## What works today

- Real-time object lifecycle: create, delete, rename synced across peers
- Live transforms: position, rotation, scale throttled at 30Hz
- Camera and light property sync
- Presence: connected users panel, selection highlights, remote cursor overlays
- Password-gated sessions
- Full state sync on join + reconnect + save-scene workflow

---

## Alpha (v0.1) — Collaborative scene layout MVP

Real-time collaborative scene layout in Blender. Multiple users arrange primitives, cameras, and lights in a shared 3D scene, organized with collections. Think Figma, but for a 3D scene blockout — you and your teammates place furniture, set up lighting, position cameras, all live.

**Demo scenario:** A team of 4 collaboratively builds a classroom blockout — desks from cylinders and cubes, ceiling lights, cameras — in under 5 minutes, with every change visible across all clients in real time.

**Who it's for:** layout teams, set dressers, level designers, architectural blockouts, storyboarders — anyone whose workflow is object-level.

### Alpha roadmap

**Remaining for v0.1:**
- [ ] Full mesh primitive coverage — Plane, Cube, Circle, UV Sphere, Icosphere, Cylinder, Cone, Torus, Grid, Monkey
- [ ] Collection sync (create, nest, move, rename — full hierarchy)
- [ ] Docker + hosted relay server
- [ ] Launch demo video: 4-person classroom blockout session
- [ ] Reconnect UX polish (no stuck states, clear error surfacing)

That's it. Alpha ships when those five boxes are checked.

---

## Post-alpha roadmap

### v0.2 — Expanded object coverage

Widening alpha's object type surface so more scene-layout workflows are viable end-to-end.

- Empty objects (plain axes, arrows, image references)
- Curve objects (Bezier — for paths and guide lines)
- Text objects (for labels and signage)
- Image reference objects (floor plans, concept art)
- Parenting (object-to-object, object-to-collection)
- Modifier stack sync, starting with Mirror and Array (most common in blockout work)

### v0.3 — Richer object types and modifiers

- Grease Pencil (v2 + v3)
- Geometry Nodes (node graph state sync)
- Material assignments + basic properties (shader node graph sync later)
- Remaining modifiers: Subsurf, Bevel, Solidify, Boolean
- Light probes, metaballs, volumes, speakers
- World and scene settings

### v0.4 — Concurrent mesh editing

The real technical headline. A team of 4 can collaboratively model a dragon, car, or chair in real time on the same mesh, simultaneously. Selection-granular locks with last-write-wins semantics let peers work on disjoint regions in parallel: extrude, loop cut, bevel, move verts, all concurrently.

Design borrows from two places: **Rust's ownership model** (exclusive mutable borrows, shared reads, RAII drop) and **Google Docs collaboration** (last-write-wins selection, per-user undo stack with cascade-delete on dependents).

**Ownership model**
- Stable per-vertex / per-edge / per-face IDs that survive topology ops
- Ownership table: `{ element_id → user_id }`, server-arbitrated
- Selection = ownership. Last-write-wins on overlap, per-element
- Shared-read borrows: peers reference owned elements for snapping and bridge targets without claiming

**Active-operator guard**
- While a Blender operator is running (G, R, S, E, loop cut, bevel), the owner's lock is protected
- Preemption attempts queued server-side, applied on op exit
- Prevents mid-drag ownership theft and the desync that would follow

**Edit flow**
- Vertex transforms streamed at 30Hz on owned elements (live sculpt feel)
- Topology ops broadcast with stable IDs, not full snapshots
- Client-side ownership pre-check fails unowned-touching ops locally
- Optional "full-mesh lock" escalation for global ops. Modeled on Rust's `unsafe {}` — opt-in, rare

**Undo (Google Docs model)**
- Per-user undo stack (hook Blender's built-in)
- Undo emits an inverse op broadcast like any edit
- Cascade-delete on topology: removing an element drops dependents (BMesh enforces this natively)
- Peers' dangling ops drop silently on their client

**Peer rendering**
- Owned elements tinted with owner's user color
- Hover tooltip: "Bob — 12s ago"
- Preemption click transfers color and control atomically

**Resilience**
- Disconnect releases all locks (bookkeeping only; LWW makes grace periods unnecessary)
- Full mesh snapshot fallback on reconnect or desync detection

### Later

- CRDT concurrent editing of the same element (no locks, automatic convergence)
- Full operational-transform undo (inverse ops against intervening peer ops)
- Material and shader-node sync
- Animation sync (keyframes, markers, playback)
- Sculpt-brush stroke streaming for high-poly workflows
- In-scene comments, snapshot timeline, in-plugin chat
- External asset library import / `.blend` linking

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

- Use, modify, and distribute freely
- Derivative work must also be open-source under GPLv3
- No proprietary forks

See the [LICENSE](LICENSE) file for full details.
