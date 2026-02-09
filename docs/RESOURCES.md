# Resources

References, prior art, academic papers, and crate links organized by topic.

## Bevy & Rendering

- [Bevy Engine](https://bevyengine.org/) — the game engine
- [`bevy_math::cubic_splines` docs](https://docs.rs/bevy_math/latest/bevy_math/cubic_splines/) — Catmull-Rom, Bézier, Hermite, B-spline, NURBS
- [`bevy_prototype_lyon`](https://github.com/Nilirad/bevy_prototype_lyon) — 2D vector path tessellation via lyon. Target for production path rendering.
- [`bevy_vector_shapes`](https://github.com/james-j-obrien/bevy_vector_shapes) — SDF shader-based immediate-mode 2D shapes
- [`bevy_pen_tool`](https://github.com/nickkwas/bevy_pen_tool) — interactive Bézier curve editor component. Potential foundation for the chart editor.
- [Bevy issue #17587](https://github.com/bevyengine/bevy/issues/17587) — tracking built-in Bézier path rendering
- [Bevy issue #13027](https://github.com/bevyengine/bevy/issues/13027) — gizmos always render on top of sprites

## Audio

- [Kira](https://github.com/tesselode/kira) — audio library with clock system, scheduling, effects. The timing backbone.
- [`bevy_kira_audio`](https://github.com/NiklasEi/bevy_kira_audio) — Bevy plugin wrapping Kira. Simplifies playback but doesn't expose clocks directly.
- [`bevy_mod_kira`](https://github.com/jcornaz/bevy_mod_kira) — minimal Bevy wrapper exposing all Kira features including clocks.
- [Symphonia](https://github.com/pdeljanov/Symphonia) — pure-Rust audio decoder (MP3, OGG, FLAC, WAV, AAC). 3.2M+ downloads.
- [`rodio_scheduler`](https://crates.io/crates/rodio_scheduler) — SIMD-accelerated mixing with precise scheduling for hit sounds
- [cpal](https://github.com/RustAudio/cpal) — cross-platform audio I/O (used internally by Kira)

## Input

- [`leafwing-input-manager`](https://github.com/leafwing-studios/leafwing-input-manager) — action mapping, dual-axis analog, dead zones, chords. 881 stars, being considered for Bevy upstream.
- [gilrs](https://gitlab.com/gilrs-project/gilrs) — gamepad abstraction (used internally by Bevy)
- [Bevy issue #9087](https://github.com/bevyengine/bevy/issues/9087) — input timing precision limited to one frame. Critical limitation for rhythm games.

## Audio Analysis & Signal Processing

### Crates

- [`rustfft`](https://crates.io/crates/rustfft) — FFT with SIMD auto-detection (AVX, SSE4.1, NEON). 11.3M downloads.
- [`realfft`](https://crates.io/crates/realfft) — real-valued FFT wrapper, ~2× speedup over complex FFT for audio.
- [`mel_spec`](https://github.com/wavey-ai/mel_spec) — mel filterbank matching librosa within 1e-7, ~480× faster than real-time.
- [`spectrograms`](https://crates.io/crates/spectrograms) — type-safe STFT, mel, MFCC computation.
- [`ferrous-waves`](https://github.com/ferrous-waves/ferrous-waves) — pure-Rust onset detection, beat tracking, pitch/key detection, LUFS. Newer; verify maturity.
- [`aubio-rs`](https://crates.io/crates/aubio-rs) — safe Rust bindings to aubio (onset, beat, tempo). `builtin` feature compiles from source.
- [`dasp`](https://crates.io/crates/dasp) — digital audio signal processing primitives (sample types, interpolation, ring buffers).
- [`noise`](https://crates.io/crates/noise) — Perlin/simplex/value noise for procedural path generation.

### Python Interop (Optional)

- [PyO3](https://github.com/PyO3/pyo3) — embed Python in Rust. Enables calling librosa/madmom directly.
- [librosa](https://librosa.org/) — Python audio analysis reference implementation
- [madmom](https://github.com/CPJKU/madmom) — RNN-based onset/beat detection (~90% F1). Gold standard for accuracy.

## Serialization & Assets

- [`ron`](https://crates.io/crates/ron) — Rusty Object Notation. Native enum support, comments.
- [`bevy_common_assets`](https://github.com/NiklasEi/bevy_common_assets) — asset loader for RON, JSON, TOML, etc.
- [`postcard`](https://crates.io/crates/postcard) — compact binary serialization for distribution builds.

## UI & Animation

- [`bevy_egui`](https://github.com/mvlabat/bevy_egui) — immediate-mode GUI for debug/editor panels.
- [`bevy_tweening`](https://github.com/djeedai/bevy_tweening) — easing-based animations for hit effects and UI transitions.
- [`bevy_time_runner`](https://github.com/Multirious/bevy_time_runner) — timeline-based animation, explicitly targets rhythm games.

## Existing Rhythm Game Projects

### Bevy-Based

- [`bevy_rhythm`](https://github.com/guimcaballero/bevy_rhythm) — tutorial-quality Bevy rhythm game (64 stars, Bevy ~0.5 era). Outdated but conceptually useful.
- **Machitan Matsuri Mambo** — more recent Bevy rhythm game, 8-lane design.

### Other Open-Source Rhythm Games

- [Cytoid](https://github.com/Cytoid/Cytoid) — open-source Cytus clone (Unity/C#, 1.1K stars). Closest to path-based gameplay.
- [Quaver](https://github.com/Quaver/Quaver) — open-source rhythm game with built-in editor (C#, MPL 2.0).
- [Etterna](https://github.com/etternagame/etterna) — StepMania fork, advanced scoring (C++).
- [Rhythmix](https://github.com/?) — Rust HTTP server that generates rhythm game patterns from audio analysis. Validates the decode → FFT → onset → chart pipeline.

### Chart Generation Projects

- [Beat Sage](https://beatsage.com/) — AI Beat Saber chart generator (neural network, Donahue & Agarwal)
- [GenéLive!](https://github.com/KLab/genebeat) — ML chart generation for Love Live! (AAAI 2023, open-source). Beat Guide + multi-scale convolution.
- [osuT5](https://github.com/) — T5 transformer for osu! map generation.
- [BeatMapSynth](https://github.com/) — HMM-based chart gen with Laplacian segmentation. No GPU required.
- [Impulse](https://github.com/) — rhythm game using SuperFlux + Perlin noise for procedural charts. Validates our approach.

## Academic Papers

### Onset Detection

- Böck & Widmer (2013), "Maximum Filter Vibrato Suppression for Onset Detection" — **SuperFlux algorithm**, best non-ML onset detector in MIREX.
- Bello et al. (2005), "A Tutorial on Onset Detection in Music Signals" — comprehensive survey of spectral flux, complex domain, HFC, phase-based methods.

### Beat Tracking

- Ellis (2007), "Beat Tracking by Dynamic Programming" — autocorrelation + DP beat tracking, implemented in librosa.
- Böck, Krebs, Widmer (2016), "Joint Beat and Downbeat Tracking with Recurrent Neural Networks" — madmom's bidirectional LSTM + DBN approach.

### Music Structure

- Foote (2000), "Automatic Audio Segmentation Using a Measure of Audio Novelty" — self-similarity matrix + checkerboard kernel for section boundaries.
- McFee & Ellis (2014), "Analyzing Song Structure with Spectral Clustering" — graph Laplacian approach to structural segmentation.

### ML Chart Generation

- Donahue, Lipton, McAuley (2017), "Dance Dance Convolution" (ICML) — two-stage CNN-LSTM for DDR step placement + selection. The foundational paper.
- Takada et al. (2023), "GenéLive! Generating Rhythm Actions in Love Live!" (AAAI) — Beat Guide + multi-scale CNN, open-source, production-deployed.

## Groove Coaster Reverse Engineering

No public documentation of Groove Coaster's internals exists. Known file formats:

- `.aar` archives with LZSS compression
- `.tumo` model files for chart data
- ALTX texture format
- OGG audio with custom headers

Hardware controller projects on GitHub: `groove_pico`, `GrooveCoasterController`. These are hardware-only — no gameplay code.

The closest documented format for path-based rhythm gameplay is **Arcaea's `.aff` format**, which encodes curved arcs with easing functions between start/end positions.

## Licensing Reference

- **Bevy**: MIT / Apache-2.0
- **Kira**: MIT / Apache-2.0
- **leafwing-input-manager**: MIT / Apache-2.0
- **Symphonia**: MPL 2.0
- **rustfft**: MIT / Apache-2.0
- "Groove Coaster" is a trademark of TAITO Corporation (Square Enix subsidiary)
- Game mechanics are not copyrightable under US law
