use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::beatmap::ChartNoteType;

use super::io::{export_chart_json, save_chart_ron};
use super::{EditorElement, EditorMode, EditorState, GridSnap, NoteBrush, PlaybackState};

// ─── Y2K Color Palette ──────────────────────────────────────────────
const NEON_PURPLE: egui::Color32 = egui::Color32::from_rgb(153, 51, 255);
const ELECTRIC_CYAN: egui::Color32 = egui::Color32::from_rgb(0, 230, 255);
const NEON_GREEN: egui::Color32 = egui::Color32::from_rgb(0, 255, 100);
const BRIGHT_TEXT: egui::Color32 = egui::Color32::from_rgb(235, 245, 255);
const DIM_TEXT: egui::Color32 = egui::Color32::from_rgb(100, 90, 130);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(15, 8, 30, 240);
const DEEP_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(5, 3, 15, 220);
const GRID_MAJOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 60, 120, 100);
const GRID_MINOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(40, 30, 60, 60);

/// Main egui rendering system for the editor.
pub fn editor_ui_system(mut contexts: EguiContexts, mut state: ResMut<EditorState>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    apply_y2k_theme(ctx);

    menu_bar(ctx, &mut state);

    match state.mode {
        EditorMode::Chart => chart_mode_ui(ctx, &mut state),
        EditorMode::Path => path_mode_ui(ctx, &mut state),
    }

    toast_overlay(ctx, &state);

    // Tell input_system whether egui owns the pointer
    state.egui_wants_pointer = ctx.wants_pointer_input();
}

// ─── Theme ──────────────────────────────────────────────────────────

fn apply_y2k_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = PANEL_BG;
    v.window_fill = PANEL_BG;
    v.extreme_bg_color = egui::Color32::from_rgb(5, 3, 15);
    v.faint_bg_color = egui::Color32::from_rgba_premultiplied(30, 15, 55, 200);
    v.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 230, 255, 80);
    v.selection.stroke = egui::Stroke::new(1.0, ELECTRIC_CYAN);
    v.widgets.active.bg_fill = egui::Color32::from_rgba_premultiplied(30, 10, 56, 240);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, ELECTRIC_CYAN);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(40, 15, 70, 230);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, NEON_PURPLE);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(20, 10, 40, 200);
    v.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(115, 90, 140));
    v.widgets.noninteractive.bg_fill =
        egui::Color32::from_rgba_premultiplied(15, 8, 30, 200);
    v.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 140, 170));
    ctx.set_style(style);
}

// ─── Menu Bar ───────────────────────────────────────────────────────

fn menu_bar(ctx: &egui::Context, state: &mut EditorState) {
    egui::TopBottomPanel::top("editor_menu_bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save (Ctrl+S)").clicked() {
                    let path = state.song_dir.join(state.chart.difficulty.filename());
                    match save_chart_ron(&state.chart, &path) {
                        Ok(()) => state.unsaved_changes = false,
                        Err(e) => error!("Save failed: {e}"),
                    }
                    ui.close();
                }
                if ui.button("Export JSON...").clicked() {
                    let path = state
                        .song_dir
                        .join(format!("{}.json", state.chart.difficulty.filename()));
                    if let Err(e) = export_chart_json(&state.chart, &path) {
                        error!("Export failed: {e}");
                    }
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(
                        !state.undo_stack.is_empty(),
                        egui::Button::new("Undo (Ctrl+Z)"),
                    )
                    .clicked()
                {
                    state.undo();
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !state.redo_stack.is_empty(),
                        egui::Button::new("Redo (Ctrl+Y)"),
                    )
                    .clicked()
                {
                    state.redo();
                    ui.close();
                }
            });

            // Mode switch
            ui.separator();
            let chart_label = if state.mode == EditorMode::Chart {
                egui::RichText::new("CHART").color(ELECTRIC_CYAN).strong()
            } else {
                egui::RichText::new("CHART").color(DIM_TEXT)
            };
            if ui.selectable_label(state.mode == EditorMode::Chart, chart_label).clicked() {
                state.mode = EditorMode::Chart;
            }
            let path_label = if state.mode == EditorMode::Path {
                egui::RichText::new("PATH").color(ELECTRIC_CYAN).strong()
            } else {
                egui::RichText::new("PATH").color(DIM_TEXT)
            };
            if ui.selectable_label(state.mode == EditorMode::Path, path_label).clicked() {
                state.mode = EditorMode::Path;
            }
            ui.label(
                egui::RichText::new("(Tab)")
                    .color(DIM_TEXT)
                    .size(10.0),
            );

            // Right-aligned title
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let title = format!(
                    "{} [{}]{}",
                    state.metadata.title,
                    state.chart.difficulty.label(),
                    if state.unsaved_changes { " *" } else { "" }
                );
                ui.label(egui::RichText::new(title).color(BRIGHT_TEXT).size(13.0));
            });
        });
    });
}

