use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::GameSet;
use crate::action::GameAction;
use crate::audio::{KiraContext, stop_song};
use crate::conductor::SongConductor;
use crate::beatmap::SelectedSong;
use crate::judgment::JudgmentFeedback;
use crate::notes::{NoteAlive, NoteQueue};
use crate::path::SplinePath;
use crate::scoring::{GradeRank, ScoreState};
use crate::state::GameScreen;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            check_song_end.in_set(GameSet::Render),
        )
        .add_systems(OnEnter(GameScreen::Results), spawn_results_overlay)
        .add_systems(
            Update,
            dismiss_results.run_if(in_state(GameScreen::Results)),
        )
        .add_systems(OnExit(GameScreen::Results), cleanup_gameplay);
    }
}

// --- Y2K Future Punk results palette ---

/// Backdrop — dark indigo wash over gameplay
const BACKDROP: Color = Color::srgba(0.02, 0.01, 0.06, 0.88);
/// Panel background — elevated dark surface
const PANEL_BG: Color = Color::srgba(0.06, 0.03, 0.12, 0.95);
/// Panel border — neon purple accent
const PANEL_BORDER: Color = Color::srgb(0.6, 0.2, 1.0);
/// Section divider
const DIVIDER: Color = Color::srgba(0.6, 0.2, 1.0, 0.3);

/// Primary text — bright near-white
const TEXT_PRIMARY: Color = Color::srgb(0.92, 0.96, 1.0);
/// Label text — muted lavender
const TEXT_LABEL: Color = Color::srgb(0.55, 0.45, 0.65);
/// Muted text for sub-details
const TEXT_MUTED: Color = Color::srgb(0.4, 0.35, 0.5);

// Grade rank colors
const RANK_SPP: Color = Color::srgb(1.0, 0.85, 0.15); // Molten gold
const RANK_SP: Color = Color::srgb(1.0, 0.75, 0.1);   // Warm gold
const RANK_S: Color = Color::srgb(0.0, 0.9, 1.0);     // Electric cyan
const RANK_A: Color = Color::srgb(0.0, 1.0, 0.4);     // Neon green
const RANK_B: Color = Color::srgb(0.5, 0.7, 1.0);     // Soft blue
const RANK_C: Color = Color::srgb(1.0, 0.85, 0.0);    // Amber
const RANK_D: Color = Color::srgb(1.0, 0.15, 0.3);    // Magenta-red

// Grade judgment colors (match judgment.rs)
const GREAT_CLR: Color = Color::srgb(0.0, 1.0, 0.4);
const COOL_CLR: Color = Color::srgb(0.0, 0.7, 1.0);
const GOOD_CLR: Color = Color::srgb(1.0, 0.85, 0.0);
const MISS_CLR: Color = Color::srgb(1.0, 0.15, 0.3);

// Font sizes
const RANK_FONT: f32 = 72.0;
const TOTAL_FONT: f32 = 36.0;
const BREAKDOWN_FONT: f32 = 20.0;
const BREAKDOWN_LABEL: f32 = 14.0;
const GRADE_FONT: f32 = 18.0;
const GRADE_LABEL: f32 = 12.0;
const DISMISS_FONT: f32 = 13.0;

// --- Resource ---

#[derive(Resource)]
pub struct SongComplete(pub bool);

// --- Systems ---

fn check_song_end(
    state: Option<Res<ScoreState>>,
    complete: Option<ResMut<SongComplete>>,
    feedback_q: Query<&JudgmentFeedback>,
    mut next_state: ResMut<NextState<GameScreen>>,
    current_state: Res<State<GameScreen>>,
) {
    if *current_state.get() != GameScreen::Playing {
        return;
    }

    let Some(mut complete) = complete else { return };
    if complete.0 {
        return;
    }

    let Some(state) = state else { return };

    if state.total_notes == 0 {
        return;
    }

    if state.notes_judged() < state.total_notes {
        return;
    }

    // Wait for all feedback animations to finish
    if !feedback_q.is_empty() {
        return;
    }

    complete.0 = true;
    info!(
        "Song complete! Score: {} | Rank: {}",
        state.total_score(),
        state.grade_rank().label()
    );
    next_state.set(GameScreen::Results);
}

