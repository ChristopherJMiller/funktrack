# Game Design

This document specifies the gameplay mechanics for Rhythm Rail: note types, scoring system, timing windows, controller mappings, and difficulty levels. The design draws from Groove Coaster's proven mechanics while adapting them for open controller hardware.

## Concept

The player avatar travels along a continuous 2D spline path. Notes approach from ahead on the path and must be hit with the correct action at the right moment. The path itself is choreographed to the music — gentle curves for mellow passages, sharp turns for intensity, dramatic sweeps at structural boundaries.

Unlike lane-based rhythm games (DDR, osu!mania, Beat Saber), the spatial dimension is the *path shape itself*. Notes don't occupy discrete lanes — they exist at positions along a continuous curve.

## Note Types

### Tap
The basic note. Any mapped button press scores a hit. Appears as a circle on the path.

### Hold
Press and sustain until the hold trail ends. Releasing early truncates the score proportionally. Visual: circle with a trailing ribbon along the path.

### Slide
Tilt an analog stick (or press a directional key) in the indicated direction. Eight possible directions. Visual: circle with an arrow overlay.

### Slide Hold
Tilt in the indicated direction and sustain. Combines slide and hold mechanics.

### Scratch
Rapidly alternate between two opposite directions on a single analog stick (or two opposing keys). Detection: 3+ zero-crossings on one axis within 500ms. Visual: jagged/zigzag symbol.

### Beat
Rapidly alternate taps between left-hand and right-hand inputs. Visual: double-circle or split symbol.

### Critical
Press both left and right action buttons simultaneously. Timing window for "simultaneous" is ±30ms between the two presses. Visual: large diamond or emphasized circle.

### Critical Hold
Dual press and sustain. Combines critical and hold mechanics.

### Dual Slide
Tilt left and right sticks in two different indicated directions simultaneously. The most demanding note type. Visual: two arrows pointing in different directions.

### Ad-Lib
Invisible bonus notes at specific positions. No visual cue, no miss penalty. Hitting one at the right time awards bonus score. Required for S++ rank. Positions are discoverable through repeated play or community knowledge.

## Timing Windows

Timing is measured as the absolute difference between the player's input timestamp and the note's target time.

| Grade | Window | Score Multiplier | Chain Effect |
|-------|--------|-----------------|--------------|
| GREAT | ≤ 20ms | 100% | +chain |
| COOL | ≤ 50ms | 80% | +chain |
| GOOD | ≤ 100ms | 50% | +chain |
| MISS | > 100ms or no input | 0% | reset chain to 0 |

These windows should be configurable per-difficulty. Easier difficulties could widen the GREAT window to ±33ms and GOOD to ±150ms. A future "strict" mode could tighten GREAT to ±15ms.

Hold notes check timing at the start, then award incremental score for each beat sustained. Releasing early stops score accrual but doesn't count as a miss.

## Scoring

Maximum score per song: **1,000,000 points**, broken down as:

- **Play score** (850,000): distributed across all notes based on grade multiplier
- **Chain bonus** (100,000): awarded proportionally to max combo / total notes
- **Clear bonus** (50,000): flat award for completing the song

### Chain System

The chain (combo) counter affects per-note score through two multiplier thresholds:

- **Normal** (chain 0–9): each hit increments chain by +1
- **FEVER** (chain 10–99): each hit increments chain by +2, visual intensity increase
- **TRANCE** (chain 100+): each hit increments chain by +4, maximum visual intensity

A MISS resets the chain to 0.

### Grade Ranks

| Rank | Requirement |
|------|-------------|
| S++ | 1,000,000 (all GREAT + all Ad-Libs) |
| S+ | ≥ 980,000 |
| S | ≥ 950,000 |
| A | ≥ 900,000 |
| B | ≥ 800,000 |
| C | ≥ 700,000 |
| D | < 700,000 |

## Controller Mappings

### Gamepad (Xbox Layout)

The design maps Groove Coaster's dual-Booster concept to dual analog sticks + shoulder buttons.

