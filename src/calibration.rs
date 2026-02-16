use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::GameAction;
use crate::config::GameSettings;
use crate::state::GameScreen;

pub struct CalibrationPlugin;

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::Calibration), setup_calibration)
            .add_systems(
                Update,
                (tick_calibration, handle_calibration_input, update_calibration_ui)
                    .chain()
                    .run_if(in_state(GameScreen::Calibration)),
            );
    }
}

// --- Y2K Future Punk palette ---

const BG_DARK: Color = Color::srgba(0.02, 0.01, 0.06, 0.95);
const PANEL_BG: Color = Color::srgba(0.06, 0.03, 0.12, 0.92);
const PANEL_BORDER: Color = Color::srgb(0.6, 0.2, 1.0);
const TITLE_COLOR: Color = Color::srgb(0.92, 0.96, 1.0);
const HINT_COLOR: Color = Color::srgb(0.4, 0.35, 0.5);
const HEADER_COLOR: Color = Color::srgb(0.6, 0.2, 1.0);
const ACCENT_CYAN: Color = Color::srgb(0.0, 0.9, 1.0);
const FLASH_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
const BEAT_MARKER_COLOR: Color = Color::srgb(0.0, 0.9, 1.0);
const GREAT_COLOR: Color = Color::srgb(0.0, 1.0, 0.4);
const MISS_COLOR: Color = Color::srgb(1.0, 0.15, 0.3);

const HEADER_FONT: f32 = 36.0;
const INFO_FONT: f32 = 18.0;
const VALUE_FONT: f32 = 28.0;
const HINT_FONT: f32 = 12.0;

/// BPM for calibration metronome.
const CAL_BPM: f64 = 120.0;
const BEAT_INTERVAL: f64 = 60.0 / CAL_BPM;
/// Number of taps to collect before computing offset.
const TAPS_NEEDED: usize = 16;
/// Flash duration in seconds.
const FLASH_DURATION: f32 = 0.08;

// --- Resources ---

#[derive(Resource)]
struct CalibrationState {
    phase: CalPhase,
    elapsed: f64,
    beat_count: u32,
    tap_offsets: Vec<f64>,
    flash_timer: f32,
    computed_offset: Option<f64>,
    dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalPhase {
    Instructions,
    Tapping,
    Results,
}

// --- Markers ---

#[derive(Component)]
struct FlashIndicator;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct OffsetText;

#[derive(Component)]
struct ProgressText;

// --- Setup ---

fn setup_calibration(mut commands: Commands) {
    commands.insert_resource(CalibrationState {
        phase: CalPhase::Instructions,
        elapsed: 0.0,
        beat_count: 0,
        tap_offsets: Vec::with_capacity(TAPS_NEEDED),
        flash_timer: 0.0,
        computed_offset: None,
        dirty: true,
    });

    commands
        .spawn((
            DespawnOnExit(GameScreen::Calibration),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(24.0),
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(BG_DARK),
        ))
        .with_children(|root| {
            // Header
            root.spawn((
                Text::new("CALIBRATION"),
                TextFont { font_size: HEADER_FONT, ..default() },
                TextColor(HEADER_COLOR),
                Node { margin: UiRect::bottom(Val::Px(8.0)), ..default() },
            ));

            // Panel
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    min_width: Val::Px(480.0),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(PANEL_BORDER),
            ))
            .with_children(|panel| {
                // Flash / beat indicator
                panel.spawn((
                    FlashIndicator,
                    Node {
                        width: Val::Px(80.0),
                        height: Val::Px(80.0),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(40.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.05, 0.2, 0.5)),
                    BorderColor::all(BEAT_MARKER_COLOR.with_alpha(0.3)),
                ));

                // Status text
                panel.spawn((
                    StatusText,
                    Text::new("Tap along with the flashing beat.\nPress SPACE to start."),
                    TextFont { font_size: INFO_FONT, ..default() },
                    TextColor(TITLE_COLOR),
                    TextLayout::new_with_justify(Justify::Center),
                ));

                // Progress
                panel.spawn((
                    ProgressText,
                    Text::new(""),
                    TextFont { font_size: HINT_FONT, ..default() },
                    TextColor(HINT_COLOR),
                ));

                // Offset result
                panel.spawn((
                    OffsetText,
                    Text::new(""),
                    TextFont { font_size: VALUE_FONT, ..default() },
                    TextColor(ACCENT_CYAN),
                ));
            });

            // Hints
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(24.0),
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            })
            .with_children(|hints| {
                spawn_hint(hints, "SPACE", "tap / start");
                spawn_hint(hints, "CONFIRM", "apply offset");
                spawn_hint(hints, "ESC", "back");
            });
        });
}

