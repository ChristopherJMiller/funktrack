# Architecture

This document describes the technical architecture of Rhythm Rail, covering the ECS entity/component design, system execution order, audio synchronization model, and path rendering pipeline.

## High-Level Data Flow

```
                    ┌──────────────┐
                    │  .ron chart  │
                    │  asset file  │
                    └──────┬───────┘
                           │ Bevy asset loader
                           ▼
┌─────────────┐    ┌──────────────┐    ┌──────────────────┐
│  Kira audio │───▶│ SongConductor│───▶│  Note spawner    │
│  clock tick │    │  (resource)  │    │  (look-ahead)    │
└─────────────┘    └──────┬───────┘    └────────┬─────────┘
                          │                     │
                          │ current_beat        │ spawn entities
                          ▼                     ▼
                   ┌──────────────┐    ┌──────────────────┐
                   │  Note mover  │◀───│  Note entities   │
                   │  (system)    │    │  w/ components   │
                   └──────┬───────┘    └──────────────────┘
                          │
                          │ path.position(t)
                          ▼
                   ┌──────────────┐    ┌──────────────────┐
                   │  Hit detect  │◀───│  Player input    │
                   │  + scoring   │    │  (events)        │
                   └──────────────┘    └──────────────────┘
```

## ECS Design

### Resources (Global State)

**`SongConductor`** is the single source of timing truth. Each frame it polls Kira's audio clock and applies linear regression smoothing to produce a stable, monotonically increasing beat position.

```rust
struct SongConductor {
    current_beat: f64,         // smoothed beat position (e.g., 32.75)
    current_time_secs: f64,    // corresponding wall time
    bpm: f64,                  // current BPM (changes at timing points)
    playing: bool,
    audio_offset_ms: f64,      // hardware latency compensation
    visual_offset_ms: f64,     // monitor latency compensation
    input_offset_ms: f64,      // per-player calibration
    // Linear regression state
    time_samples: VecDeque<(f64, f64)>,  // (game_time, audio_time) pairs
}
```

**`ScoreState`** tracks the current song's scoring.

```rust
struct ScoreState {
    score: u32,                // 0–1,000,000
    chain: u32,                // current combo
    max_chain: u32,
    great_count: u32,
    cool_count: u32,
    good_count: u32,
    miss_count: u32,
    fever_active: bool,        // chain ≥ 10
    trance_active: bool,       // chain ≥ 100
}
```

**`ActiveBeatMap`** holds the loaded chart data (path segments, note list, timing points, events).

### Components (Per-Entity Data)

Notes are spawned as Bevy entities with these components:

```rust
#[derive(Component)]
struct NoteType(NoteKind);  // Tap, Hold, Slide, Scratch, etc.

#[derive(Component)]
struct NoteTiming {
    target_beat: f64,          // when the player should hit
    path_parameter: f64,       // resolved position on spline at target_beat
    spawn_beat: f64,           // when this entity was spawned (for approach calc)
}

#[derive(Component)]
struct NoteProgress(f64);      // 0.0 (spawned) → 1.0 (at judgment line)

#[derive(Component)]
struct NoteAlive;              // marker, removed on hit or miss

#[derive(Component)]
struct HoldState {             // only on Hold/SlideHold/CriticalHold notes
    end_beat: f64,
    held: bool,
}
```

The path itself is not an entity — it's stored in the `ActiveBeatMap` resource and rendered by a dedicated path rendering system.

## System Execution Order

Ordering is critical for a rhythm game. A single frame of lag between input reading and hit detection can cause phantom misses.

```
Schedule: Update
│
├─ 1. update_conductor          (read Kira clock → update SongConductor)
├─ 2. spawn_notes               (check look-ahead window → spawn note entities)
├─ 3. read_input                (collect InputActions from leafwing-input-manager)
├─ 4. move_notes                (advance NoteProgress based on conductor beat)
├─ 5. check_hits                (compare input timing vs note timing → grade)
├─ 6. despawn_missed            (remove notes past the miss window)
├─ 7. update_score              (tally grades → update ScoreState)
├─ 8. update_combo_effects      (check fever/trance thresholds)
├─ 9. render_path               (draw spline via bevy_prototype_lyon)
├─ 10. render_notes             (position sprites on path, animate)
├─ 11. render_hud               (score, combo, grade counters)
└─ 12. animate_feedback         (hit flashes, miss shakes, combo popups)
```

Steps 1–2 run in `PreUpdate` or very early `Update`. Input is read as `EventReader<GamepadEvent>` / `EventReader<KeyboardInput>` rather than `ButtonInput` polling, to preserve within-frame event ordering.

## Audio Synchronization Model

This is the highest-risk technical area. The architecture follows the proven approach documented by DDRKirby(ISQ) and the Rhythm Quest devlogs.