// ─── Chart Mode: timeline-dominant ──────────────────────────────────

fn chart_mode_ui(ctx: &egui::Context, state: &mut EditorState) {
    // Right panel: brush selector + metadata
    egui::SidePanel::right("chart_right_panel")
        .default_width(180.0)
        .min_width(160.0)
        .show(ctx, |ui| {
            brush_panel(ui, state);
            ui.add_space(12.0);
            metadata_panel(ui, state);
            ui.add_space(12.0);
            selection_panel(ui, state);
        });

    // Bottom: status bar
    status_bar(ctx, state);

    // Central area: the big timeline
    egui::CentralPanel::default().show(ctx, |ui| {
        timeline_view(ui, state);
    });
}

// ─── Path Mode: viewport-dominant ───────────────────────────────────

fn path_mode_ui(ctx: &egui::Context, state: &mut EditorState) {
    // Thin bottom timeline bar
    egui::TopBottomPanel::bottom("path_timeline_bar")
        .exact_height(50.0)
        .show(ctx, |ui| {
            compact_timeline(ui, state);
        });

    // Right panel: path properties
    egui::SidePanel::right("path_right_panel")
        .default_width(180.0)
        .min_width(160.0)
        .show(ctx, |ui| {
            section_heading(ui, "PATH TOOLS");
            ui.label(
                egui::RichText::new("Ctrl+Click to add points")
                    .color(DIM_TEXT)
                    .size(11.0),
            );
            ui.label(
                egui::RichText::new("Click+drag to move points")
                    .color(DIM_TEXT)
                    .size(11.0),
            );
            ui.label(
                egui::RichText::new("Delete to remove selected")
                    .color(DIM_TEXT)
                    .size(11.0),
            );

            let n_points: usize = state
                .chart
                .path_segments
                .iter()
                .map(|s| match s {
                    crate::beatmap::PathSegment::CatmullRom { points, .. } => points.len(),
                    _ => 0,
                })
                .sum();
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!("{} control points", n_points))
                    .color(NEON_GREEN)
                    .size(12.0),
            );
            if n_points < 4 {
                ui.label(
                    egui::RichText::new("Need at least 4 for spline")
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .size(11.0),
                );
            }

            ui.add_space(12.0);
            selection_panel(ui, state);
            ui.add_space(12.0);
            chart_settings_panel(ui, state);
        });

    // The central panel is the game viewport (rendered by lyon, not egui)
    // Just leave it empty so the Bevy viewport shows through.
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |_| {});
}

// ─── Shared Panels ──────────────────────────────────────────────────