| Note Type | Primary Binding | Alt Binding |
|-----------|----------------|-------------|
| Tap | Any face button or bumper | LT / RT |
| Hold | Hold any button | — |
| Slide | Left or right stick in direction | — |
| Slide Hold | Stick direction + held button | — |
| Scratch | Rapidly wiggle one stick | — |
| Beat | Alternate LB and RB | Alternate LT and RT |
| Critical | LB + RB simultaneously | LT + RT simultaneously |
| Critical Hold | LB + RB held | — |
| Dual Slide | Left stick + right stick | — |
| Ad-Lib | Same as Tap (hidden) | — |

### Keyboard

| Note Type | Left Hand | Right Hand |
|-----------|-----------|------------|
| Tap | Space | Right Ctrl |
| Hold | Hold Space | Hold Right Ctrl |
| Slide (left set) | WASD | — |
| Slide (right set) | — | Arrow keys |
| Scratch | Rapid A↔D or W↔S | Rapid Left↔Right or Up↔Down |
| Beat | Alternate Space and Right Ctrl | — |
| Critical | Space + Right Ctrl | — |
| Dual Slide | WASD + Arrow keys | — |
| Ad-Lib | Same as Tap | — |

All bindings are rebindable through `leafwing-input-manager`'s action mapping system.

### Analog Stick Direction Detection

1. Read `(x, y)` from the stick
2. Check magnitude against dead zone threshold (default: 0.35)
3. If above threshold, compute angle: `atan2(y, x)`
4. Quantize to 8 sectors of 45° each, with sector boundaries at 22.5° offsets
5. Map to `Direction8` enum: N, NE, E, SE, S, SW, W, NW

### Scratch Gesture Detection

Monitor a single axis (x or y) of one analog stick:

1. Track zero-crossings (sign changes) on the axis
2. Use a sliding window of 500ms
3. If 3+ zero-crossings occur within the window, register a scratch
4. Require minimum magnitude of 0.4 on each swing to filter noise

## Difficulty Levels

Four base difficulty levels determine chart density and complexity.

| Level | Note Density | Rhythm | Note Types Used |
|-------|-------------|--------|----------------|
| Easy | ~1× BPM notes/min | Whole beats (1/1) | Tap, Hold |
| Normal | ~2× BPM notes/min | Half beats (1/2) | + Slide, Critical |
| Hard | ~4× BPM notes/min | Quarter beats (1/4) | + Scratch, Beat, Slide Hold |
| Expert | All detected events | 1/8 or 1/16 | All types including Dual Slide |

Minimum inter-note intervals: Easy 500ms, Normal 250ms, Hard 125ms, Expert none.

When auto-generating charts, onset importance ranking determines which notes survive thinning for lower difficulties: `importance = onset_strength × beat_position_weight × phrase_position_weight`. Downbeats are always preferred over off-beats, and first notes of rhythmic phrases are never removed.

## Calibration

A dedicated calibration screen lets players measure and compensate for their hardware latency:

1. **Audio calibration**: play a metronome, player taps along. Measure average offset between tap timestamps and beat timestamps.
2. **Visual calibration**: flash a visual cue on beat, player taps along. Measures combined audio + visual + input latency.
3. The difference between these two measurements isolates visual latency.

Three offset values are stored per player profile and applied to the `SongConductor`:

- `audio_offset_ms`: shifts audio playback timing
- `visual_offset_ms`: shifts note rendering position on path
- `input_offset_ms`: shifts hit judgment window

## Camera Behavior

The camera follows the player avatar along the path with configurable parameters:

- **Look-ahead**: camera leads the avatar position by a configurable beat distance (default: 1.5 beats)
- **Zoom**: adjustable per-section in the chart file (wider zoom for complex path sections)
- **Rotation**: optional — camera can rotate to keep the path's forward direction pointing rightward
- **Transition events**: the chart format supports camera zoom, pan, and rotation events triggered at specific beats
