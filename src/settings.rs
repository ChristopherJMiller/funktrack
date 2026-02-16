use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::GameAction;
use crate::state::GameScreen;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::Settings), setup_settings)
            .add_systems(
                Update,
                (navigate_settings, capture_remap, update_settings_ui)
                    .chain()
                    .run_if(in_state(GameScreen::Settings)),
            );
    }
}

// --- Y2K Future Punk palette (shared with song_select) ---

const BG_DARK: Color = Color::srgba(0.02, 0.01, 0.06, 0.95);
const PANEL_BG: Color = Color::srgba(0.06, 0.03, 0.12, 0.92);
const PANEL_BORDER: Color = Color::srgb(0.6, 0.2, 1.0);
const PANEL_BORDER_DIM: Color = Color::srgba(0.6, 0.2, 1.0, 0.3);
const SELECTED_BG: Color = Color::srgba(0.12, 0.04, 0.22, 0.95);
const SELECTED_BORDER: Color = Color::srgb(0.0, 0.9, 1.0);
const TITLE_COLOR: Color = Color::srgb(0.92, 0.96, 1.0);
const HINT_COLOR: Color = Color::srgb(0.4, 0.35, 0.5);
const HEADER_COLOR: Color = Color::srgb(0.6, 0.2, 1.0);
const ACCENT_CYAN: Color = Color::srgb(0.0, 0.9, 1.0);
const LISTENING_COLOR: Color = Color::srgb(1.0, 0.15, 0.3);

const HEADER_FONT: f32 = 36.0;
const ACTION_FONT: f32 = 20.0;
const BINDING_FONT: f32 = 16.0;
const HINT_FONT: f32 = 12.0;

// --- Remappable actions (excludes Confirm/Back needed for navigation) ---

const REMAPPABLE_ACTIONS: &[GameAction] = &[
    GameAction::Tap,
    GameAction::Up,
    GameAction::Down,
    GameAction::Left,
    GameAction::Right,
];

/// Total entries = remappable actions + 1 for "RESET DEFAULTS".
const TOTAL_ENTRIES: usize = 5 + 1;
const RESET_INDEX: usize = 5;

fn action_label(action: GameAction) -> &'static str {
    match action {
        GameAction::Tap => "TAP",
        GameAction::Confirm => "CONFIRM",
        GameAction::Back => "BACK",
        GameAction::Up => "UP",
        GameAction::Down => "DOWN",
        GameAction::Left => "LEFT",
        GameAction::Right => "RIGHT",
    }
}

// --- Resources ---

#[derive(Resource)]
struct SettingsState {
    selected_index: usize,
    listening: bool,
    actions: Vec<GameAction>,
}

// --- Markers ---

#[derive(Component)]
struct ActionRow(usize);

#[derive(Component)]
struct ActionNameText(usize);

#[derive(Component)]
struct BindingText(usize);

#[derive(Component)]
struct ResetRow;

#[derive(Component)]
struct ResetLabel;

// --- Systems ---

fn setup_settings(mut commands: Commands, input_map: Res<InputMap<GameAction>>) {
    let actions = REMAPPABLE_ACTIONS.to_vec();

    commands.insert_resource(SettingsState {
        selected_index: 0,
        listening: false,
        actions: actions.clone(),
    });

    commands
        .spawn((
            DespawnOnExit(GameScreen::Settings),
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
                Text::new("SETTINGS"),
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

            // Bindings panel
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    min_width: Val::Px(480.0),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(PANEL_BORDER),
            ))
            .with_children(|panel: &mut ChildSpawnerCommands| {
                // Action rows
                for (i, &action) in actions.iter().enumerate() {
                    let is_selected = i == 0;
                    let binding = binding_display_name(&input_map, &action);
                    spawn_action_row(panel, i, action, &binding, is_selected);
                }

                // Reset Defaults row
                let is_reset_selected = false;
                spawn_reset_row(panel, is_reset_selected);
            });

            // Controls hint
            root.spawn((Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(24.0),
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },))
            .with_children(|hints: &mut ChildSpawnerCommands| {
                spawn_hint(hints, "UP/DOWN", "select");
                spawn_hint(hints, "CONFIRM", "rebind / reset");
                spawn_hint(hints, "ESC", "back");
            });
        });
}

