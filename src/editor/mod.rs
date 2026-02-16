mod actions;
mod camera;
mod io;
mod viewport;
mod ui;

use std::collections::HashSet;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use crate::beatmap::{
    ChartFile, ChartNoteEntry, ChartNoteType, Difficulty,
    PathSegment, SongMetadata,
};
use crate::state::GameScreen;

pub use self::actions::EditorAction;

pub struct EditorPluginBundle;

impl Plugin for EditorPluginBundle {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
        .add_systems(OnEnter(GameScreen::Editor), setup_editor)
        .add_systems(
            Update,
            (
                input_system,
                ui::editor_ui_system,
            )
                .chain()
                .run_if(in_state(GameScreen::Editor)),
        )
        .add_systems(
            Update,
            (
                camera::editor_camera_system,
                viewport::render_viewport,
            )
                .run_if(in_state(GameScreen::Editor)),
        )
        .add_systems(OnExit(GameScreen::Editor), cleanup_editor);
    }
}

// ─── Editor modes ──────────────────────────────────────────────────
//
// The editor has two major modes reflecting the two creative acts:
//
// 1. **Chart mode** (default) — the timeline is king.
//    A tall horizontal beat-grid timeline dominates the screen.
//    You listen to the music, see the beat grid, and tap notes into
//    beat positions. The game viewport is a small preview in the corner.
//    This is where 90% of charting time is spent.
//
// 2. **Path mode** — the viewport is king.
//    The game viewport fills the center. You click to place/drag
//    Catmull-Rom control points. The timeline shrinks to a thin bar.
//    You design the visual route notes will travel.
//
// Switching is instant (Tab key or mode button).
// ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Chart,
    Path,
}

/// Which note type the user will place next.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NoteBrush {
    #[default]
    Tap,
    Hold { duration_beats: f64 },
    Slide { direction: crate::beatmap::SlideDirection },
    Scratch,
    Beat,
    Critical,
    DualSlide {
        left: crate::beatmap::SlideDirection,
        right: crate::beatmap::SlideDirection,
    },
    AdLib,
}

impl NoteBrush {
    pub fn to_chart_note_type(&self) -> ChartNoteType {
        match self {
            NoteBrush::Tap => ChartNoteType::Tap,
            NoteBrush::Hold { duration_beats } => ChartNoteType::Hold {
                duration_beats: *duration_beats,
            },
            NoteBrush::Slide { direction } => ChartNoteType::Slide {
                direction: *direction,
            },
            NoteBrush::Scratch => ChartNoteType::Scratch,
            NoteBrush::Beat => ChartNoteType::Beat,
            NoteBrush::Critical => ChartNoteType::Critical,
            NoteBrush::DualSlide { left, right } => ChartNoteType::DualSlide {
                left: *left,
                right: *right,
            },
            NoteBrush::AdLib => ChartNoteType::AdLib,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            NoteBrush::Tap => "TAP",
            NoteBrush::Hold { .. } => "HOLD",
            NoteBrush::Slide { .. } => "SLIDE",
            NoteBrush::Scratch => "SCRATCH",
            NoteBrush::Beat => "BEAT",
            NoteBrush::Critical => "CRITICAL",
            NoteBrush::DualSlide { .. } => "DUAL SLIDE",
            NoteBrush::AdLib => "AD-LIB",
        }
    }
}

/// Beat grid snap resolution.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum GridSnap {
    None,
    Whole,
    Half,
    #[default]
    Quarter,
    Eighth,
    Sixteenth,
}

impl GridSnap {
    pub fn divisor(self) -> f64 {
        match self {
            GridSnap::None => 0.0,
            GridSnap::Whole => 1.0,
            GridSnap::Half => 2.0,
            GridSnap::Quarter => 4.0,
            GridSnap::Eighth => 8.0,
            GridSnap::Sixteenth => 16.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GridSnap::None => "FREE",
            GridSnap::Whole => "1/1",
            GridSnap::Half => "1/2",
            GridSnap::Quarter => "1/4",
            GridSnap::Eighth => "1/8",
            GridSnap::Sixteenth => "1/16",
        }
    }

    pub fn snap_beat(self, beat: f64) -> f64 {
        if self == GridSnap::None {
            return beat;
        }
        let d = self.divisor();
        (beat * d).round() / d
    }