fn spawn_hint(parent: &mut ChildSpawnerCommands, key: &str, action: &str) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|hint| {
            hint.spawn((
                Text::new(format!("[{key}]")),
                TextFont { font_size: HINT_FONT, ..default() },
                TextColor(ACCENT_CYAN.with_alpha(0.6)),
            ));
            hint.spawn((
                Text::new(action.to_string()),
                TextFont { font_size: HINT_FONT, ..default() },
                TextColor(HINT_COLOR),
            ));
        });
}

// --- Systems ---

fn tick_calibration(
    time: Res<Time>,
    mut state: ResMut<CalibrationState>,
) {
    if state.phase != CalPhase::Tapping {
        return;
    }

    state.elapsed += time.delta_secs_f64();

    // Check for new beat
    let expected_beats = (state.elapsed / BEAT_INTERVAL) as u32;
    if expected_beats > state.beat_count {
        state.beat_count = expected_beats;
        state.flash_timer = FLASH_DURATION;
        state.dirty = true;
    }

    // Tick flash
    if state.flash_timer > 0.0 {
        state.flash_timer -= time.delta_secs();
        if state.flash_timer <= 0.0 {
            state.flash_timer = 0.0;
            state.dirty = true;
        }
    }
}

fn handle_calibration_input(
    action: Res<ActionState<GameAction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CalibrationState>,
    mut next_state: ResMut<NextState<GameScreen>>,
    mut settings: ResMut<GameSettings>,
) {
    if action.just_pressed(&GameAction::Back) {
        next_state.set(GameScreen::Settings);
        return;
    }

    match state.phase {
        CalPhase::Instructions => {
            if keys.just_pressed(KeyCode::Space) || action.just_pressed(&GameAction::Confirm) {
                state.phase = CalPhase::Tapping;
                state.elapsed = 0.0;
                state.beat_count = 0;
                state.tap_offsets.clear();
                state.dirty = true;
            }
        }
        CalPhase::Tapping => {
            if keys.just_pressed(KeyCode::Space) {
                // Ignore taps before the first beat
                if state.beat_count == 0 {
                    return;
                }

                // Compute offset from nearest beat
                let nearest_beat = (state.elapsed / BEAT_INTERVAL).round();
                let nearest_beat_time = nearest_beat * BEAT_INTERVAL;
                let offset_ms = (state.elapsed - nearest_beat_time) * 1000.0;

                // Only accept taps within a reasonable window (±200ms)
                if offset_ms.abs() <= 200.0 {
                    state.tap_offsets.push(offset_ms);
                    state.dirty = true;

                    if state.tap_offsets.len() >= TAPS_NEEDED {
                        // Compute median offset
                        let mut sorted = state.tap_offsets.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let median = if sorted.len() % 2 == 0 {
                            let mid = sorted.len() / 2;
                            (sorted[mid - 1] + sorted[mid]) / 2.0
                        } else {
                            sorted[sorted.len() / 2]
                        };
                        state.computed_offset = Some(median);
                        state.phase = CalPhase::Results;
                        state.dirty = true;
                    }
                }
            }
        }
        CalPhase::Results => {
            if action.just_pressed(&GameAction::Confirm) {
                // Apply offset
                if let Some(offset) = state.computed_offset {
                    settings.audio_offset_ms = offset.round() as i32;
                    settings.save();
                    info!("Applied audio offset: {} ms", settings.audio_offset_ms);
                }
                next_state.set(GameScreen::Settings);
            }
            if keys.just_pressed(KeyCode::Space) {
                // Retry
                state.phase = CalPhase::Instructions;
                state.tap_offsets.clear();
                state.computed_offset = None;
                state.dirty = true;
            }
        }
    }
}

