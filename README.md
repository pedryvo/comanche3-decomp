<div align="center">

# comanche3-decomp

**Reverse engineering and a modern Rust reimplementation of _Comanche 3_**

[![Rust](https://img.shields.io/badge/Rust-2024%20edition-b7410e?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-playable%20MVP-blue)](#-progress)
[![Platform](https://img.shields.io/badge/tested%20on-Linux-lightgrey)](#-building-and-running)
[![License](https://img.shields.io/badge/license-GPL--3.0--only-blue)](LICENSE)

_Fly over terrain from the original DOS game in a resizable window with 1080p and 4K render modes._

</div>

---

## 🚁 About

**comanche3-decomp** studies the DOS edition of NovaLogic’s _Comanche 3_ and builds a Rust engine around its original data formats. The playable MVP reads the color and elevation maps for `c1m3` directly from `RESOURCE.RES`, renders them with a CPU heightfield renderer, and lets you take off and fly through three waypoints in a navigation training mission. The default chase camera shows the original high-detail desert Comanche exterior assembled from the `PLAYER.AI` model parts, with its textures and animated rotors. It uses simplified flight controls and a training route written for the prototype.

> [!IMPORTANT]
> You need your own copy of the DOS edition’s `RESOURCE.RES`. The repository contains the engine source, tools, and research notes. Original game data stays in your local installation.

## 📊 Progress

| Area | Status | Details |
|---|---|---|
| Resource archive and terrain | ✅ MVP | Direct `RESOURCE2xxx` loading and LZP1 decoding for the two `c1m3` terrain layers |
| Voxel-style renderer | ✅ MVP | Perspective terrain, fog, waypoint HUD, and 1080p and 4K render modes |
| Playable flight | ✅ MVP | Takeoff, steering, altitude control, and three ordered waypoint triggers and mission completion |
| Mission scripting | 🔍 Under analysis | Rust parser and interpreter driving the training route. 2,124 campaign rules inventoried · [trigger census](docs/mission-triggers.md) |
| Original flight behavior and AI | ⏳ Planned | Compare behavior against the DOS executable |
| Platform validation | 🟡 Partial | Interactive demo verified on Linux. Windows validation is pending. |

## 🧭 Roadmap

See the [work plan](docs/PLAN.md) for dependencies and acceptance criteria.

1. Load the original terrain and deliver a playable navigation training mission. **Complete.**
2. Verify trigger timing, condition order, and AI state changes against the DOS game.
3. Expand the trigger vocabulary and validate it with new training missions.
4. Validate Windows builds, rendering performance, and input on both platforms.

## 🔧 Building and running

Install stable Rust and a C toolchain. The Linux window uses X11, including XWayland on a Wayland desktop.

```sh
git clone https://github.com/pedryvo/comanche3-decomp.git
cd comanche3-decomp
cargo build --release

# Run from the directory containing the original RESOURCE.RES.
cd /path/to/comanche3
/path/to/comanche3-decomp/target/release/comanche3-mvp --1080p
```

Hold **Space + W** to take off toward the first waypoint. Use **W/S** to accelerate and brake, **A/D** to steer, **Space/Shift** to climb and descend, **V** to toggle between the chase and first-person cameras, **R** to restart, and **Esc** to quit.

- `--1080p` renders at 1920 × 1080. `--4k` renders at 3840 × 2160.
- `--cockpit` starts in first-person view.
- `--demo` flies the waypoint route automatically in the window.
- `--smoke` verifies terrain loading, ordered waypoint completion, reset, and rendering without opening a window. Combine it with either resolution flag.

In the chase view, drag with the **right mouse button** to orbit the helicopter. **J/L** orbit left and right, and **I/K** raise and lower the view.

## 🧰 Repository layout

| Path | Purpose |
|---|---|
| [`src/terrain.rs`](src/terrain.rs) | Archive reader, LZP1 decoder, and terrain sampling |
| [`src/render.rs`](src/render.rs) | CPU heightfield renderer and waypoint marker |
| [`src/aircraft.rs`](src/aircraft.rs) | Original 3DO exterior mesh, PCX textures, and rotor rendering |
| [`src/flight.rs`](src/flight.rs) | Prototype flight controls and airborne state |
| [`src/main.rs`](src/main.rs) | Window, keyboard input, HUD, and smoke test |
| [`src/mission.rs`](src/mission.rs), [`src/trigger_runtime.rs`](src/trigger_runtime.rs) | Mission trigger parser and headless rule evaluator |
| [`src/training.rs`](src/training.rs) | First training mission and its connection to flight state |
| [`tools/trigger_census.py`](tools/trigger_census.py) | Inventory of conditions, operators, and actions in extracted mission files |
| [`docs/mission-triggers.md`](docs/mission-triggers.md) | Reproducible census and open questions, in Brazilian Portuguese |

## 🔬 Methodology

The [Sioux project](https://github.com/hanatos/sioux) provides a reference for archive and terrain formats. The Rust decoder’s elevation indices and RGB samples were compared against the Sioux decoder across the full 1024 × 1024 map. The next fidelity milestone is to compare mission events and flight behavior against the DOS game. The REA MCP server identified `C3.EXE` as DOS MZ x86. The current Ghidra checkpoint configuration prevented function-level analysis.

## ⚖️ License

This is an independent fan project. The repository’s source code is licensed under the [GNU General Public License v3.0 only](LICENSE). _Comanche 3_ and its original game data belong to their respective rights holders.