fn brush_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    section_heading(ui, "NOTE BRUSH");
    let brushes: &[(&str, &str, NoteBrush)] = &[
        ("1", "TAP", NoteBrush::Tap),
        ("2", "HOLD", NoteBrush::Hold { duration_beats: 1.0 }),
        (
            "3",
            "SLIDE",
            NoteBrush::Slide {
                direction: crate::beatmap::SlideDirection::E,
            },
        ),
        ("4", "CRITICAL", NoteBrush::Critical),
        ("5", "REST", NoteBrush::Rest),
    ];

    for &(key, label, ref brush) in brushes {
        let is_active = std::mem::discriminant(&state.note_brush) == std::mem::discriminant(brush);
        let text = format!("[{}] {}", key, label);
        let rich = if is_active {
            egui::RichText::new(&text).color(ELECTRIC_CYAN).strong().size(12.0)
        } else {
            egui::RichText::new(&text).size(12.0)
        };
        if ui.selectable_label(is_active, rich).clicked() {
            state.note_brush = brush.clone();
        }
    }

    ui.add_space(8.0);
    section_heading(ui, "GRID SNAP");
    let snaps = [
        GridSnap::None,
        GridSnap::Whole,
        GridSnap::Half,
        GridSnap::Quarter,
        GridSnap::Eighth,
        GridSnap::Sixteenth,
    ];
    ui.horizontal_wrapped(|ui| {
        for snap in snaps {
            let is_active = state.grid_snap == snap;
            let rich = if is_active {
                egui::RichText::new(snap.label()).color(NEON_GREEN).strong().size(11.0)
            } else {
                egui::RichText::new(snap.label()).size(11.0)
            };
            if ui.selectable_label(is_active, rich).clicked() {
                state.grid_snap = snap;
            }
        }
    });
    ui.label(
        egui::RichText::new("[ / ] to cycle")
            .color(DIM_TEXT)
            .size(10.0),
    );
}

fn metadata_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    section_heading(ui, "METADATA");
    egui::Grid::new("metadata_grid")
        .num_columns(2)
        .spacing([6.0, 3.0])
        .show(ui, |ui| {
            ui.label("Title:");
            ui.text_edit_singleline(&mut state.metadata.title);
            ui.end_row();
            ui.label("Artist:");
            ui.text_edit_singleline(&mut state.metadata.artist);
            ui.end_row();
            ui.label("Charter:");
            ui.text_edit_singleline(&mut state.metadata.charter);
            ui.end_row();

            ui.label("BPM:");
            ui.label(egui::RichText::new(format!("{:.1}", state.bpm())).color(NEON_GREEN));
            ui.end_row();
            ui.label("Notes:");
            ui.label(
                egui::RichText::new(format!("{}", state.chart.notes.len())).color(ELECTRIC_CYAN),
            );
            ui.end_row();
        });
}

fn selection_panel(ui: &mut egui::Ui, state: &EditorState) {
    section_heading(ui, "SELECTION");
    if state.selected.is_empty() {
        ui.label(egui::RichText::new("Nothing selected").color(DIM_TEXT).italics());
        return;
    }
    for element in &state.selected {
        match element {
            EditorElement::Note { index } => {
                if let Some(note) = state.chart.notes.get(*index) {
                    ui.label(
                        egui::RichText::new(format!(
                            "Note #{} — beat {:.3}",
                            index, note.beat
                        ))
                        .color(BRIGHT_TEXT)
                        .size(11.0),
                    );
                    ui.label(
                        egui::RichText::new(format!("  {:?}", note.note_type))
                            .color(DIM_TEXT)
                            .size(10.0),
                    );
                }
            }
            EditorElement::PathControlPoint { segment, index } => {
                if let Some(seg) = state.chart.path_segments.get(*segment) {
                    if let crate::beatmap::PathSegment::CatmullRom { points, .. } = seg {
                        if let Some(&(x, y)) = points.get(*index) {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Point [{},{}] ({:.0}, {:.0})",
                                    segment, index, x, y
                                ))
                                .color(BRIGHT_TEXT)
                                .size(11.0),
                            );
                        }
                    }
                }
            }
            EditorElement::Event { index } => {
                if let Some(event) = state.chart.events.get(*index) {
                    ui.label(
                        egui::RichText::new(format!(
                            "Event #{} — beat {:.3}",
                            index, event.beat
                        ))
                        .color(BRIGHT_TEXT)
                        .size(11.0),
                    );
                }
            }
        }
    }
}

