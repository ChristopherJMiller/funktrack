use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::GameAction;
use crate::beatmap::{Difficulty, DiscoveredSong, SelectedSong, discover_songs, load_chart};
use crate::state::GameScreen;

pub struct SongSelectPlugin;

impl Plugin for SongSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::SongSelect), setup_song_select)
            .add_systems(
                Update,
                (navigate_songs, update_song_select_ui)
                    .chain()
                    .run_if(in_state(GameScreen::SongSelect)),
            );
    }
}

// --- Y2K Future Punk palette ---

const BG_DARK: Color = Color::srgba(0.02, 0.01, 0.06, 0.95);
const PANEL_BG: Color = Color::srgba(0.06, 0.03, 0.12, 0.92);
const PANEL_BORDER: Color = Color::srgb(0.6, 0.2, 1.0);
const PANEL_BORDER_DIM: Color = Color::srgba(0.6, 0.2, 1.0, 0.3);
const SELECTED_BG: Color = Color::srgba(0.12, 0.04, 0.22, 0.95);
const SELECTED_BORDER: Color = Color::srgb(0.0, 0.9, 1.0);
const TITLE_COLOR: Color = Color::srgb(0.92, 0.96, 1.0);
const ARTIST_COLOR: Color = Color::srgb(0.55, 0.45, 0.65);
const DIFF_ACTIVE: Color = Color::srgb(0.0, 1.0, 0.4);
const DIFF_INACTIVE: Color = Color::srgb(0.35, 0.3, 0.45);
const HINT_COLOR: Color = Color::srgb(0.4, 0.35, 0.5);
const HEADER_COLOR: Color = Color::srgb(0.6, 0.2, 1.0);
const ACCENT_CYAN: Color = Color::srgb(0.0, 0.9, 1.0);

const HEADER_FONT: f32 = 36.0;
const SONG_TITLE_FONT: f32 = 22.0;
const SONG_ARTIST_FONT: f32 = 14.0;
const DIFF_FONT: f32 = 13.0;
const HINT_FONT: f32 = 12.0;

// --- Resources ---

#[derive(Resource)]
struct SongSelectState {
    songs: Vec<DiscoveredSong>,
    selected_index: usize,
    selected_difficulty_index: usize,
}

impl SongSelectState {
    fn available_difficulties(&self) -> &[Difficulty] {
        if self.songs.is_empty() {
            return &[];
        }
        &self.songs[self.selected_index].metadata.difficulties
    }

    fn current_difficulty(&self) -> Option<Difficulty> {
        let diffs = self.available_difficulties();
        diffs.get(self.selected_difficulty_index).copied()
    }
}

// --- Markers ---

#[derive(Component)]
struct SongListItem(usize);

#[derive(Component)]
struct SongTitleText(usize);

#[derive(Component)]
struct SongArtistText(usize);

#[derive(Component)]
struct DifficultyIndicator(usize, Difficulty);

#[derive(Component)]
struct DifficultyDisplay;

// --- Systems ---

fn setup_song_select(mut commands: Commands) {
    let songs = discover_songs(std::path::Path::new("assets/songs"));

    let initial_diff_idx = 0;
    let state = SongSelectState {
        songs,
        selected_index: 0,
        selected_difficulty_index: initial_diff_idx,
    };

    // Root — full screen, dark background
    commands
        .spawn((
            DespawnOnExit(GameScreen::SongSelect),
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
        .with_children(|root: &mut ChildSpawnerCommands| {
            // Header
            root.spawn((
                Text::new("SELECT TRACK"),
                TextFont {
                    font_size: HEADER_FONT,
                    ..default()
                },
                TextColor(HEADER_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            // Song list panel
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    min_width: Val::Px(460.0),
                    max_height: Val::Px(420.0),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(PANEL_BORDER),
            ))
            .with_children(|list: &mut ChildSpawnerCommands| {
                if state.songs.is_empty() {
                    list.spawn((
                        Text::new("No songs found in assets/songs/"),
                        TextFont {
                            font_size: SONG_TITLE_FONT,
                            ..default()
                        },
                        TextColor(HINT_COLOR),
                    ));
                } else {
                    for (i, song) in state.songs.iter().enumerate() {
                        let is_selected = i == state.selected_index;
                        spawn_song_row(list, i, song, is_selected);
                    }
                }
            });

            // Difficulty selector
            root.spawn((
                DifficultyDisplay,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    padding: UiRect {
                        top: Val::Px(8.0),
                        bottom: Val::Px(8.0),
                        left: Val::Px(24.0),
                        right: Val::Px(24.0),
                    },
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.02, 0.08, 0.8)),
                BorderColor::all(PANEL_BORDER_DIM),
            ))
            .with_children(|diff_row: &mut ChildSpawnerCommands| {
                let all_diffs = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard, Difficulty::Expert];
                for diff in all_diffs {
                    let is_available = !state.songs.is_empty()
                        && state.songs[state.selected_index]
                            .metadata
                            .difficulties
                            .contains(&diff);
                    let is_selected = state.current_difficulty() == Some(diff);

                    let color = if is_selected {
                        DIFF_ACTIVE
                    } else if is_available {
                        DIFF_INACTIVE
                    } else {
                        Color::srgba(0.25, 0.2, 0.3, 0.4)
                    };

                    diff_row.spawn((
                        DifficultyIndicator(state.selected_index, diff),
                        Text::new(diff.label()),
                        TextFont {
                            font_size: DIFF_FONT,
                            ..default()
                        },
                        TextColor(color),
                    ));
                }
            });

            // Controls hint
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(24.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ))
            .with_children(|hints: &mut ChildSpawnerCommands| {
                spawn_hint(hints, "UP/DOWN", "select");
                spawn_hint(hints, "LEFT/RIGHT", "difficulty");
                spawn_hint(hints, "A/SPACE", "play");
            });
        });

    commands.insert_resource(state);
}