fn spawn_action_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    action: GameAction,
    binding: &str,
    is_selected: bool,
) {
    let (bg, border) = if is_selected {
        (SELECTED_BG, SELECTED_BORDER)
    } else {
        (Color::NONE, PANEL_BORDER_DIM)
    };

    parent
        .spawn((
            ActionRow(index),
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect {
                    top: Val::Px(10.0),
                    bottom: Val::Px(10.0),
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                },
                border: UiRect::left(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                min_width: Val::Px(440.0),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
        ))
        .with_children(|row: &mut ChildSpawnerCommands| {
            row.spawn((
                ActionNameText(index),
                Text::new(action_label(action)),
                TextFont {
                    font_size: ACTION_FONT,
                    ..default()
                },
                TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR }),
            ));
            row.spawn((
                BindingText(index),
                Text::new(binding.to_string()),
                TextFont {
                    font_size: BINDING_FONT,
                    ..default()
                },
                TextColor(HINT_COLOR),
            ));
        });
}

fn spawn_reset_row(parent: &mut ChildSpawnerCommands, is_selected: bool) {
    let (bg, border) = if is_selected {
        (SELECTED_BG, SELECTED_BORDER)
    } else {
        (Color::NONE, PANEL_BORDER_DIM)
    };

    parent.spawn((
        ResetRow,
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect {
                top: Val::Px(10.0),
                bottom: Val::Px(10.0),
                left: Val::Px(16.0),
                right: Val::Px(16.0),
            },
            border: UiRect::left(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            min_width: Val::Px(440.0),
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border),
        ResetLabel,
        Text::new("RESET DEFAULTS"),
        TextFont {
            font_size: ACTION_FONT,
            ..default()
        },
        TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR }),
    ));
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

fn binding_display_name(input_map: &InputMap<GameAction>, action: &GameAction) -> String {
    if let Some(bindings) = input_map.get_buttonlike(action) {
        let mut parts: Vec<String> = Vec::new();
        for binding in bindings {
            let s = format!("{:?}", binding);
            if !parts.contains(&s) {
                parts.push(s);
            }
        }
        if !parts.is_empty() {
            return parts.join("  |  ");
        }
    }
    "---".to_string()
}

fn navigate_settings(
    action: Res<ActionState<GameAction>>,
    mut state: ResMut<SettingsState>,
    mut next_state: ResMut<NextState<GameScreen>>,
    mut commands: Commands,
) {
    if state.listening {
        // While listening, only Back cancels
        if action.just_pressed(&GameAction::Back) {
            state.listening = false;
        }
        return;
    }

    if action.just_pressed(&GameAction::Back) {
        next_state.set(GameScreen::SongSelect);
        return;
    }

    if action.just_pressed(&GameAction::Up) {
        if state.selected_index > 0 {
            state.selected_index -= 1;
        } else {
            state.selected_index = TOTAL_ENTRIES - 1;
        }
    }

    if action.just_pressed(&GameAction::Down) {
        state.selected_index = (state.selected_index + 1) % TOTAL_ENTRIES;
    }

    if action.just_pressed(&GameAction::Confirm) {
        if state.selected_index == RESET_INDEX {
            // Reset all bindings to defaults
            let default_map = default_input_map();
            commands.insert_resource(default_map);
            info!("Input bindings reset to defaults");
        } else {
            // Enter listening mode for the selected action
            state.listening = true;
        }
    }
}

fn capture_remap(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SettingsState>,
    mut input_map: ResMut<InputMap<GameAction>>,
) {
    if !state.listening {
        return;
    }

    let action = state.actions[state.selected_index];

    // Check for any key press (skip modifiers and Escape)
    for &key in keys.get_just_pressed() {
        if matches!(
            key,
            KeyCode::Escape
                | KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::ControlLeft
                | KeyCode::ControlRight
                | KeyCode::AltLeft
                | KeyCode::AltRight
                | KeyCode::SuperLeft
                | KeyCode::SuperRight
        ) {
            continue;
        }

        // Clear all bindings for this action and insert the new keyboard binding
        clear_action_bindings(&mut input_map, action);
        input_map.insert(action, key);
        info!("Rebound {} -> {:?}", action_label(action), key);
        state.listening = false;
        return;
    }
}