fn chart_settings_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    section_heading(ui, "CHART SETTINGS");
    egui::Grid::new("chart_settings_grid")
        .num_columns(2)
        .spacing([6.0, 3.0])
        .show(ui, |ui| {
            ui.label("Travel beats:");
            let mut travel = state.chart.travel_beats;
            if ui
                .add(
                    egui::DragValue::new(&mut travel)
                        .range(0.5..=10.0)
                        .speed(0.1),
                )
                .changed()
            {
                state.chart.travel_beats = travel;
                state.unsaved_changes = true;
            }
            ui.end_row();

            ui.label("Look-ahead:");
            let mut look = state.chart.look_ahead_beats;
            if ui
                .add(
                    egui::DragValue::new(&mut look)
                        .range(0.5..=10.0)
                        .speed(0.1),
                )
                .changed()
            {
                state.chart.look_ahead_beats = look;
                state.unsaved_changes = true;
            }
            ui.end_row();
        });
}

fn status_bar(ctx: &egui::Context, state: &EditorState) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(22.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let play_icon = match state.playback {
                    PlaybackState::Stopped => "||",
                    PlaybackState::Playing => ">>",
                };
                ui.label(
                    egui::RichText::new(play_icon)
                        .color(if state.playback == PlaybackState::Playing {
                            NEON_GREEN
                        } else {
                            DIM_TEXT
                        })
                        .monospace()
                        .size(11.0),
                );
                ui.label(
                    egui::RichText::new(format!("Beat {:.2}", state.cursor_beat))
                        .color(ELECTRIC_CYAN)
                        .monospace()
                        .size(11.0),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Grid: {}", state.grid_snap.label()))
                        .color(NEON_GREEN)
                        .size(11.0),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Brush: {}", state.note_brush.label()))
                        .color(NEON_PURPLE)
                        .size(11.0),
                );
                ui.separator();
                let mode_label = match state.mode {
                    EditorMode::Chart => "CHART MODE",
                    EditorMode::Path => "PATH MODE",
                };
                ui.label(egui::RichText::new(mode_label).color(BRIGHT_TEXT).size(11.0));

                // Enter hint in Chart mode
                if state.mode == EditorMode::Chart {
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new("Enter = place note | Space = play/pause")
                                    .color(DIM_TEXT)
                                    .size(10.0),
                            );
                        },
                    );
                }
            });
        });
}

// ─── Timeline Views ─────────────────────────────────────────────────

