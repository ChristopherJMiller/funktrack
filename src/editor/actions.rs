use crate::beatmap::{ChartEvent, ChartFile, ChartNoteEntry, ChartTimingPoint};

/// A reversible editor action for undo/redo.
#[derive(Debug, Clone)]
pub enum EditorAction {
    AddNote {
        note: ChartNoteEntry,
    },
    RemoveNote {
        index: usize,
        note: ChartNoteEntry,
    },
    ModifyNote {
        index: usize,
        old: ChartNoteEntry,
        new: ChartNoteEntry,
    },
    AddPathPoint {
        segment: usize,
        point: (f32, f32),
    },
    RemovePathPoint {
        segment: usize,
        index: usize,
        point: (f32, f32),
    },
    MovePathPoint {
        segment: usize,
        index: usize,
        old_pos: (f32, f32),
        new_pos: (f32, f32),
    },
    AddEvent {
        event: ChartEvent,
    },
    RemoveEvent {
        index: usize,
        event: ChartEvent,
    },
    ModifyTimingPoint {
        index: usize,
        old: ChartTimingPoint,
        new: ChartTimingPoint,
    },
}

impl EditorAction {
    pub fn apply(&self, chart: &mut ChartFile) {
        match self {
            EditorAction::AddNote { note } => {
                let pos = chart
                    .notes
                    .partition_point(|n| n.beat < note.beat);
                chart.notes.insert(pos, note.clone());
            }
            EditorAction::RemoveNote { index, .. } => {
                if *index < chart.notes.len() {
                    chart.notes.remove(*index);
                }
            }
            EditorAction::ModifyNote { index, new, .. } => {
                if *index < chart.notes.len() {
                    chart.notes[*index] = new.clone();
                }
            }
            EditorAction::AddPathPoint { segment, point } => {
                if let Some(seg) = chart.path_segments.get_mut(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        points.push(*point);
                    }
                }
            }
            EditorAction::RemovePathPoint {
                segment, index, ..
            } => {
                if let Some(seg) = chart.path_segments.get_mut(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        if *index < points.len() {
                            points.remove(*index);
                        }
                    }
                }
            }
            EditorAction::MovePathPoint {
                segment,
                index,
                new_pos,
                ..
            } => {
                if let Some(seg) = chart.path_segments.get_mut(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        if *index < points.len() {
                            points[*index] = *new_pos;
                        }
                    }
                }
            }
            EditorAction::AddEvent { event } => {
                let pos = chart
                    .events
                    .partition_point(|e| e.beat < event.beat);
                chart.events.insert(pos, event.clone());
            }
            EditorAction::RemoveEvent { index, .. } => {
                if *index < chart.events.len() {
                    chart.events.remove(*index);
                }
            }
            EditorAction::ModifyTimingPoint { index, new, .. } => {
                if *index < chart.timing_points.len() {
                    chart.timing_points[*index] = new.clone();
                }
            }
        }
    }

    pub fn undo(&self, chart: &mut ChartFile) {
        match self {
            EditorAction::AddNote { note } => {
                // Remove the note we added — find it by beat + type
                if let Some(pos) = chart.notes.iter().position(|n| {
                    (n.beat - note.beat).abs() < 1e-6
                }) {
                    chart.notes.remove(pos);
                }
            }
            EditorAction::RemoveNote { index, note } => {
                chart.notes.insert(*index, note.clone());
            }
            EditorAction::ModifyNote { index, old, .. } => {
                if *index < chart.notes.len() {
                    chart.notes[*index] = old.clone();
                }
            }
            EditorAction::AddPathPoint { segment, .. } => {
                if let Some(seg) = chart.path_segments.get_mut(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        points.pop();
                    }
                }
            }
            EditorAction::RemovePathPoint {
                segment,
                index,
                point,
            } => {
                if let Some(seg) = chart.path_segments.get_mut(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        points.insert(*index, *point);
                    }
                }
            }
            EditorAction::MovePathPoint {
                segment,
                index,
                old_pos,
                ..
            } => {
                if let Some(seg) = chart.path_segments.get_mut(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        if *index < points.len() {
                            points[*index] = *old_pos;
                        }
                    }
                }
            }
            EditorAction::AddEvent { event } => {
                if let Some(pos) = chart.events.iter().position(|e| {
                    (e.beat - event.beat).abs() < 1e-6
                }) {
                    chart.events.remove(pos);
                }
            }
            EditorAction::RemoveEvent { index, event } => {
                chart.events.insert(*index, event.clone());
            }
            EditorAction::ModifyTimingPoint { index, old, .. } => {
                if *index < chart.timing_points.len() {
                    chart.timing_points[*index] = old.clone();
                }
            }
        }
    }
}