    pub fn next(self) -> GridSnap {
        match self {
            GridSnap::Whole => GridSnap::Half,
            GridSnap::Half => GridSnap::Quarter,
            GridSnap::Quarter => GridSnap::Eighth,
            GridSnap::Eighth => GridSnap::Sixteenth,
            GridSnap::Sixteenth => GridSnap::None,
            GridSnap::None => GridSnap::Whole,
        }
    }

    pub fn prev(self) -> GridSnap {
        match self {
            GridSnap::Whole => GridSnap::None,
            GridSnap::Half => GridSnap::Whole,
            GridSnap::Quarter => GridSnap::Half,
            GridSnap::Eighth => GridSnap::Quarter,
            GridSnap::Sixteenth => GridSnap::Eighth,
            GridSnap::None => GridSnap::Sixteenth,
        }
    }
}

/// Playback state within the editor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
}

/// Represents a selectable element.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum EditorElement {
    PathControlPoint { segment: usize, index: usize },
    Note { index: usize },
    Event { index: usize },
}

/// The central editor state resource.
#[derive(Resource)]
pub struct EditorState {
    pub chart: ChartFile,
    pub metadata: SongMetadata,
    pub song_dir: PathBuf,

    pub mode: EditorMode,
    pub note_brush: NoteBrush,
    pub grid_snap: GridSnap,

    pub cursor_beat: f64,
    pub playback: PlaybackState,
    pub total_beats: f64,
    pub timeline_view_beats: f64,

    pub selected: HashSet<EditorElement>,
    pub dragging_cp: Option<(usize, usize)>,

    pub undo_stack: Vec<EditorAction>,
    pub redo_stack: Vec<EditorAction>,
    pub unsaved_changes: bool,

    pub egui_wants_pointer: bool,
    pub toast: Option<(String, f64)>,
}

impl EditorState {
    pub fn new(chart: ChartFile, metadata: SongMetadata, song_dir: PathBuf) -> Self {
        let total_beats = Self::compute_total_beats(&chart);
        Self {
            chart,
            metadata,
            song_dir,
            mode: EditorMode::default(),
            note_brush: NoteBrush::default(),
            grid_snap: GridSnap::default(),
            cursor_beat: 0.0,
            playback: PlaybackState::default(),
            total_beats,
            timeline_view_beats: 16.0,
            selected: HashSet::new(),
            dragging_cp: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            unsaved_changes: false,
            egui_wants_pointer: false,
            toast: None,
        }
    }

    fn compute_total_beats(chart: &ChartFile) -> f64 {
        let last_note = chart.notes.iter().map(|n| n.beat).fold(0.0_f64, f64::max);
        let last_event = chart
            .events
            .iter()
            .map(|e| e.beat)
            .fold(0.0_f64, f64::max);
        (last_note.max(last_event) + 8.0).max(32.0)
    }

    pub fn execute(&mut self, action: EditorAction) {
        action.apply(&mut self.chart);
        self.undo_stack.push(action);
        self.redo_stack.clear();
        self.unsaved_changes = true;
        self.total_beats = Self::compute_total_beats(&self.chart);
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            action.undo(&mut self.chart);
            self.redo_stack.push(action);
            self.unsaved_changes = true;
            self.total_beats = Self::compute_total_beats(&self.chart);
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            action.apply(&mut self.chart);
            self.undo_stack.push(action);
            self.unsaved_changes = true;
            self.total_beats = Self::compute_total_beats(&self.chart);
        }
    }

    pub fn bpm(&self) -> f64 {
        self.chart
            .timing_points
            .first()
            .map(|tp| tp.bpm)
            .unwrap_or(120.0)
    }

    pub fn beat_to_time(&self, beat: f64) -> f64 {
        beat * 60.0 / self.bpm()
    }

    pub fn show_toast(&mut self, msg: impl Into<String>, now: f64) {
        self.toast = Some((msg.into(), now + 2.5));
    }
}

/// Marker for entities spawned by the editor (for cleanup).
#[derive(Component)]
pub struct EditorEntity;

/// Resource to track the song that was selected for editing.
#[derive(Resource)]
pub struct EditingSong {
    pub song_dir: PathBuf,
    pub difficulty: Difficulty,
    pub metadata: SongMetadata,
    pub chart: ChartFile,
}

// ─── Global input system ───────────────────────────────────────────