/// The main timeline view that dominates Chart mode.
fn timeline_view(ui: &mut egui::Ui, state: &mut EditorState) {
    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 0.0, DEEP_BG);

    // Compute visible beat range
    let view_beats = state.timeline_view_beats;
    let center_beat = state.cursor_beat;
    let start_beat = (center_beat - view_beats / 2.0).max(0.0);
    let end_beat = start_beat + view_beats;

    let beat_to_x = |beat: f64| -> f32 {
        rect.left() + ((beat - start_beat) / view_beats * rect.width() as f64) as f32
    };

    // Grid lines
    let grid_div = state.grid_snap.divisor().max(1.0);
    let first_grid = (start_beat * grid_div).ceil() / grid_div;
    let mut grid_beat = first_grid;
    while grid_beat <= end_beat {
        let x = beat_to_x(grid_beat);
        let is_whole = (grid_beat - grid_beat.round()).abs() < 0.01;
        let color = if is_whole { GRID_MAJOR } else { GRID_MINOR };
        painter.line_segment(
            [egui::Pos2::new(x, rect.top()), egui::Pos2::new(x, rect.bottom())],
            egui::Stroke::new(if is_whole { 1.5 } else { 0.5 }, color),
        );
        if is_whole {
            painter.text(
                egui::Pos2::new(x + 3.0, rect.top() + 4.0),
                egui::Align2::LEFT_TOP,
                format!("{}", grid_beat as i32),
                egui::FontId::monospace(11.0),
                GRID_MAJOR,
            );
        }
        grid_beat += 1.0 / grid_div;
    }

    // Note lanes — notes are drawn as colored rectangles on horizontal "rows"
    // Each note type gets a row for easy visual grouping
    let lane_height = (rect.height() / 7.0).min(40.0).max(16.0);
    let lane_start_y = rect.top() + 24.0; // Below beat numbers

    for (i, note) in state.chart.notes.iter().enumerate() {
        if note.beat < start_beat - 1.0 || note.beat > end_beat + 1.0 {
            continue;
        }
        let x = beat_to_x(note.beat);
        let (color, lane) = note_visual_info(&note.note_type);
        let y = lane_start_y + lane as f32 * lane_height;
        let is_selected = state.selected.contains(&EditorElement::Note { index: i });

        // Note marker: small colored rectangle
        let note_width = 6.0;
        let note_height = lane_height * 0.7;
        let note_rect = egui::Rect::from_center_size(
            egui::Pos2::new(x, y + lane_height * 0.5),
            egui::Vec2::new(note_width, note_height),
        );
        painter.rect_filled(note_rect, 2.0, color);

        // Hold/SlideHold duration bar
        if let Some(dur) = note_duration(&note.note_type) {
            let end_x = beat_to_x(note.beat + dur);
            let bar_rect = egui::Rect::from_min_max(
                egui::Pos2::new(x, y + lane_height * 0.35),
                egui::Pos2::new(end_x, y + lane_height * 0.65),
            );
            painter.rect_filled(
                bar_rect,
                1.0,
                color.gamma_multiply(0.4),
            );
        }

        if is_selected {
            let sel_rect = note_rect.expand(3.0);
            painter.rect_stroke(sel_rect, 2.0, egui::Stroke::new(1.5, egui::Color32::WHITE), egui::StrokeKind::Outside);
        }
    }

    // Lane labels on the left edge
    let lane_labels = [
        "TAP", "HOLD", "SLIDE", "CRIT", "REST",
    ];
    for (lane, label) in lane_labels.iter().enumerate() {
        let y = lane_start_y + lane as f32 * lane_height + lane_height * 0.5;
        painter.text(
            egui::Pos2::new(rect.left() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            *label,
            egui::FontId::monospace(9.0),
            egui::Color32::from_rgba_premultiplied(80, 60, 120, 120),
        );
    }

    // Cursor line (playhead)
    let cursor_x = beat_to_x(state.cursor_beat);
    painter.line_segment(
        [
            egui::Pos2::new(cursor_x, rect.top()),
            egui::Pos2::new(cursor_x, rect.bottom()),
        ],
        egui::Stroke::new(2.0, ELECTRIC_CYAN),
    );
    // Cursor beat label
    painter.text(
        egui::Pos2::new(cursor_x + 4.0, rect.bottom() - 14.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{:.2}", state.cursor_beat),
        egui::FontId::monospace(10.0),
        ELECTRIC_CYAN,
    );

    // Click to seek
    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frac = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
            let beat = start_beat + frac * view_beats;
            state.cursor_beat = state.grid_snap.snap_beat(beat);
        }
    }

    // Click on a note to select it
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let click_beat = start_beat
                + ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * view_beats;
            // Find nearest note within ~0.3 beats
            let mut best = None;
            let mut best_dist = 0.3;
            for (i, note) in state.chart.notes.iter().enumerate() {
                let d = (note.beat - click_beat).abs();
                if d < best_dist {
                    best_dist = d;
                    best = Some(i);
                }
            }
            if let Some(idx) = best {
                state.selected.clear();
                state.selected.insert(EditorElement::Note { index: idx });
            }
        }
    }

    // Scroll wheel to zoom timeline
    let scroll = ui.input(|i| i.raw_scroll_delta.y);
    if scroll.abs() > 0.1 {
        let zoom_factor = if scroll > 0.0 { 0.85 } else { 1.18 };
        state.timeline_view_beats = (state.timeline_view_beats * zoom_factor).clamp(4.0, 128.0);
    }
}

