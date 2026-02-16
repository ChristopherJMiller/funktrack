# Roadmap

Development is organized into phases. Each phase produces a playable (or at least demonstrable) milestone. The guiding principle is to get notes moving on a path with music as early as possible, then layer on complexity.

## Phase 1 — Path & Notes (Foundation)

**Goal:** A single hardcoded path with notes that travel along it. No audio, no scoring. Prove the core rendering and movement loop.

- [x] Bevy project scaffolding with basic 2D camera
- [x] Load Catmull-Rom spline from hardcoded control points using `bevy_math::cubic_splines`
- [x] Render path using Bevy gizmos (`gizmos.linestrip_2d()`)
- [x] Pre-compute arc-length lookup table for uniform-speed traversal
- [x] Spawn note entities that move along the path at constant speed
- [x] Basic `NoteProgress` component advancing over time
- [x] Tap note positioned and rotated along path tangent (gizmo circles + tangent lines)

**Milestone:** Notes visibly glide along a curved path on screen.

## Phase 2 — Audio Sync

**Goal:** Notes are synchronized to actual music. The conductor system drives all timing.

- [x] Integrate Kira 0.11 directly (thin `KiraPlugin` in `src/audio.rs`)
- [x] Implement `SongConductor` resource reading Kira clock ticks
- [x] Linear regression smoothing (rolling 15-sample window)
- [x] Drift detection and resync (>50ms threshold, 3-frame hard resync)
- [x] Note spawning driven by conductor look-ahead window (3 beats ahead)
- [x] Note movement driven by `conductor.current_beat` rather than wall time
- [x] Timing point support (BPM changes mid-song)

**Milestone:** Notes arrive at the judgment point exactly when the beat hits in the music.

## Phase 3 — Input & Hit Detection

**Goal:** Players can hit notes and receive feedback. Basic keyboard input only.

- [x] Read input via `leafwing-input-manager` `ActionState` (keyboard + gamepad unified)
- [x] Beat-stamped `TapInput` message from Tap action (Space / gamepad South)
- [x] Hit detection: closest-note matching within timing windows
- [x] Timing window grading: GREAT (≤20ms) / COOL (≤50ms) / GOOD (≤100ms) / MISS (>100ms)
- [x] Y2K future punk hit feedback (expanding blast rings, starburst rays, diamond flash)
- [x] Auto-miss detection for notes passing judgment point
- [x] Despawn notes after judgment (hit or miss)
- [x] Judgment point indicator (double white circle at end of path)
- [x] Run at uncapped fps with `PresentMode::AutoNoVsync` for timing precision

**Milestone:** Playable rhythm game loop — hit notes, see feedback, hear music.

## Phase 4 — Scoring & Combo

**Goal:** Full scoring system with chain mechanics.

- [x] `JudgmentResult` message decoupling judgments from feedback/scoring consumers
- [x] `ScoreState` resource tracking score, chain, grade counts
- [x] Per-note score calculation weighted by total note count (850K play score pool)
- [x] Chain incrementing: +1 normal, +2 fever (≥10), +4 trance (≥100)
- [x] Chain reset on miss
- [x] Chain bonus calculation (100K pool, capped)
- [x] Clear bonus (50K)
- [x] End-of-song results screen with grade rank (S++ through D)
- [x] HUD: real-time score, combo counter with tier colors, grade distribution
- [x] Unit tests for scoring math (grade multipliers, chain tiers, rank boundaries, perfect/miss edge cases)

**Milestone:** Complete scoring loop with meaningful feedback on performance quality.

## Phase 5 — Beat Map Format

**Goal:** Charts loaded from files instead of hardcoded data.

