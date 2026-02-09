# Beat Map Format

Rhythm Rail charts use **RON (Rusty Object Notation)** as the primary format. RON was chosen because it natively supports Rust enums (critical for note types), allows comments, and integrates directly with Bevy's asset system via `bevy_common_assets`.

## File Structure

Each song lives in its own directory under `assets/songs/`:

```
assets/songs/stellar_drive/
├── audio.ogg              # song audio (OGG Vorbis preferred)
├── metadata.ron           # song info shared across difficulties
├── easy.ron               # chart: easy difficulty
├── normal.ron             # chart: normal difficulty
├── hard.ron               # chart: hard difficulty
└── expert.ron             # chart: expert difficulty
```

## Metadata File

```ron
(
    title: "Stellar Drive",
    artist: "Example Artist",
    charter: "ChartAuthor",
    audio_file: "audio.ogg",
    preview_start_ms: 45000,  // song select preview start
    preview_duration_ms: 15000,
    source: "",               // original game/album if applicable
)
```

## Chart File

A chart file contains timing information, the path definition, note placements, and optional visual events. All timing is expressed in **beats** rather than milliseconds — this makes charts resilient to BPM changes and simplifies authoring.

```ron
(
    difficulty: Normal,
    difficulty_rating: 7,  // 1–15 subjective rating

    // BPM and time signature changes
    timing_points: [
        (beat: 0.0, bpm: 128.0, time_signature: (4, 4)),
        (beat: 64.0, bpm: 140.0, time_signature: (4, 4)),  // BPM change at beat 64
    ],

    // The path: a sequence of curve segments that join end-to-end
    path_segments: [
        CatmullRom(
            points: [(0.0, 0.0), (200.0, 50.0), (400.0, -30.0), (600.0, 80.0), (800.0, 0.0)],
            start_beat: 0.0,
            end_beat: 16.0,
        ),
        Bezier(
            control_points: [(800.0, 0.0), (900.0, 150.0), (1000.0, -100.0), (1100.0, 0.0)],
            start_beat: 16.0,
            end_beat: 24.0,
        ),
        Arc(
            center: (1200.0, 0.0),
            radius: 100.0,
            start_angle: 3.14,  // radians
            end_angle: 0.0,
            start_beat: 24.0,
            end_beat: 32.0,
        ),
    ],

    // Notes: type + beat position. The engine resolves beat → path position at load time.
    notes: [
        (beat: 4.0,  note_type: Tap),
        (beat: 5.0,  note_type: Tap),
        (beat: 6.0,  note_type: Slide(direction: E)),
        (beat: 8.0,  note_type: Hold(duration_beats: 2.0)),
        (beat: 12.0, note_type: Scratch),
        (beat: 14.0, note_type: Critical),
        (beat: 15.5, note_type: Tap),  // off-beat note
        (beat: 16.0, note_type: Beat),
        (beat: 20.0, note_type: DualSlide(left: NW, right: SE)),
        (beat: 24.0, note_type: AdLib),  // hidden bonus note
    ],

    // Optional visual/camera events
    events: [
        (beat: 0.0,  event: CameraZoom(scale: 1.0, duration_beats: 0.0)),
        (beat: 16.0, event: CameraZoom(scale: 0.8, duration_beats: 2.0)),  // zoom out over 2 beats
        (beat: 32.0, event: ColorShift(hue: 180.0, duration_beats: 4.0)),
        (beat: 48.0, event: PathGlow(intensity: 2.0)),
    ],
)
```

## Type Definitions

### NoteType Enum

```ron
// All possible note types
Tap                                       // any single press
Hold(duration_beats: 2.0)                // press and sustain
Slide(direction: N)                       // directional: N, NE, E, SE, S, SW, W, NW
SlideHold(direction: E, duration_beats: 1.5)
Scratch                                   // rapid stick wiggle
Beat                                      // rapid alternating taps
Critical                                  // both buttons simultaneously
CriticalHold(duration_beats: 3.0)
DualSlide(left: NW, right: SE)           // two directions at once
AdLib                                     // invisible bonus note
```

### PathSegment Variants

**CatmullRom** — smooth curve passing through all points. Best for most path sections. Minimum 4 points required.

**Bezier** — cubic Bézier with explicit control points. Groups of 4 points define one segment (start, control1, control2, end). Good for precise artistic shapes.

**Arc** — circular arc segment. Useful for loops and spirals. `start_angle` and `end_angle` are in radians.

**Linear** — straight line between two points. Useful for dramatic contrast after curves.

```ron
Linear(
    start: (500.0, 0.0),
    end: (700.0, 0.0),
    start_beat: 8.0,
    end_beat: 12.0,
)
```

### Event Types

```ron
CameraZoom(scale: 1.2, duration_beats: 2.0)
CameraPan(offset: (100.0, 50.0), duration_beats: 1.0)
CameraRotate(angle_degrees: 15.0, duration_beats: 4.0)
ColorShift(hue: 90.0, duration_beats: 2.0)
PathGlow(intensity: 1.5)
BackgroundPulse
SpeedChange(multiplier: 1.5, duration_beats: 4.0)  // visual speed only, doesn't affect timing
```

## Design Decisions

**Beats, not milliseconds.** Note positions are in beats because this decouples chart authoring from BPM. A chart authored at 120 BPM works identically if the song's BPM is later corrected to 121 — only the timing points need updating, not every note.

**Path segments have beat ranges.** Each path segment spans a `start_beat` to `end_beat` range. The engine linearly maps beat progress within that range to the spline parameter `t ∈ [0, 1]` (after arc-length reparameterization). This means the path's visual shape and the musical timing are authored independently — a fast BPM section doesn't force a longer path.

**Notes reference beats, not path positions.** The engine resolves `note.beat` → find which path segment contains that beat → compute the corresponding path parameter → sample position. Authors only think in beats.

**Adjacent segments must share endpoints.** The last point of one segment should equal the first point of the next to ensure a continuous path. The loader should warn (not error) if there's a gap, and auto-connect with a linear segment.

## Alternative Formats

**JSON** can be used for interop with web-based editors. The same structs serialize to JSON via `serde_json`. The loader accepts both `.ron` and `.json` extensions.

**Postcard** (binary) is used for pre-compiled distribution builds. The `chart_gen` tool can emit either format. Postcard files use the `.chart` extension.

## Prior Art

The format draws ideas from:

- **Arcaea's `.aff` format** — curved arc objects with easing functions between positions (closest existing path-based format)
- **osu!'s `.osu` format** — timing points with beat-based positioning, proven at scale
- **StepMania's `.sm` / `.ssc` format** — BPM change handling and measure-based note placement