fn input_system(
    mut state: ResMut<EditorState>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut ctx: NonSendMut<crate::audio::KiraContext>,
    mut next_state: ResMut<NextState<GameScreen>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let now = time.elapsed_secs_f64();

    // Expire toast
    if let Some((_, deadline)) = &state.toast {
        if now > *deadline {
            state.toast = None;
        }
    }

    // ── Escape ──
    if keys.just_pressed(KeyCode::Escape) {
        crate::audio::stop_preview(&mut ctx);
        next_state.set(GameScreen::SongSelect);
        return;
    }

    // ── Ctrl+S save ──
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        let path = state.song_dir.join(state.chart.difficulty.filename());
        match io::save_chart_ron(&state.chart, &path) {
            Ok(()) => {
                state.unsaved_changes = false;
                state.show_toast("Saved", now);
            }
            Err(e) => {
                state.show_toast(format!("Save failed: {e}"), now);
            }
        }
        return;
    }

    // ── Ctrl+Z / Ctrl+Y ──
    if ctrl && keys.just_pressed(KeyCode::KeyZ) {
        state.undo();
        return;
    }
    if ctrl && keys.just_pressed(KeyCode::KeyY) {
        state.redo();
        return;
    }

    // ── Tab to switch mode ──
    if keys.just_pressed(KeyCode::Tab) {
        state.mode = match state.mode {
            EditorMode::Chart => EditorMode::Path,
            EditorMode::Path => EditorMode::Chart,
        };
        return;
    }

    // ── Space: play/pause ──
    if keys.just_pressed(KeyCode::Space) && !ctrl {
        match state.playback {
            PlaybackState::Stopped => {
                state.playback = PlaybackState::Playing;
                let start_ms = (state.beat_to_time(state.cursor_beat) * 1000.0) as u64;
                let total_ms = (state.beat_to_time(state.total_beats) * 1000.0) as u64;
                let remaining = total_ms.saturating_sub(start_ms).max(1000);
                let audio_path = state.song_dir.join(&state.metadata.audio_file);
                if let Some(s) = audio_path.to_str() {
                    crate::audio::play_preview(&mut ctx, s, start_ms, remaining, 0.7);
                }
            }
            PlaybackState::Playing => {
                state.playback = PlaybackState::Stopped;
                crate::audio::stop_preview(&mut ctx);
            }
        }
        return;
    }

    // ── Playback advance ──
    if state.playback == PlaybackState::Playing {
        let bps = state.bpm() / 60.0;
        state.cursor_beat += bps * time.delta_secs_f64();
        if state.cursor_beat >= state.total_beats {
            state.playback = PlaybackState::Stopped;
            state.cursor_beat = 0.0;
            crate::audio::stop_preview(&mut ctx);
        }
        return; // Don't process edit keys during playback
    }

    // ── Grid snap cycling: [ / ] ──
    if keys.just_pressed(KeyCode::BracketLeft) {
        state.grid_snap = state.grid_snap.prev();
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        state.grid_snap = state.grid_snap.next();
    }

    // ── Note brush shortcuts: 1-8 ──
    let brush_keys: [(KeyCode, NoteBrush); 8] = [
        (KeyCode::Digit1, NoteBrush::Tap),
        (KeyCode::Digit2, NoteBrush::Hold { duration_beats: 1.0 }),
        (KeyCode::Digit3, NoteBrush::Slide { direction: crate::beatmap::SlideDirection::E }),
        (KeyCode::Digit4, NoteBrush::Scratch),
        (KeyCode::Digit5, NoteBrush::Beat),
        (KeyCode::Digit6, NoteBrush::Critical),
        (KeyCode::Digit7, NoteBrush::DualSlide {
            left: crate::beatmap::SlideDirection::W,
            right: crate::beatmap::SlideDirection::E,
        }),
        (KeyCode::Digit8, NoteBrush::AdLib),
    ];
    for (key, brush) in brush_keys {
        if keys.just_pressed(key) {
            state.note_brush = brush;
        }
    }

    // ── Arrow keys: timeline navigation ──
    let step = if ctrl {
        1.0
    } else {
        match state.grid_snap {
            GridSnap::None => 0.25,
            other => 1.0 / other.divisor(),
        }
    };
    if keys.just_pressed(KeyCode::ArrowRight) {
        state.cursor_beat = (state.cursor_beat + step).min(state.total_beats);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        state.cursor_beat = (state.cursor_beat - step).max(0.0);
    }
    if keys.just_pressed(KeyCode::Home) {
        state.cursor_beat = 0.0;
    }
    if keys.just_pressed(KeyCode::End) {
        state.cursor_beat = state.total_beats;
    }

    // ── Delete selected ──
    if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
        let mut note_indices: Vec<usize> = state
            .selected
            .iter()
            .filter_map(|e| match e {
                EditorElement::Note { index } => Some(*index),
                _ => None,
            })
            .collect();
        note_indices.sort_unstable();
        note_indices.reverse();
        for index in note_indices {
            if index < state.chart.notes.len() {
                let note = state.chart.notes[index].clone();
                state.execute(EditorAction::RemoveNote { index, note });
            }
        }
        state.selected.clear();
    }

    // ── Enter: place note at cursor (Chart mode) ──
    if state.mode == EditorMode::Chart && keys.just_pressed(KeyCode::Enter) {
        let beat = state.grid_snap.snap_beat(state.cursor_beat);
        let note = ChartNoteEntry {
            beat,
            note_type: state.note_brush.to_chart_note_type(),
        };
        state.execute(EditorAction::AddNote { note });
    }

    // ── Path mode: mouse interaction for control points ──
    if state.mode == EditorMode::Path && !state.egui_wants_pointer {
        let Ok((camera, camera_transform)) = camera_q.single() else {
            return;
        };
        let Ok(window) = windows.single() else {
            return;
        };
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        let Some(world_pos) = camera::screen_to_world(camera, camera_transform, cursor_pos)
        else {
            return;
        };

        if mouse.just_pressed(MouseButton::Left) {
            if ctrl {
                let point = (world_pos.x, world_pos.y);
                if state.chart.path_segments.is_empty() {
                    let total = state.total_beats;
                    state.chart.path_segments.push(PathSegment::CatmullRom {
                        points: vec![point],
                        start_beat: 0.0,
                        end_beat: total,
                    });
                    state.unsaved_changes = true;
                } else {
                    state.execute(EditorAction::AddPathPoint {
                        segment: 0,
                        point,
                    });
                }
            } else if let Some((seg, idx)) =
                find_nearest_cp(&state.chart.path_segments, world_pos, 20.0)
            {
                state.dragging_cp = Some((seg, idx));
                state.selected.clear();
                state.selected.insert(EditorElement::PathControlPoint {
                    segment: seg,
                    index: idx,
                });
            }
        }

        if mouse.pressed(MouseButton::Left) {
            if let Some((seg, idx)) = state.dragging_cp {
                if let Some(s) = state.chart.path_segments.get_mut(seg) {
                    if let PathSegment::CatmullRom { points, .. } = s {
                        if idx < points.len() {
                            points[idx] = (world_pos.x, world_pos.y);
                            state.unsaved_changes = true;
                        }
                    }
                }
            }
        }

        if mouse.just_released(MouseButton::Left) {
            state.dragging_cp = None;
        }
    }
}

