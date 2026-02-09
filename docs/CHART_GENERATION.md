# Chart Generation

The auto-chart generator is a standalone command-line tool (`tools/chart_gen`) that analyzes an audio file and produces a `.ron` chart. It runs offline — all analysis happens before gameplay, never in real-time.

## Usage

```bash
# Generate a normal difficulty chart
cargo run -p chart_gen -- input.ogg --difficulty normal --output chart.ron

# Generate all four difficulties
cargo run -p chart_gen -- input.ogg --all-difficulties --output-dir ./charts/

# Override BPM detection (useful for songs with unstable tempo)
cargo run -p chart_gen -- input.ogg --bpm 128 --difficulty hard
```

## Pipeline Overview

```
Audio File
    │
    ▼
┌─────────────┐
│  Decode      │  symphonia → f32 PCM samples
└──────┬──────┘
       ▼
┌─────────────┐
│  STFT        │  2048 window, 512 hop, Hann → complex spectrogram
└──────┬──────┘
       ▼
┌──────┴──────────────────────────────────┐
│                                          │
▼                                          ▼
┌──────────────┐                 ┌─────────────────┐
│ Spectral Flux │                 │ Mel Spectrogram  │
│ (onsets)      │                 │ (structure)      │
└──────┬───────┘                 └────────┬────────┘
       ▼                                  ▼
┌──────────────┐                 ┌─────────────────┐
│ Peak Picking  │                 │ Beat Tracking    │
│ (timestamps)  │                 │ (grid)           │
└──────┬───────┘                 └────────┬────────┘
       │                                  │
       ▼                                  ▼
┌──────────────────────────────────────────┐
│         Quantize Onsets to Beat Grid      │
└──────────────────┬───────────────────────┘
                   ▼
         ┌─────────────────┐
         │ Difficulty Scale  │  threshold + thin by onset importance
         └────────┬────────┘
                  ▼
         ┌─────────────────┐
         │ Path Generation   │  sub-band energy → spline control points
         └────────┬────────┘
                  ▼
         ┌─────────────────┐
         │ Serialize to RON  │
         └─────────────────┘
```

## Stage 1: Audio Decoding

Symphonia decodes MP3, OGG, FLAC, WAV, and AAC into mono f32 PCM at the native sample rate. For stereo files, mix to mono by averaging channels. Resample to 44100 Hz if the source differs.

## Stage 2: STFT

Compute the Short-Time Fourier Transform with these parameters:

- **Window size:** 2048 samples (~46ms at 44100 Hz)
- **Hop size:** 512 samples (~11.6ms) — this determines the time resolution of onset detection
- **Window function:** Hann

Use `realfft` for ~2× speedup over complex FFT on real-valued audio. The output is a sequence of 1025 complex frequency bins per frame.

## Stage 3: Onset Detection

**Spectral flux** is the default algorithm. For each frame, sum the positive differences between consecutive magnitude spectra:

```
SF(n) = Σ max(0, |X(n,k)| - |X(n-1,k)|)
```

This captures energy increases — exactly what note onsets look like. The result is a 1D onset detection function (ODF) with one value per STFT frame.

**SuperFlux** (Phase 2 upgrade) adds maximum filtering along the frequency axis before computing flux. For each bin `k`, replace `|X(n-1,k)|` with `max(|X(n-1, k-w)|, ..., |X(n-1, k+w)|)` where `w` is typically 3 bins. This suppresses false positives from vibrato and tremolo — it's the best non-learned onset detector in MIREX evaluations.

**Adaptive peak picking** extracts discrete onset timestamps from the ODF:

1. Compute a moving average of the ODF (window ~0.5 seconds, or ~43 frames at 512-hop)
2. An onset is detected where the ODF exceeds `mean + sensitivity × std_dev` (sensitivity default: 1.5)
3. Enforce minimum inter-onset interval of 30–50ms
4. Apply a silence gate: reject onsets where frame energy is below -74 dB
5. The sensitivity parameter directly controls chart density

## Stage 4: Beat Tracking

Beat tracking provides the rhythmic grid. The algorithm:

1. Compute an onset strength envelope from the mel spectrogram (weighted sum across bands)
2. Autocorrelate the envelope to find dominant periodicities
3. Apply a perceptual tempo bias centered at ~120 BPM (Gaussians in the lag domain)
4. Select the dominant tempo
5. Use dynamic programming to find the globally optimal beat sequence that maximizes onset alignment while maintaining even spacing