/// Compact horizontal timeline for Path mode — just shows cursor position.
fn compact_timeline(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        let play_text = match state.playback {
            PlaybackState::Stopped => "PLAY",
            PlaybackState::Playing => "STOP",
        };
        if ui
            .button(egui::RichText::new(play_text).size(12.0))
            .clicked()
        {
            state.playback = match state.playback {
                PlaybackState::Stopped => PlaybackState::Playing,
                PlaybackState::Playing => PlaybackState::Stopped,
            };
        }
        ui.label(
            egui::RichText::new(format!("Beat: {:.2}", state.cursor_beat))
                .color(ELECTRIC_CYAN)
                .monospace()
                .size(12.0),
        );

        // Slider for scrubbing
        let total = state.total_beats;
        let mut beat = state.cursor_beat;
        ui.add(
            egui::Slider::new(&mut beat, 0.0..=total)
                .text("")
                .show_value(false),
        );
        state.cursor_beat = beat;
    });
}

// ─── Toast Overlay ──────────────────────────────────────────────────

fn toast_overlay(ctx: &egui::Context, state: &EditorState) {
    if let Some((ref msg, _)) = state.toast {
        egui::Area::new(egui::Id::new("editor_toast"))
            .fixed_pos(egui::Pos2::new(
                ctx.content_rect().center().x,
                ctx.content_rect().top() + 40.0,
            ))
            .pivot(egui::Align2::CENTER_TOP)
            .show(ctx, |ui| {
                egui::Frame::popup(&ctx.style())
                    .fill(egui::Color32::from_rgba_premultiplied(0, 180, 0, 200))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(msg)
                                .color(egui::Color32::WHITE)
                                .strong()
                                .size(14.0),
                        );
                    });
            });
    }
}

// ─── Helpers ────────────────────────────────────────────────────────

fn section_heading(ui: &mut egui::Ui, label: &str) {
    ui.heading(egui::RichText::new(label).color(NEON_PURPLE).size(14.0));
    ui.separator();
}

/// Returns (color, lane_index) for a note type.
fn note_visual_info(note_type: &ChartNoteType) -> (egui::Color32, usize) {
    match note_type {
        ChartNoteType::Tap => (egui::Color32::from_rgb(255, 102, 178), 0),
        ChartNoteType::Hold { .. } => (egui::Color32::from_rgb(255, 217, 38), 1),
        ChartNoteType::Slide { .. } | ChartNoteType::SlideHold { .. } => {
            (egui::Color32::from_rgb(0, 230, 255), 2)
        }
        ChartNoteType::Critical | ChartNoteType::CriticalHold { .. } => {
            (egui::Color32::from_rgb(255, 242, 204), 3)
        }
        ChartNoteType::Rest => {
            (egui::Color32::from_rgba_premultiplied(230, 230, 255, 120), 4)
        }
        // Deprecated types show as grey on the last lane
        ChartNoteType::Scratch | ChartNoteType::Beat | ChartNoteType::DualSlide { .. } => {
            (egui::Color32::from_rgb(100, 100, 100), 4)
        }
    }
}

/// Returns duration in beats for note types that have one.
fn note_duration(note_type: &ChartNoteType) -> Option<f64> {
    match note_type {
        ChartNoteType::Hold { duration_beats } => Some(*duration_beats),
        ChartNoteType::SlideHold { duration_beats, .. } => Some(*duration_beats),
        ChartNoteType::CriticalHold { duration_beats } => Some(*duration_beats),
        _ => None,
    }
}