fn spawn_song_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    song: &DiscoveredSong,
    is_selected: bool,
) {
    let (bg, border) = if is_selected {
        (SELECTED_BG, SELECTED_BORDER)
    } else {
        (Color::NONE, PANEL_BORDER_DIM)
    };

    parent
        .spawn((
            SongListItem(index),
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    top: Val::Px(10.0),
                    bottom: Val::Px(10.0),
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                },
                border: UiRect::left(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
        ))
        .with_children(|row: &mut ChildSpawnerCommands| {
            row.spawn((
                SongTitleText(index),
                Text::new(&song.metadata.title),
                TextFont {
                    font_size: SONG_TITLE_FONT,
                    ..default()
                },
                TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR }),
            ));
            row.spawn((
                SongArtistText(index),
                Text::new(&song.metadata.artist),
                TextFont {
                    font_size: SONG_ARTIST_FONT,
                    ..default()
                },
                TextColor(ARTIST_COLOR),
            ));
        });
}

fn spawn_hint(parent: &mut ChildSpawnerCommands, key: &str, action: &str) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|hint: &mut ChildSpawnerCommands| {
            hint.spawn((
                Text::new(format!("[{key}]")),
                TextFont {
                    font_size: HINT_FONT,
                    ..default()
                },
                TextColor(ACCENT_CYAN.with_alpha(0.6)),
            ));
            hint.spawn((
                Text::new(action.to_string()),
                TextFont {
                    font_size: HINT_FONT,
                    ..default()
                },
                TextColor(HINT_COLOR),
            ));
        });
}

fn navigate_songs(
    action: Res<ActionState<GameAction>>,
    mut state: ResMut<SongSelectState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameScreen>>,
) {
    if state.songs.is_empty() {
        return;
    }

    let mut changed = false;

    if action.just_pressed(&GameAction::Up) {
        if state.selected_index > 0 {
            state.selected_index -= 1;
        } else {
            state.selected_index = state.songs.len() - 1;
        }
        // Reset difficulty index to first available
        state.selected_difficulty_index = 0;
        changed = true;
    }

    if action.just_pressed(&GameAction::Down) {
        state.selected_index = (state.selected_index + 1) % state.songs.len();
        state.selected_difficulty_index = 0;
        changed = true;
    }

    if action.just_pressed(&GameAction::Left) {
        let diffs = state.available_difficulties();
        if !diffs.is_empty() && state.selected_difficulty_index > 0 {
            state.selected_difficulty_index -= 1;
            changed = true;
        }
    }

    if action.just_pressed(&GameAction::Right) {
        let diffs = state.available_difficulties();
        if !diffs.is_empty() && state.selected_difficulty_index < diffs.len() - 1 {
            state.selected_difficulty_index += 1;
            changed = true;
        }
    }

    if action.just_pressed(&GameAction::Confirm) {
        let Some(difficulty) = state.current_difficulty() else {
            warn!("No difficulty selected");
            return;
        };

        let song = &state.songs[state.selected_index];
        match load_chart(&song.dir, difficulty) {
            Ok(chart) => {
                info!(
                    "Selected: {} [{}]",
                    song.metadata.title,
                    difficulty.label()
                );
                commands.insert_resource(SelectedSong {
                    song_dir: song.dir.clone(),
                    difficulty,
                    metadata: song.metadata.clone(),
                    chart,
                });
                next_state.set(GameScreen::Playing);
            }
            Err(err) => {
                error!("Failed to load chart: {}", err);
            }
        }
    }

    let _ = changed; // UI update handled by update_song_select_ui
}

fn update_song_select_ui(
    state: Res<SongSelectState>,
    mut song_items: Query<(&SongListItem, &mut BackgroundColor, &mut BorderColor)>,
    mut title_texts: Query<(&SongTitleText, &mut TextColor), Without<SongArtistText>>,
    mut diff_indicators: Query<(&DifficultyIndicator, &mut TextColor), Without<SongTitleText>>,
) {
    if !state.is_changed() {
        return;
    }

    // Update song list selection visuals
    for (item, mut bg, mut border) in &mut song_items {
        let is_selected = item.0 == state.selected_index;
        *bg = BackgroundColor(if is_selected { SELECTED_BG } else { Color::NONE });
        *border = BorderColor::all(if is_selected { SELECTED_BORDER } else { PANEL_BORDER_DIM });
    }

    for (title, mut color) in &mut title_texts {
        let is_selected = title.0 == state.selected_index;
        *color = TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR });
    }

    // Update difficulty indicators
    let current_diff = state.current_difficulty();
    for (indicator, mut color) in &mut diff_indicators {
        let is_available = !state.songs.is_empty()
            && state.songs[state.selected_index]
                .metadata
                .difficulties
                .contains(&indicator.1);
        let is_selected = current_diff == Some(indicator.1);

        *color = TextColor(if is_selected {
            DIFF_ACTIVE
        } else if is_available {
            DIFF_INACTIVE
        } else {
            Color::srgba(0.25, 0.2, 0.3, 0.4)
        });
    }
}