fn spawn_results_overlay(
    mut commands: Commands,
    state: Option<Res<ScoreState>>,
) {
    let Some(state) = state else { return };

    let rank = state.grade_rank();
    let rank_color = grade_rank_color(rank);

    let play = state.play_score();
    let chain = state.chain_bonus();
    let clear = state.clear_bonus();
    let total = state.total_score();

    commands
        // Full-screen backdrop
        .spawn((
            DespawnOnExit(GameScreen::Results),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BACKDROP),
        ))
        .with_children(|backdrop: &mut ChildSpawnerCommands| {
            // Center panel
            backdrop
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect {
                            top: Val::Px(32.0),
                            bottom: Val::Px(24.0),
                            left: Val::Px(48.0),
                            right: Val::Px(48.0),
                        },
                        row_gap: Val::Px(16.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        min_width: Val::Px(360.0),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                    BorderColor::all(PANEL_BORDER),
                ))
                .with_children(|panel: &mut ChildSpawnerCommands| {
                    // --- Grade rank (huge, bold) ---
                    panel.spawn((
                        Text::new(rank.label()),
                        TextFont {
                            font_size: RANK_FONT,
                            ..default()
                        },
                        TextColor(rank_color),
                    ));

                    // --- Total score ---
                    panel.spawn((
                        Text::new(format!("{total}")),
                        TextFont {
                            font_size: TOTAL_FONT,
                            ..default()
                        },
                        TextColor(TEXT_PRIMARY),
                    ));

                    // --- Divider ---
                    panel.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(DIVIDER),
                    ));

                    // --- Score breakdown ---
                    panel
                        .spawn((Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: Val::Px(6.0),
                            width: Val::Percent(100.0),
                            ..default()
                        },))
                        .with_children(|breakdown: &mut ChildSpawnerCommands| {
                            spawn_breakdown_row(breakdown, "PLAY", &format!("{play}"), TEXT_PRIMARY);
                            spawn_breakdown_row(
                                breakdown,
                                "CHAIN",
                                &format!("{chain}"),
                                TEXT_PRIMARY,
                            );
                            spawn_breakdown_row(
                                breakdown,
                                "CLEAR",
                                &format!("{clear}"),
                                TEXT_PRIMARY,
                            );
                        });

                    // --- Divider ---
                    panel.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(DIVIDER),
                    ));

                    // --- Grade distribution ---
                    panel
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceEvenly,
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(16.0),
                            ..default()
                        },))
                        .with_children(|grades: &mut ChildSpawnerCommands| {
                            spawn_grade_column(grades, "GREAT", state.great_count, GREAT_CLR);
                            spawn_grade_column(grades, "COOL", state.cool_count, COOL_CLR);
                            spawn_grade_column(grades, "GOOD", state.good_count, GOOD_CLR);
                            spawn_grade_column(grades, "MISS", state.miss_count, MISS_CLR);
                        });

                    // --- Max chain ---
                    panel
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },))
                        .with_children(|chain_row: &mut ChildSpawnerCommands| {
                            chain_row.spawn((
                                Text::new("MAX CHAIN"),
                                TextFont {
                                    font_size: GRADE_LABEL,
                                    ..default()
                                },
                                TextColor(TEXT_LABEL),
                            ));
                            chain_row.spawn((
                                Text::new(format!(
                                    "{} / {}",
                                    state.max_chain, state.total_notes
                                )),
                                TextFont {
                                    font_size: GRADE_FONT,
                                    ..default()
                                },
                                TextColor(TEXT_PRIMARY),
                            ));
                        });

                    // --- Dismiss hint ---
                    panel.spawn((
                        Text::new("[A / SPACE]"),
                        TextFont {
                            font_size: DISMISS_FONT,
                            ..default()
                        },
                        TextColor(TEXT_MUTED),
                        Node {
                            margin: UiRect::top(Val::Px(12.0)),
                            ..default()
                        },
                    ));
                });
        });
}

fn dismiss_results(
    action: Res<ActionState<GameAction>>,
    mut next_state: ResMut<NextState<GameScreen>>,
) {
    if action.just_pressed(&GameAction::Confirm) {
        info!("Results dismissed → Song Select");
        next_state.set(GameScreen::SongSelect);
    }
}

fn cleanup_gameplay(
    mut commands: Commands,
    mut ctx: NonSendMut<KiraContext>,
    note_entities: Query<Entity, With<NoteAlive>>,
    feedback_entities: Query<Entity, With<JudgmentFeedback>>,
) {
    stop_song(&mut ctx);

    commands.remove_resource::<SplinePath>();
    commands.remove_resource::<NoteQueue>();
    commands.remove_resource::<SongConductor>();
    commands.remove_resource::<ScoreState>();
    commands.remove_resource::<SongComplete>();
    commands.remove_resource::<SelectedSong>();

    for entity in &note_entities {
        commands.entity(entity).despawn();
    }
    for entity in &feedback_entities {
        commands.entity(entity).despawn();
    }
}

fn spawn_breakdown_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: &str,
    value_color: Color,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            width: Val::Percent(100.0),
            ..default()
        },))
        .with_children(|row: &mut ChildSpawnerCommands| {
            row.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: BREAKDOWN_LABEL,
                    ..default()
                },
                TextColor(TEXT_LABEL),
            ));
            row.spawn((
                Text::new(value.to_string()),
                TextFont {
                    font_size: BREAKDOWN_FONT,
                    ..default()
                },
                TextColor(value_color),
            ));
        });
}

fn spawn_grade_column(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    count: u32,
    color: Color,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            ..default()
        },))
        .with_children(|col: &mut ChildSpawnerCommands| {
            col.spawn((
                Text::new(format!("{count}")),
                TextFont {
                    font_size: GRADE_FONT,
                    ..default()
                },
                TextColor(color),
            ));
            col.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: GRADE_LABEL,
                    ..default()
                },
                TextColor(color.with_alpha(0.6)),
            ));
        });
}

fn grade_rank_color(rank: GradeRank) -> Color {
    match rank {
        GradeRank::SPlusPlus => RANK_SPP,
        GradeRank::SPlus => RANK_SP,
        GradeRank::S => RANK_S,
        GradeRank::A => RANK_A,
        GradeRank::B => RANK_B,
        GradeRank::C => RANK_C,
        GradeRank::D => RANK_D,
    }
}
