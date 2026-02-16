use serde::{Deserialize, Serialize};

// Mirror of the game's chart structures from src/beatmap.rs,
// but with Serialize added for RON output generation.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub charter: String,
    pub audio_file: String,
    #[serde(default)]
    pub preview_start_ms: u64,
    #[serde(default = "default_preview_duration")]
    pub preview_duration_ms: u64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub difficulties: Vec<Difficulty>,
}

fn default_preview_duration() -> u64 {
    15000
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    Expert,
}

impl Difficulty {
    pub fn filename(&self) -> &'static str {
        match self {
            Difficulty::Easy => "easy.ron",
            Difficulty::Normal => "normal.ron",
            Difficulty::Hard => "hard.ron",
            Difficulty::Expert => "expert.ron",
        }
    }

    /// Beat-grid quantization resolution (subdivisions per beat).
    pub fn grid_resolution(&self) -> u32 {
        match self {
            Difficulty::Easy => 1,
            Difficulty::Normal => 2,
            Difficulty::Hard => 4,
            Difficulty::Expert => 8,
        }
    }

    /// Percentile threshold — keep onsets above this importance percentile.
    pub fn importance_percentile(&self) -> f64 {
        match self {
            Difficulty::Easy => 0.80,
            Difficulty::Normal => 0.50,
            Difficulty::Hard => 0.20,
            Difficulty::Expert => 0.0,
        }
    }

    /// How many beats a note takes to travel from spawn to judgment.
    pub fn travel_beats(&self) -> f64 {
        match self {
            Difficulty::Easy => 4.0,
            Difficulty::Normal => 3.5,
            Difficulty::Hard => 3.0,
            Difficulty::Expert => 3.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartFile {
    pub difficulty: Difficulty,
    #[serde(default)]
    pub difficulty_rating: u32,
    pub timing_points: Vec<ChartTimingPoint>,
    pub path_segments: Vec<PathSegment>,
    pub notes: Vec<ChartNoteEntry>,
    #[serde(default)]
    pub events: Vec<ChartEvent>,
    #[serde(default = "default_travel_beats")]
    pub travel_beats: f64,
    #[serde(default = "default_look_ahead")]
    pub look_ahead_beats: f64,
}

fn default_travel_beats() -> f64 {
    3.0
}

fn default_look_ahead() -> f64 {
    3.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartTimingPoint {
    pub beat: f64,
    pub bpm: f64,
    #[serde(default = "default_time_sig")]
    pub time_signature: (u32, u32),
}

fn default_time_sig() -> (u32, u32) {
    (4, 4)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathSegment {
    CatmullRom {
        points: Vec<(f32, f32)>,
        start_beat: f64,
        end_beat: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartNoteEntry {
    pub beat: f64,
    pub note_type: ChartNoteType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartNoteType {
    Tap,
    Hold { duration_beats: f64 },
    Slide { direction: SlideDirection },
    SlideHold { direction: SlideDirection, duration_beats: f64 },
    Scratch,
    Beat,
    Critical,
    CriticalHold { duration_beats: f64 },
    DualSlide { left: SlideDirection, right: SlideDirection },
    AdLib,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideDirection {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl SlideDirection {
    pub const ALL: [SlideDirection; 8] = [
        SlideDirection::N,
        SlideDirection::NE,
        SlideDirection::E,
        SlideDirection::SE,
        SlideDirection::S,
        SlideDirection::SW,
        SlideDirection::W,
        SlideDirection::NW,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartEvent {
    pub beat: f64,
    pub event: EventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    CameraZoom { scale: f32, duration_beats: f64 },
    CameraPan { offset: (f32, f32), duration_beats: f64 },
    CameraRotate { angle_degrees: f32, duration_beats: f64 },
    ColorShift { hue: f32, duration_beats: f64 },
    PathGlow { intensity: f32 },
    BackgroundPulse,
    SpeedChange { multiplier: f32, duration_beats: f64 },
}

/// Serialize a ChartFile to pretty-printed RON.
pub fn serialize_chart(chart: &ChartFile) -> Result<String, String> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .separate_tuple_members(false)
        .enumerate_arrays(false);
    ron::ser::to_string_pretty(chart, config)
        .map_err(|e| format!("RON serialization error: {e}"))
}

/// Serialize SongMetadata to pretty-printed RON.
pub fn serialize_metadata(meta: &SongMetadata) -> Result<String, String> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(3)
        .separate_tuple_members(false)
        .enumerate_arrays(false);
    ron::ser::to_string_pretty(meta, config)
        .map_err(|e| format!("RON serialization error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_chart_serialization() {
        let chart = ChartFile {
            difficulty: Difficulty::Normal,
            difficulty_rating: 5,
            timing_points: vec![ChartTimingPoint {
                beat: 0.0,
                bpm: 120.0,
                time_signature: (4, 4),
            }],
            path_segments: vec![PathSegment::CatmullRom {
                points: vec![
                    (-500.0, -200.0),
                    (-250.0, 200.0),
                    (-50.0, -150.0),
                    (50.0, 150.0),
                    (250.0, -200.0),
                    (500.0, 200.0),
                ],
                start_beat: 0.0,
                end_beat: 44.0,
            }],
            notes: vec![
                ChartNoteEntry { beat: 4.0, note_type: ChartNoteType::Tap },
                ChartNoteEntry { beat: 6.0, note_type: ChartNoteType::Slide { direction: SlideDirection::E } },
                ChartNoteEntry { beat: 8.0, note_type: ChartNoteType::Hold { duration_beats: 1.5 } },
                ChartNoteEntry { beat: 12.0, note_type: ChartNoteType::Beat },
                ChartNoteEntry { beat: 14.0, note_type: ChartNoteType::Scratch },
                ChartNoteEntry { beat: 16.0, note_type: ChartNoteType::Critical },
                ChartNoteEntry { beat: 18.0, note_type: ChartNoteType::DualSlide { left: SlideDirection::N, right: SlideDirection::E } },
                ChartNoteEntry { beat: 20.0, note_type: ChartNoteType::AdLib },
            ],
            events: Vec::new(),
            travel_beats: 3.0,
            look_ahead_beats: 3.0,
        };

        let ron_str = serialize_chart(&chart).expect("serialization failed");
        let deserialized: ChartFile = ron::from_str(&ron_str).expect("deserialization failed");

        assert_eq!(deserialized.notes.len(), chart.notes.len());
        assert_eq!(deserialized.difficulty_rating, 5);
        assert_eq!(deserialized.timing_points[0].bpm, 120.0);
    }
}