### The Problem

Game frame time (`Time::delta()`) drifts relative to the audio thread's clock. Accumulating deltas will desync after minutes of play. Querying the audio clock directly gives accurate but jittery readings (due to audio buffer boundaries). We need both accuracy and smoothness.

### The Solution: Linear Regression Smoothing

Each frame:

1. Sample game wall time and Kira's `ClockHandle::time()` (tick + fractional beat)
2. Push the pair into a rolling window of 10–15 samples
3. Compute linear regression: `predicted_audio_time = slope * game_time + intercept`
4. If prediction diverges from actual by >50ms, discard window and resync
5. Use the regression output as `SongConductor::current_beat` — this is smooth and monotonic

The slope naturally compensates for minor clock rate differences between CPU and audio hardware.

### Latency Compensation

Three independent offsets are applied:

- **Audio offset**: delay from audio command → sound reaching ears. Ranges from 2–10ms (ASIO/JACK) to 50–100ms (legacy DirectSound). On Windows, enabling cpal's `asio` feature flag can save ~30–35ms.
- **Visual offset**: monitor response time + vsync delay. Typically 5–20ms.
- **Input offset**: physical keypress → game event timestamp. Measured via tap-test calibration screen.

Notes are rendered at `current_beat - visual_offset` but judged at `current_beat - input_offset`. This means the visual and judgment positions are slightly different, which is standard in rhythm games.

### Kira Integration

`bevy_kira_audio` simplifies Kira's API but does not expose the clock system directly. Two options:

1. **Use Kira's `AudioManager` directly** alongside the bevy_kira_audio plugin, creating and managing clocks manually.
2. **Use `bevy_mod_kira`** instead — a minimal wrapper that exposes all Kira features including clocks.

The clock is configured with `ClockSpeed::TicksPerMinute(bpm)` and ticks on the audio thread, making it authoritative.

## Path Rendering Pipeline

### Development Stages

**Stage 1 — Gizmos (prototyping):** Use `gizmos.curve_2d()` for immediate debug rendering. Supports color gradients and dash patterns. Limitation: always renders on top of sprites (Bevy issue #13027).

**Stage 2 — bevy_prototype_lyon (production):** Tessellates 2D paths into real meshes via the lyon library. Supports cubic Bézier segments, arcs, stroke width, fill. Proper z-ordering with other sprites. This is the target for release builds.

### Spline Types

`bevy_math::cubic_splines` provides:

- **`CubicCardinalSpline::new_catmull_rom(points)`** — recommended default. C1-continuous, interpolates through all control points. Ideal for level designers placing waypoints.
- **`CubicBezier`** — explicit control points, useful for precise artistic curves.
- **`CubicHermite`** — position + tangent pairs, good for importing from external editors.

The key API: `curve.position(t)` samples at parameter `t ∈ [0, N]`. Bevy 0.17+ adds `curve.with_derivative()` for tangent-aligned note rotation.

### Arc-Length Reparameterization

Raw spline parameters don't produce uniform speed — notes would cluster in high-curvature regions. The solution is to pre-compute an arc-length lookup table at chart load time:

1. Sample the curve at high resolution (e.g., 1000 points per segment)
2. Accumulate distances to build a `parameter → arc_length` table
3. At runtime, invert via binary search: `desired_arc_length → parameter`

This ensures notes travel at visually constant speed regardless of path shape.

## Beat Map Asset Pipeline

Charts are stored as `.ron` files and loaded through Bevy's asset system via `bevy_common_assets`.

```
assets/songs/example_song/
├── audio.ogg         # song audio
├── easy.ron          # easy difficulty chart
├── normal.ron        # normal difficulty chart
└── metadata.ron      # title, artist, BPM, preview timing
```

The asset loader deserializes the RON into a `BeatMap` struct, pre-computes the spline curves and arc-length tables, and resolves beat positions to path parameters. This happens once at song load, not per-frame.

For binary distribution builds, charts can be pre-compiled to the Postcard binary format for faster loading.

## External Crate Dependencies

### Required

- `bevy` — engine
- `bevy_kira_audio` or `bevy_mod_kira` — audio playback and clock sync
- `kira` — underlying audio library
- `leafwing-input-manager` — input abstraction and action mapping
- `serde` + `ron` — chart serialization
- `bevy_common_assets` — asset loader integration for RON files

### Recommended

- `bevy_prototype_lyon` — production path rendering
- `bevy_tweening` — easing animations for hit effects and UI
- `bevy_egui` — debug UI, future editor panels

### Chart Generation (separate binary)

- `symphonia` — audio decoding
- `rustfft` / `realfft` — spectral analysis
- `noise` or custom Perlin — path generation variation