The output is a list of beat timestamps and the estimated BPM. For songs with tempo changes, the DP alignment will show systematic drift — detect this by monitoring beat-to-beat interval variance and split into segments with stable tempo.

## Stage 5: Quantization

Snap each detected onset to the nearest position on the beat grid. The grid resolution depends on difficulty:

- Easy: 1/1 beats only (whole notes)
- Normal: 1/2 beats (half notes)
- Hard: 1/4 beats (quarter notes)
- Expert: 1/8 beats (eighth notes), with 1/16 allowed for very fast songs

If two onsets quantize to the same grid position, keep the one with higher spectral flux magnitude.

## Stage 6: Difficulty Scaling

Starting from all detected onsets, filter by importance for each difficulty:

```
importance = onset_strength × beat_weight × phrase_weight
```

Where:
- `onset_strength` is the raw spectral flux value (normalized 0–1)
- `beat_weight` is 1.0 for downbeats, 0.8 for beats, 0.5 for off-beats, 0.3 for subdivisions
- `phrase_weight` is 1.0 for first/last note of a detected phrase, 0.7 otherwise

Then for each difficulty, keep the top N% by importance:

| Difficulty | Percentile Threshold | Target Density |
|------------|---------------------|----------------|
| Easy | 80th | ~BPM notes/min |
| Normal | 50th | ~2× BPM notes/min |
| Hard | 20th | ~4× BPM notes/min |
| Expert | All onsets | All detected |

Rules applied after filtering: never remove the first note of a rhythmic phrase, enforce minimum inter-note interval per difficulty, ensure at least one note per 4-beat measure.

## Stage 7: Path Generation

The path is generated from audio features, not from note positions. It should feel like a visual interpretation of the music.

### Control Point Placement

Place one control point per beat (or per half-beat for faster songs). The x-coordinate advances linearly. The y-coordinate is driven by audio features:

```
y(beat) = bass_sweep(beat) + high_oscillation(beat) + noise(beat)
```

Where:
- **`bass_sweep`**: smoothed bass energy (20–250 Hz) scaled to ±200 pixels. Creates large undulations on kick-heavy sections.
- **`high_oscillation`**: high-frequency content (4000–20000 Hz) multiplied by a sine wave at 2× beat frequency, scaled to ±50 pixels. Creates shimmering wiggles on hi-hat/cymbal sections.
- **`noise`**: fractal Brownian motion (Perlin noise, 3–4 octaves, persistence 0.5, lacunarity 2.0), with amplitude modulated by overall RMS energy. Adds organic variation.

### Constraints

- **Mean-reversion spring**: `y_correction = -0.03 × current_y` per beat, pulling toward center
- **Screen bounds**: soft sigmoid clamp at ±40% of screen height
- **Maximum curvature**: turning angle per beat-distance ≤ 120°
- **Forward-only**: x-coordinate always increases (the path never doubles back)

### Section-Aware Modulation

If structural analysis is available (Phase 2+ of the chart gen roadmap):

- **Verse sections**: moderate, predictable curves (amplitude × 0.7)
- **Chorus sections**: amplified, dramatic curves (amplitude × 1.5)
- **Breakdowns/bridges**: minimal curves or spiral patterns
- **Intros/outros**: gentle fade in/out of curve intensity

### Output

Control points are assembled into `CatmullRom` path segments and serialized into the chart's `path_segments` array.

## Crate Dependencies

```toml
[package]
name = "chart_gen"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "chart_gen"
path = "src/main.rs"

[dependencies]
symphonia = { version = "0.5", features = ["all"] }
rustfft = "6.4"
realfft = "3.5"
serde = { version = "1", features = ["derive"] }
ron = "0.8"
clap = { version = "4", features = ["derive"] }
noise = "0.9"  # Perlin noise
```

Optional for advanced analysis:
- `aubio-rs` — bindings to aubio's battle-tested onset/beat detection
- `mel_spec` — mel filterbank computation matching librosa within 1e-7

## References

- Böck & Widmer (2013), "Maximum Filter Vibrato Suppression for Onset Detection" — SuperFlux algorithm
- Ellis (2007), "Beat Tracking by Dynamic Programming" — the beat tracking algorithm used here
- Donahue, Lipton, McAuley (2017), "Dance Dance Convolution" — ML approach for future phases
- The Impulse rhythm game project — validates SuperFlux + Perlin noise for rhythm game chart gen
