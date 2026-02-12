use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::Deserialize;

use crate::audio::{KiraContext, play_song};
use crate::conductor::{SongConductor, TimingPoint};
use crate::notes::{ChartNote, NoteKind, NoteQueue};
use crate::path::SplinePath;
use crate::results::SongComplete;
use crate::state::GameScreen;

pub struct BeatMapPlugin;

impl Plugin for BeatMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::Playing), setup_playing);
    }
}

// --- Serde data structures ---

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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

    pub fn label(&self) -> &'static str {
        match self {
            Difficulty::Easy => "EASY",
            Difficulty::Normal => "NORMAL",
            Difficulty::Hard => "HARD",
            Difficulty::Expert => "EXPERT",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct ChartTimingPoint {
    pub beat: f64,
    pub bpm: f64,
    #[serde(default = "default_time_sig")]
    pub time_signature: (u32, u32),
}

fn default_time_sig() -> (u32, u32) {
    (4, 4)
}

#[derive(Debug, Clone, Deserialize)]
pub enum PathSegment {
    CatmullRom {
        points: Vec<(f32, f32)>,
        start_beat: f64,
        end_beat: f64,
    },
    Bezier {
        control_points: Vec<(f32, f32)>,
        start_beat: f64,
        end_beat: f64,
    },
    Arc {
        center: (f32, f32),
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        start_beat: f64,
        end_beat: f64,
    },
    Linear {
        start: (f32, f32),
        end: (f32, f32),
        start_beat: f64,
        end_beat: f64,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChartNoteEntry {
    pub beat: f64,
    pub note_type: ChartNoteType,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SlideDirection {
    N, NE, E, SE, S, SW, W, NW,
}

impl SlideDirection {
    /// Unit vector for this direction.
    pub fn to_vec2(self) -> Vec2 {
        use std::f32::consts::FRAC_1_SQRT_2;
        match self {
            SlideDirection::N  => Vec2::new(0.0, 1.0),
            SlideDirection::NE => Vec2::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2),
            SlideDirection::E  => Vec2::new(1.0, 0.0),
            SlideDirection::SE => Vec2::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
            SlideDirection::S  => Vec2::new(0.0, -1.0),
            SlideDirection::SW => Vec2::new(-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
            SlideDirection::W  => Vec2::new(-1.0, 0.0),
            SlideDirection::NW => Vec2::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
        }
    }

    /// Quantize an input vector to one of 8 directions.
    /// Returns `None` for near-zero vectors (dead zone).
    pub fn from_vec2(v: Vec2) -> Option<Self> {
        if v.length_squared() < 0.01 {
            return None;
        }
        let angle = v.y.atan2(v.x); // radians, -π..π
        // Divide circle into 8 sectors of π/4 each, centered on each direction
        let sector = ((angle + std::f32::consts::PI / 8.0).rem_euclid(std::f32::consts::TAU)
            / (std::f32::consts::PI / 4.0)) as u32;
        Some(match sector {
            0 => SlideDirection::E,
            1 => SlideDirection::NE,
            2 => SlideDirection::N,
            3 => SlideDirection::NW,
            4 => SlideDirection::W,
            5 => SlideDirection::SW,
            6 => SlideDirection::S,
            7 => SlideDirection::SE,
            _ => SlideDirection::E, // wrap-around edge case
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChartEvent {
    pub beat: f64,
    pub event: EventType,
}

#[derive(Debug, Clone, Deserialize)]
pub enum EventType {
    CameraZoom { scale: f32, duration_beats: f64 },
    CameraPan { offset: (f32, f32), duration_beats: f64 },
    CameraRotate { angle_degrees: f32, duration_beats: f64 },
    ColorShift { hue: f32, duration_beats: f64 },
    PathGlow { intensity: f32 },
    BackgroundPulse,
    SpeedChange { multiplier: f32, duration_beats: f64 },
}

// --- Runtime resource ---

#[derive(Resource)]
pub struct SelectedSong {
    pub song_dir: PathBuf,
    pub difficulty: Difficulty,
    pub metadata: SongMetadata,
    pub chart: ChartFile,
}

// --- Song discovery & loading ---

pub struct DiscoveredSong {
    pub dir: PathBuf,
    pub metadata: SongMetadata,
}

pub fn discover_songs(songs_root: &Path) -> Vec<DiscoveredSong> {
    let mut songs = Vec::new();

    let entries = match std::fs::read_dir(songs_root) {
        Ok(e) => e,
        Err(err) => {
            warn!("Failed to read songs directory {:?}: {}", songs_root, err);
            return songs;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let meta_path = path.join("metadata.ron");
        if !meta_path.exists() {
            continue;
        }
        match load_metadata(&meta_path) {
            Ok(metadata) => {
                songs.push(DiscoveredSong {
                    dir: path,
                    metadata,
                });
            }
            Err(err) => {
                warn!("Failed to load metadata from {:?}: {}", meta_path, err);
            }
        }
    }

    songs.sort_by(|a, b| a.metadata.title.cmp(&b.metadata.title));
    songs
}

pub fn load_metadata(path: &Path) -> Result<SongMetadata, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {e}"))?;
    ron::from_str(&contents)
        .map_err(|e| format!("parse error: {e}"))
}

pub fn load_chart(song_dir: &Path, difficulty: Difficulty) -> Result<ChartFile, String> {
    let chart_path = song_dir.join(difficulty.filename());
    let contents = std::fs::read_to_string(&chart_path)
        .map_err(|e| format!("read error for {:?}: {e}", chart_path))?;
    ron::from_str(&contents)
        .map_err(|e| format!("parse error for {:?}: {e}", chart_path))
}

// --- OnEnter(Playing) setup ---

fn setup_playing(
    mut commands: Commands,
    mut ctx: NonSendMut<KiraContext>,
    selected: Res<SelectedSong>,
) {
    // 1. Build SplinePath from CatmullRom segments
    let mut all_points: Vec<Vec2> = Vec::new();
    for seg in &selected.chart.path_segments {
        match seg {
            PathSegment::CatmullRom { points, .. } => {
                for &(x, y) in points {
                    all_points.push(Vec2::new(x, y));
                }
            }
            other => {
                warn!("Unsupported path segment type {:?}, skipping", std::mem::discriminant(other));
            }
        }
    }

    if all_points.len() < 4 {
        error!("Need at least 4 control points for CatmullRom spline, got {}", all_points.len());
        return;
    }

    let spline_path = SplinePath::from_catmull_rom_points(all_points);
    commands.insert_resource(spline_path);

    // 2. Build NoteQueue (Tap notes only for now)
    let mut notes = Vec::new();
    for entry in &selected.chart.notes {
        match entry.note_type {
            ChartNoteType::Tap => {
                notes.push(ChartNote {
                    target_beat: entry.beat,
                    kind: NoteKind::Tap,
                });
            }
            ChartNoteType::Slide { direction } => {
                notes.push(ChartNote {
                    target_beat: entry.beat,
                    kind: NoteKind::Slide(direction),
                });
            }
            ref other => {
                warn!("Unsupported note type {:?}, skipping", std::mem::discriminant(other));
            }
        }
    }
    notes.sort_by(|a, b| a.target_beat.partial_cmp(&b.target_beat).unwrap());

    commands.insert_resource(NoteQueue {
        notes,
        next_index: 0,
        look_ahead_beats: selected.chart.look_ahead_beats,
        travel_beats: selected.chart.travel_beats,
    });

    // 3. Build SongConductor
    let first_tp = selected.chart.timing_points.first()
        .expect("Chart must have at least one timing point");
    let bpm = first_tp.bpm;

    let remaining_timing_points: Vec<TimingPoint> = selected.chart.timing_points.iter()
        .skip(1)
        .map(|tp| TimingPoint { beat: tp.beat, bpm: tp.bpm })
        .collect();

    let mut conductor = SongConductor::new(bpm);
    conductor.timing_points = remaining_timing_points;
    commands.insert_resource(conductor);

    // 4. Play song
    let audio_path = selected.song_dir.join(&selected.metadata.audio_file);
    let audio_str = audio_path.to_str().expect("audio path must be valid UTF-8");
    play_song(&mut ctx, audio_str, bpm);

    // 5. Insert SongComplete
    commands.insert_resource(SongComplete(false));

    info!(
        "Playing: {} [{}] — {} BPM",
        selected.metadata.title,
        selected.chart.difficulty.label(),
        bpm
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_direction_roundtrip_cardinals() {
        for dir in [
            SlideDirection::N, SlideDirection::E,
            SlideDirection::S, SlideDirection::W,
        ] {
            let v = dir.to_vec2();
            let back = SlideDirection::from_vec2(v).unwrap();
            assert_eq!(dir, back, "roundtrip failed for {:?}", dir);
        }
    }

    #[test]
    fn slide_direction_roundtrip_diagonals() {
        for dir in [
            SlideDirection::NE, SlideDirection::SE,
            SlideDirection::SW, SlideDirection::NW,
        ] {
            let v = dir.to_vec2();
            let back = SlideDirection::from_vec2(v).unwrap();
            assert_eq!(dir, back, "roundtrip failed for {:?}", dir);
        }
    }

    #[test]
    fn slide_direction_dead_zone() {
        assert!(SlideDirection::from_vec2(Vec2::ZERO).is_none());
        assert!(SlideDirection::from_vec2(Vec2::new(0.05, 0.05)).is_none());
    }

    #[test]
    fn slide_direction_from_vec2_off_axis() {
        // Slightly off-axis should still quantize correctly
        let v = Vec2::new(0.9, 0.2); // mostly east
        assert_eq!(SlideDirection::from_vec2(v), Some(SlideDirection::E));

        let v = Vec2::new(0.2, 0.9); // mostly north
        assert_eq!(SlideDirection::from_vec2(v), Some(SlideDirection::N));
    }
}