/// Remove all bindings for a given action.
fn clear_action_bindings(input_map: &mut InputMap<GameAction>, action: GameAction) {
    input_map.clear_action(&action);
}

fn update_settings_ui(
    state: Res<SettingsState>,
    input_map: Res<InputMap<GameAction>>,
    mut rows: Query<(&ActionRow, &mut BackgroundColor, &mut BorderColor)>,
    mut names: Query<(&ActionNameText, &mut TextColor), Without<BindingText>>,
    mut bindings: Query<(&BindingText, &mut Text, &mut TextColor), Without<ActionNameText>>,
    mut reset_row: Query<
        (&mut BackgroundColor, &mut BorderColor, &mut TextColor),
        (With<ResetRow>, Without<ActionRow>, Without<ActionNameText>, Without<BindingText>),
    >,
) {
    if !state.is_changed() && !input_map.is_changed() {
        return;
    }

    // Update action rows
    for (row, mut bg, mut border) in &mut rows {
        let is_selected = row.0 == state.selected_index;
        *bg = BackgroundColor(if is_selected { SELECTED_BG } else { Color::NONE });
        *border = BorderColor::all(if is_selected {
            SELECTED_BORDER
        } else {
            PANEL_BORDER_DIM
        });
    }

    // Update action name text colors
    for (name, mut color) in &mut names {
        let is_selected = name.0 == state.selected_index;
        *color = TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR });
    }

    // Update binding text
    for (binding, mut text, mut color) in &mut bindings {
        let is_selected = binding.0 == state.selected_index;
        let is_listening = is_selected && state.listening;
        let action = &state.actions[binding.0];

        if is_listening {
            **text = "[PRESS KEY]".to_string();
            *color = TextColor(LISTENING_COLOR);
        } else {
            **text = binding_display_name(&input_map, action);
            *color = TextColor(HINT_COLOR);
        }
    }

    // Update reset row
    let is_reset_selected = state.selected_index == RESET_INDEX;
    for (mut bg, mut border, mut color) in &mut reset_row {
        *bg = BackgroundColor(if is_reset_selected {
            SELECTED_BG
        } else {
            Color::NONE
        });
        *border = BorderColor::all(if is_reset_selected {
            SELECTED_BORDER
        } else {
            PANEL_BORDER_DIM
        });
        *color = TextColor(if is_reset_selected {
            ACCENT_CYAN
        } else {
            TITLE_COLOR
        });
    }
}

/// Rebuild the default input map (mirrors `GameAction::default_input_map` in action.rs).
fn default_input_map() -> InputMap<GameAction> {
    use GameAction::*;
    let mut map = InputMap::default();

    // Gameplay
    map.insert(Tap, KeyCode::Space);
    map.insert(Tap, GamepadButton::South);

    // Menu confirm
    map.insert(Confirm, KeyCode::Space);
    map.insert(Confirm, KeyCode::Enter);
    map.insert(Confirm, GamepadButton::South);

    // Menu back
    map.insert(Back, KeyCode::Escape);
    map.insert(Back, GamepadButton::East);

    // Navigation — keyboard
    map.insert(Up, KeyCode::ArrowUp);
    map.insert(Down, KeyCode::ArrowDown);
    map.insert(Left, KeyCode::ArrowLeft);
    map.insert(Right, KeyCode::ArrowRight);

    // Navigation — gamepad d-pad
    map.insert(Up, GamepadButton::DPadUp);
    map.insert(Down, GamepadButton::DPadDown);
    map.insert(Left, GamepadButton::DPadLeft);
    map.insert(Right, GamepadButton::DPadRight);

    // Navigation — gamepad left stick
    map.insert(Up, GamepadControlDirection::LEFT_UP);
    map.insert(Down, GamepadControlDirection::LEFT_DOWN);
    map.insert(Left, GamepadControlDirection::LEFT_LEFT);
    map.insert(Right, GamepadControlDirection::LEFT_RIGHT);

    map
}