fn update_calibration_ui(
    mut state: ResMut<CalibrationState>,
    mut flash: Query<(&mut BackgroundColor, &mut BorderColor), With<FlashIndicator>>,
    mut status: Query<&mut Text, (With<StatusText>, Without<ProgressText>, Without<OffsetText>)>,
    mut progress: Query<&mut Text, (With<ProgressText>, Without<StatusText>, Without<OffsetText>)>,
    mut offset_text: Query<&mut Text, (With<OffsetText>, Without<StatusText>, Without<ProgressText>)>,
) {
    // Always update flash (smooth animation)
    for (mut bg, mut border) in &mut flash {
        if state.flash_timer > 0.0 {
            let intensity = state.flash_timer / FLASH_DURATION;
            *bg = BackgroundColor(FLASH_COLOR.with_alpha(intensity * 0.9));
            *border = BorderColor::all(BEAT_MARKER_COLOR);
        } else if state.phase == CalPhase::Tapping {
            *bg = BackgroundColor(Color::srgba(0.1, 0.05, 0.2, 0.5));
            *border = BorderColor::all(BEAT_MARKER_COLOR.with_alpha(0.3));
        }
    }

    if !state.dirty {
        return;
    }
    state.dirty = false;

    // Update status text
    for mut text in &mut status {
        **text = match state.phase {
            CalPhase::Instructions => {
                "Tap along with the flashing beat.\nPress SPACE to start.".to_string()
            }
            CalPhase::Tapping => {
                "Tap SPACE on each flash...".to_string()
            }
            CalPhase::Results => {
                if let Some(offset) = state.computed_offset {
                    let quality = if offset.abs() < 10.0 {
                        "Excellent timing!"
                    } else if offset.abs() < 30.0 {
                        "Good timing."
                    } else {
                        "Offset detected."
                    };
                    format!(
                        "{quality}\nPress CONFIRM to apply, SPACE to retry."
                    )
                } else {
                    "Error computing offset.".to_string()
                }
            }
        };
    }

    // Update progress
    for mut text in &mut progress {
        match state.phase {
            CalPhase::Tapping => {
                let count = state.tap_offsets.len();
                **text = format!("{count} / {TAPS_NEEDED} taps");
            }
            _ => {
                **text = String::new();
            }
        }
    }

    // Update offset display
    for mut text in &mut offset_text {
        if let Some(offset) = state.computed_offset {
            let sign = if offset >= 0.0 { "+" } else { "" };
            **text = format!("{sign}{:.0} ms", offset);
        } else {
            **text = String::new();
        }
    }

    // Update flash indicator for non-tapping phases
    for (mut bg, mut border) in &mut flash {
        match state.phase {
            CalPhase::Instructions => {
                *bg = BackgroundColor(Color::srgba(0.1, 0.05, 0.2, 0.5));
                *border = BorderColor::all(BEAT_MARKER_COLOR.with_alpha(0.3));
            }
            CalPhase::Results => {
                let color = if state
                    .computed_offset
                    .map_or(false, |o| o.abs() < 30.0)
                {
                    GREAT_COLOR
                } else {
                    MISS_COLOR
                };
                *bg = BackgroundColor(color.with_alpha(0.3));
                *border = BorderColor::all(color);
            }
            CalPhase::Tapping => {} // Handled above
        }
    }
}