- [x] Define serde-serializable chart structs (SongMetadata, ChartFile, full note/path/event enums)
- [x] RON format: metadata, timing points, path segments, notes, events
- [x] Direct filesystem loading with `std::fs` + `ron::de` (simpler than `bevy_common_assets`, matches Kira's direct file I/O)
- [x] Game state machine: `GameScreen` (SongSelect → Playing → Results → SongSelect)
- [x] `OnEnter(Playing)` setup builds SplinePath, NoteQueue, SongConductor from chart data
- [x] `OnExit(Results)` cleanup removes all gameplay resources and stops audio
- [x] Song select screen (scan `assets/songs/`, keyboard nav, Y2K punk aesthetic)
- [x] Support multiple difficulties per song (Left/Right to switch)
- [x] `DespawnOnExit` on UI entities for automatic cleanup on state transitions
- [x] All existing systems tolerate missing resources via `Option<Res<T>>`
- [x] Test charts: 120 BPM (Easy + Normal), 140 BPM (Hard with off-beat patterns)

**Milestone:** New songs can be added by dropping a folder into `assets/songs/`.

## Phase 6 — Gamepad Support & Additional Note Types

**Goal:** Full controller support and the complete note type catalog.

- [x] Gamepad bindings via `leafwing-input-manager` (D-pad nav, South/East buttons)
- [x] Analog stick direction detection (8-way quantization with dead zone)
- [x] Implement Slide notes (directional input check)
- [x] Implement Hold notes (sustained input tracking with partial scoring)
- [x] Implement Scratch notes (zero-crossing gesture detection)
- [x] Implement Beat notes (alternating tap detection)
- [x] Implement Critical notes (simultaneous dual-press with ±30ms window)
- [x] Implement Dual Slide notes
- [x] Implement Ad-Lib notes (invisible, no miss penalty)
- [x] Per-player input remapping UI

**Milestone:** All 10 note types functional on both keyboard and gamepad.

## Phase 7 — Auto-Chart Generator

**Goal:** Offline tool that produces playable charts from any audio file.

- [x] Separate binary crate (`tools/chart_gen`)
- [x] Audio decoding via symphonia
- [x] STFT computation via rustfft/realfft (2048 window, 512 hop, Hann)
- [x] Spectral flux onset detection with adaptive peak picking
- [x] Beat tracking via autocorrelation with ~120 BPM perceptual bias
- [x] Beat-grid quantization of onsets
- [x] Difficulty scaling via onset strength thresholding (Easy through Expert)
- [x] Audio-reactive path generation:
  - [x] Catmull-Rom splines with beat-aligned control points
  - [x] Sub-band energy mapping (bass → sweeps, highs → oscillations)
  - [x] Perlin noise modulation scaled by RMS energy
  - [x] Mean-reversion spring to prevent drift
  - [x] Curvature cap and screen bounds clamping
- [x] Output as `.ron` chart file

**Milestone:** Run `chart_gen song.ogg --difficulty normal` and get a playable chart.

## Phase 8 — Polish & Production Path Rendering

**Goal:** Visual quality suitable for a public release.

- [x] Migrate path rendering from gizmos to `bevy_prototype_lyon`
- [x] Note type-specific lyon shapes and animations (all 10 types)
- [x] Hit effect particles (CPU particle system with lyon shapes)
- [x] FEVER / TRANCE visual escalation (path color/width by chain tier)
- [x] Pause/resume during gameplay (Paused state, audio freeze, overlay UI)
- [x] Camera look-ahead and zoom driven by chart events
- [x] Song preview on select screen
- [x] Calibration screen (tap-test for audio/visual/input offsets)
- [x] Settings menu (key bindings, offsets, volume, display)

**Milestone:** The game looks and feels like a finished product.

## Phase 9 — Editor

**Goal:** In-app beat map editor for community chart creation.

- [x] `bevy_egui` 0.39 UI panels with Y2K Future Punk theme
- [x] Dual-mode editor architecture:
  - [x] **Chart mode** — timeline-dominant with lane-based note visualization (8 lanes), beat grid, playhead cursor, hold-duration bars, scroll-to-zoom
  - [x] **Path mode** — viewport-dominant with lyon-rendered spline preview, Ctrl+Click to add control points, click+drag to move
  - [x] Tab key to switch modes instantly
- [x] Beat-grid note placement (snap to 1/1, 1/2, 1/4, 1/8, 1/16) via `GridSnap` with `[`/`]` cycling
- [x] Note brush selector (all 8 note types, keys 1-8)
- [x] Spline drawing tool (click to place waypoints, drag to adjust control points)
- [x] Real-time playback preview (Space to play/pause, arrow keys to scrub)
- [x] Undo/redo system (Ctrl+Z/Y) with command pattern (`EditorAction` enum)
- [x] Metadata editing panel (title, artist, charter, travel beats, look-ahead)
- [x] Selection system with properties inspector
- [x] Export to `.ron` format (Ctrl+S)
- [x] Export JSON for web-based tooling interop
- [x] JSON import support
- [x] Toast notification system for save feedback
- [x] Editor camera with scroll-to-zoom
- [ ] Waveform/audio visualization in timeline
- [ ] Camera event timeline track
- [ ] Autosave

**Milestone:** Charts can be created entirely within the application.

## Phase 10 — Advanced Chart Generation (Stretch Goal)

**Goal:** ML-enhanced chart quality for the auto-generator.

- [ ] SuperFlux onset detection (max filtering before flux)
- [ ] Sub-band onset detection for richer feature extraction
- [ ] Self-similarity matrix from chroma features for section boundaries
- [ ] Section-aware density variation (sparse verses, dense choruses)
- [ ] Optional: madmom integration via aubio-rs or PyO3 for RNN onset detection
- [ ] Optional: small CNN trained on mel spectrograms for onset probability
- [ ] Note type assignment based on audio features (percussive → tap, sustained → hold, pitched bends → slide)

**Milestone:** Auto-generated charts that feel like they understand the music's structure.

## Non-Goals (For Now)

These are explicitly out of scope for the initial development arc:

- Multiplayer / online play
- 3D rendering (paths and notes are 2D)
- Mobile/touch input
- Video background playback
- Online leaderboards
- Song download / marketplace