fn find_nearest_cp(
    segments: &[PathSegment],
    pos: Vec2,
    max_dist: f32,
) -> Option<(usize, usize)> {
    let mut best = None;
    let mut best_d = max_dist;
    for (si, seg) in segments.iter().enumerate() {
        if let PathSegment::CatmullRom { points, .. } = seg {
            for (pi, &(x, y)) in points.iter().enumerate() {
                let d = pos.distance(Vec2::new(x, y));
                if d < best_d {
                    best_d = d;
                    best = Some((si, pi));
                }
            }
        }
    }
    best
}

// ─── Setup / Cleanup ───────────────────────────────────────────────

fn setup_editor(mut commands: Commands, editing: Option<Res<EditingSong>>) {
    let Some(editing) = editing else {
        warn!("No song selected for editing, returning to song select");
        commands.insert_resource(NextState::<GameScreen>::Pending(GameScreen::SongSelect));
        return;
    };
    let state = EditorState::new(
        editing.chart.clone(),
        editing.metadata.clone(),
        editing.song_dir.clone(),
    );
    commands.insert_resource(state);
}

fn cleanup_editor(
    mut commands: Commands,
    editor_entities: Query<Entity, With<EditorEntity>>,
) {
    commands.remove_resource::<EditorState>();
    for entity in &editor_entities {
        commands.entity(entity).despawn();
    }
}
