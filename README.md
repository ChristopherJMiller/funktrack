# Rhythm Rail

**A path-based rhythm game inspired by Groove Coaster, built in Rust with Bevy.**

Notes travel along hand-crafted (or procedurally generated) 2D spline paths synchronized to music. Players hit, hold, slide, and scratch their way through songs using keyboard or gamepad. This is the first open-source path-based rhythm game — no lane grids, just a single continuous curve that dances with the music.

> **Status:** Early development. See the [Roadmap](docs/ROADMAP.md) for current progress.

## Why This Exists

Groove Coaster's "notes on a roller coaster" concept is one of the most compelling ideas in rhythm gaming, but no open-source implementation exists. The Rust/Bevy ecosystem now has every building block needed: mature spline math, precise audio synchronization via Kira, and an ECS architecture that maps naturally to rhythm game entities. This project aims to bring that gameplay style to an open platform with community-driven chart creation.

## Core Features (Planned)

- **Path-based gameplay** — notes travel along arbitrary 2D Catmull-Rom and Bézier spline paths
- **10 note types** — tap, hold, slide (8-directional), scratch, beat, critical, dual slide, and hidden ad-lib notes
- **Precise audio sync** — Kira's clock system as the single source of truth, with linear regression smoothing and configurable latency compensation
- **Gamepad + keyboard** — full dual-analog controller support via `leafwing-input-manager`, with per-player rebindable mappings
- **RON beat map format** — human-readable, comment-friendly, natively supports Rust enums for note types
- **Auto-chart generator** — offline audio analysis pipeline using spectral flux onset detection and beat tracking to produce playable charts from any audio file
- **Built-in editor** — spline drawing, beat-grid note placement, camera timeline, and real-time preview (future)

## Tech Stack

| Layer | Crate | Role |
|-------|-------|------|
| Engine | `bevy` 0.18 | ECS, rendering, windowing |
| Audio | `bevy_kira_audio` / `kira` | Playback, clock sync, scheduling |
| Input | `leafwing-input-manager` | Action mapping, dead zones, chords |
| Path rendering | `bevy_prototype_lyon` | Tessellated 2D vector paths |
| Spline math | `bevy_math::cubic_splines` | Catmull-Rom, Bézier, B-spline curves |
| Serialization | `serde` + `ron` | Beat map format |
| Audio analysis | `symphonia` + `rustfft` | Offline chart generation |

## Getting Started

### Prerequisites

- Rust 1.82+ (edition 2024)
- On Linux: ALSA dev libraries (`libasound2-dev` on Ubuntu/Debian)

### Build and Run

```bash
git clone https://github.com/yourname/rhythm-rail.git
cd rhythm-rail
cargo run --release
```

The `--release` flag matters for performance. Debug builds may not sustain the high frame rates needed for tight input timing.

### Project Structure

```
rhythm-rail/
├── src/
│   ├── main.rs              # App entry, plugin registration
│   ├── conductor.rs          # Song timing (SongConductor resource)
│   ├── path.rs               # Spline loading, sampling, rendering
│   ├── notes.rs              # Note entities, spawning, movement
│   ├── input.rs              # Action definitions, gesture detection
│   ├── scoring.rs            # Hit detection, grading, combo/fever
│   ├── beatmap/
│   │   ├── mod.rs            # BeatMap asset type
│   │   ├── format.rs         # RON/JSON (de)serialization
│   │   └── loader.rs         # Bevy asset loader integration
│   └── ui/
│       ├── hud.rs            # Score, combo, grade display
│       └── menus.rs          # Song select, settings, calibration
├── tools/
│   └── chart_gen/            # Offline auto-chart generator (separate binary)
│       ├── main.rs
│       ├── onset.rs          # Spectral flux / SuperFlux
│       ├── beat_track.rs     # Autocorrelation beat tracker
│       ├── path_gen.rs       # Audio-reactive spline generation
│       └── difficulty.rs     # Onset thresholding & scaling
├── assets/
│   ├── songs/                # Audio files + .ron chart files
│   └── themes/               # Visual themes and sprites
├── docs/                     # Technical documentation
└── Cargo.toml
```

## Documentation

- **[Architecture](docs/ARCHITECTURE.md)** — System design, ECS layout, data flow, and synchronization model
- **[Game Design](docs/GAME_DESIGN.md)** — Note types, scoring, controller mappings, and timing windows
- **[Roadmap](docs/ROADMAP.md)** — Development phases and milestones
- **[Chart Generation](docs/CHART_GENERATION.md)** — Audio analysis pipeline for auto-generating charts
- **[Beat Map Format](docs/BEATMAP_FORMAT.md)** — Specification for the `.ron` chart file format
- **[Resources](docs/RESOURCES.md)** — References, prior art, academic papers, and useful crates

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. Key areas where help is needed:

- Path rendering and visual polish
- Additional note type implementations
- Beat map editor tooling
- Chart creation for songs (once the format stabilizes)
- Testing on different audio backends and controller hardware

## License

Dual-licensed under MIT and Apache 2.0, at your option. This is the Rust ecosystem standard and matches Bevy's own license.

## Legal

"Groove Coaster" is a registered trademark of TAITO Corporation. This project is an independent, original work with no affiliation to TAITO or Square Enix. Game mechanics are not copyrightable under US law — the concept of notes traveling on a curved path is a general gameplay idea. All visual assets, audio, and code in this repository are original or appropriately licensed.
