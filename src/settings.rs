use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::GameAction;
use crate::config::GameSettings;
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
            )
            .add_systems(OnExit(GameScreen::Settings), save_on_exit);
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
const HINT_COLOR: Color = Color::srgb(0.4, 0.35, 0.5);
const HEADER_COLOR: Color = Color::srgb(0.6, 0.2, 1.0);
const ACCENT_CYAN: Color = Color::srgb(0.0, 0.9, 1.0);
const LISTENING_COLOR: Color = Color::srgb(1.0, 0.15, 0.3);
const TAB_ACTIVE_BG: Color = Color::srgba(0.12, 0.04, 0.22, 0.95);
const TAB_INACTIVE_BG: Color = Color::NONE;
const TAB_UNDERLINE_ACTIVE: Color = Color::srgb(0.0, 0.9, 1.0);
const TAB_UNDERLINE_INACTIVE: Color = Color::srgba(0.6, 0.2, 1.0, 0.15);
const VALUE_COLOR: Color = Color::srgb(0.0, 1.0, 0.4);
const SLIDER_TRACK: Color = Color::srgba(0.2, 0.1, 0.35, 0.6);
const SLIDER_FILL: Color = Color::srgb(0.0, 0.9, 1.0);
const TOGGLE_ON: Color = Color::srgb(0.0, 1.0, 0.4);
const TOGGLE_OFF: Color = Color::srgb(0.4, 0.35, 0.5);

const HEADER_FONT: f32 = 36.0;
const TAB_FONT: f32 = 16.0;
const ROW_LABEL_FONT: f32 = 18.0;
const ROW_VALUE_FONT: f32 = 16.0;
const HINT_FONT: f32 = 12.0;

// --- Tab definitions ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Input,
    Audio,
    Visual,
    Display,
}

impl SettingsTab {
    const ALL: &[SettingsTab] = &[
        SettingsTab::Input,
        SettingsTab::Audio,
        SettingsTab::Visual,
        SettingsTab::Display,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsTab::Input => "INPUT",
            SettingsTab::Audio => "AUDIO",
            SettingsTab::Visual => "VISUAL",
            SettingsTab::Display => "DISPLAY",
        }
    }

    fn row_count(self) -> usize {
        match self {
            SettingsTab::Input => 6,   // 5 remappable + reset
            SettingsTab::Audio => 5,   // master, sfx, preview, audio offset, calibrate
            SettingsTab::Visual => 2,  // visual offset, note speed
            SettingsTab::Display => 1, // fullscreen
        }
    }
}

// --- Row types ---

#[derive(Debug, Clone, Copy)]
enum RowKind {
    KeyBind(GameAction),
    ResetBindings,
    Slider { min: f32, max: f32, step: f32 },
    Offset { min: i32, max: i32, step: i32 },
    Toggle,
    NavAction,
}

struct RowDef {
    label: &'static str,
    kind: RowKind,
}

fn rows_for_tab(tab: SettingsTab) -> Vec<RowDef> {
    match tab {
        SettingsTab::Input => vec![
            RowDef { label: "TAP", kind: RowKind::KeyBind(GameAction::Tap) },
            RowDef { label: "UP", kind: RowKind::KeyBind(GameAction::Up) },
            RowDef { label: "DOWN", kind: RowKind::KeyBind(GameAction::Down) },
            RowDef { label: "LEFT", kind: RowKind::KeyBind(GameAction::Left) },
            RowDef { label: "RIGHT", kind: RowKind::KeyBind(GameAction::Right) },
            RowDef { label: "RESET DEFAULTS", kind: RowKind::ResetBindings },
        ],
        SettingsTab::Audio => vec![
            RowDef { label: "MASTER VOLUME", kind: RowKind::Slider { min: 0.0, max: 100.0, step: 5.0 } },
            RowDef { label: "SFX VOLUME", kind: RowKind::Slider { min: 0.0, max: 100.0, step: 5.0 } },
            RowDef { label: "PREVIEW VOLUME", kind: RowKind::Slider { min: 0.0, max: 100.0, step: 5.0 } },
            RowDef { label: "AUDIO OFFSET", kind: RowKind::Offset { min: -200, max: 200, step: 5 } },
            RowDef { label: "TAP TO CALIBRATE", kind: RowKind::NavAction },
        ],
        SettingsTab::Visual => vec![
            RowDef { label: "VISUAL OFFSET", kind: RowKind::Offset { min: -200, max: 200, step: 5 } },
            RowDef { label: "NOTE SPEED", kind: RowKind::Slider { min: 0.5, max: 3.0, step: 0.1 } },
        ],
        SettingsTab::Display => vec![
            RowDef { label: "FULLSCREEN", kind: RowKind::Toggle },
        ],
    }
}

// --- Resources ---

#[derive(Resource)]
struct SettingsState {
    active_tab: SettingsTab,
    selected_row: usize,
    listening: bool,
    dirty: bool,
}

// --- Markers ---

#[derive(Component)]
struct TabButton(SettingsTab);

#[derive(Component)]
struct TabUnderline(SettingsTab);

#[derive(Component)]
struct TabPanel(SettingsTab);

#[derive(Component)]
struct SettingRow(SettingsTab, usize);

#[derive(Component)]
struct RowLabelText(SettingsTab, usize);

#[derive(Component)]
struct RowValueText(SettingsTab, usize);

#[derive(Component)]
#[allow(dead_code)]
struct SliderTrack(SettingsTab, usize);

#[derive(Component)]
struct SliderFill(SettingsTab, usize);

#[derive(Component)]
struct HintBar;

// --- Setup ---

fn setup_settings(
    mut commands: Commands,
    input_map: Res<InputMap<GameAction>>,
    settings: Res<GameSettings>,
) {
    commands.insert_resource(SettingsState {
        active_tab: SettingsTab::Input,
        selected_row: 0,
        listening: false,
        dirty: true,
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
                row_gap: Val::Px(16.0),
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(BG_DARK),
        ))
        .with_children(|root| {
            // Header
            root.spawn((
                Text::new("SETTINGS"),
                TextFont { font_size: HEADER_FONT, ..default() },
                TextColor(HEADER_COLOR),
                Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
            ));

            // Tab bar
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(0.0),
                    min_width: Val::Px(520.0),
                    ..default()
                },
            ))
            .with_children(|tab_bar| {
                for &tab in SettingsTab::ALL {
                    let is_active = tab == SettingsTab::Input;
                    tab_bar.spawn((
                        TabButton(tab),
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            flex_grow: 1.0,
                            padding: UiRect {
                                top: Val::Px(10.0),
                                bottom: Val::Px(0.0),
                                left: Val::Px(12.0),
                                right: Val::Px(12.0),
                            },
                            row_gap: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(if is_active { TAB_ACTIVE_BG } else { TAB_INACTIVE_BG }),
                    ))
                    .with_children(|tab_btn| {
                        tab_btn.spawn((
                            Text::new(tab.label()),
                            TextFont { font_size: TAB_FONT, ..default() },
                            TextColor(if is_active { ACCENT_CYAN } else { HINT_COLOR }),
                        ));
                        // Underline indicator
                        tab_btn.spawn((
                            TabUnderline(tab),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(if is_active { 3.0 } else { 1.0 }),
                                ..default()
                            },
                            BackgroundColor(if is_active { TAB_UNDERLINE_ACTIVE } else { TAB_UNDERLINE_INACTIVE }),
                        ));
                    });
                }
            });

            // Content panel
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::new(
                        Val::Px(0.0), Val::Px(0.0), Val::Px(6.0), Val::Px(6.0),
                    ),
                    min_width: Val::Px(520.0),
                    min_height: Val::Px(280.0),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(PANEL_BORDER),
            ))
            .with_children(|panel| {
                for &tab in SettingsTab::ALL {
                    let rows = rows_for_tab(tab);
                    let visible = tab == SettingsTab::Input;
                    panel.spawn((
                        TabPanel(tab),
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            width: Val::Percent(100.0),
                            display: if visible { Display::Flex } else { Display::None },
                            ..default()
                        },
                    ))
                    .with_children(|tab_panel| {
                        for (i, row_def) in rows.iter().enumerate() {
                            let is_selected = visible && i == 0;
                            spawn_setting_row(
                                tab_panel, tab, i, row_def, is_selected,
                                &input_map, &settings,
                            );
                        }
                    });
                }
            });

            // Hints
            root.spawn((
                HintBar,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(24.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ))
            .with_children(|hints| {
                spawn_hint(hints, "Q/E", "tab");
                spawn_hint(hints, "UP/DOWN", "select");
                spawn_hint(hints, "LEFT/RIGHT", "adjust");
                spawn_hint(hints, "CONFIRM", "rebind");
                spawn_hint(hints, "ESC", "back");
            });
        });
}

fn spawn_setting_row(
    parent: &mut ChildSpawnerCommands,
    tab: SettingsTab,
    index: usize,
    row_def: &RowDef,
    is_selected: bool,
    input_map: &InputMap<GameAction>,
    settings: &GameSettings,
) {
    let (bg, border) = if is_selected {
        (SELECTED_BG, SELECTED_BORDER)
    } else {
        (Color::NONE, PANEL_BORDER_DIM)
    };

    let is_centered = matches!(row_def.kind, RowKind::ResetBindings | RowKind::NavAction);

    parent.spawn((
        SettingRow(tab, index),
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: if is_centered { JustifyContent::Center } else { JustifyContent::SpaceBetween },
            align_items: AlignItems::Center,
            padding: UiRect {
                top: Val::Px(8.0),
                bottom: Val::Px(8.0),
                left: Val::Px(16.0),
                right: Val::Px(16.0),
            },
            border: UiRect::left(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            min_width: Val::Px(480.0),
            margin: if is_centered { UiRect::top(Val::Px(8.0)) } else { UiRect::ZERO },
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border),
    ))
    .with_children(|row| {
        // Label
        row.spawn((
            RowLabelText(tab, index),
            Text::new(row_def.label),
            TextFont { font_size: ROW_LABEL_FONT, ..default() },
            TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR }),
        ));

        // Value display (right side)
        match row_def.kind {
            RowKind::KeyBind(action) => {
                let binding = binding_display_name(input_map, &action);
                row.spawn((
                    RowValueText(tab, index),
                    Text::new(binding),
                    TextFont { font_size: ROW_VALUE_FONT, ..default() },
                    TextColor(HINT_COLOR),
                ));
            }
            RowKind::Slider { min, max, .. } => {
                let value = get_slider_value(tab, index, settings);
                // Right side: value text + slider bar
                row.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|right| {
                    // Slider track
                    right.spawn((
                        SliderTrack(tab, index),
                        Node {
                            width: Val::Px(140.0),
                            height: Val::Px(8.0),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK),
                    ))
                    .with_children(|track| {
                        let pct = ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0);
                        track.spawn((
                            SliderFill(tab, index),
                            Node {
                                width: Val::Percent(pct),
                                height: Val::Percent(100.0),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(SLIDER_FILL),
                        ));
                    });

                    // Value text
                    let display = format_slider_value(tab, index, value);
                    right.spawn((
                        RowValueText(tab, index),
                        Text::new(display),
                        TextFont { font_size: ROW_VALUE_FONT, ..default() },
                        TextColor(VALUE_COLOR),
                        Node { min_width: Val::Px(48.0), ..default() },
                    ));
                });
            }
            RowKind::Offset { .. } => {
                let value = get_offset_value(tab, index, settings);
                let sign = if value >= 0 { "+" } else { "" };
                row.spawn((
                    RowValueText(tab, index),
                    Text::new(format!("{sign}{value} ms")),
                    TextFont { font_size: ROW_VALUE_FONT, ..default() },
                    TextColor(VALUE_COLOR),
                ));
            }
            RowKind::Toggle => {
                let on = get_toggle_value(tab, index, settings);
                row.spawn((
                    RowValueText(tab, index),
                    Text::new(if on { "ON" } else { "OFF" }),
                    TextFont { font_size: ROW_VALUE_FONT, ..default() },
                    TextColor(if on { TOGGLE_ON } else { TOGGLE_OFF }),
                ));
            }
            RowKind::ResetBindings | RowKind::NavAction => {}
        }
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

// --- Value accessors ---

fn get_slider_value(tab: SettingsTab, index: usize, settings: &GameSettings) -> f32 {
    match (tab, index) {
        (SettingsTab::Audio, 0) => settings.master_volume,
        (SettingsTab::Audio, 1) => settings.sfx_volume,
        (SettingsTab::Audio, 2) => settings.preview_volume,
        (SettingsTab::Visual, 1) => settings.note_speed,
        _ => 0.0,
    }
}

fn set_slider_value(tab: SettingsTab, index: usize, settings: &mut GameSettings, value: f32) {
    match (tab, index) {
        (SettingsTab::Audio, 0) => settings.master_volume = value,
        (SettingsTab::Audio, 1) => settings.sfx_volume = value,
        (SettingsTab::Audio, 2) => settings.preview_volume = value,
        (SettingsTab::Visual, 1) => settings.note_speed = value,
        _ => {}
    }
}

fn get_offset_value(tab: SettingsTab, index: usize, settings: &GameSettings) -> i32 {
    match (tab, index) {
        (SettingsTab::Audio, 3) => settings.audio_offset_ms,
        (SettingsTab::Visual, 0) => settings.visual_offset_ms,
        _ => 0,
    }
}

fn set_offset_value(tab: SettingsTab, index: usize, settings: &mut GameSettings, value: i32) {
    match (tab, index) {
        (SettingsTab::Audio, 3) => settings.audio_offset_ms = value,
        (SettingsTab::Visual, 0) => settings.visual_offset_ms = value,
        _ => {}
    }
}

fn get_toggle_value(tab: SettingsTab, index: usize, settings: &GameSettings) -> bool {
    match (tab, index) {
        (SettingsTab::Display, 0) => settings.fullscreen,
        _ => false,
    }
}

fn set_toggle_value(tab: SettingsTab, index: usize, settings: &mut GameSettings, value: bool) {
    match (tab, index) {
        (SettingsTab::Display, 0) => settings.fullscreen = value,
        _ => {}
    }
}

fn format_slider_value(tab: SettingsTab, index: usize, value: f32) -> String {
    match (tab, index) {
        (SettingsTab::Visual, 1) => format!("{value:.1}x"),
        _ => format!("{:.0}%", value),
    }
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

// --- Navigation ---

fn navigate_settings(
    action: Res<ActionState<GameAction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SettingsState>,
    mut next_state: ResMut<NextState<GameScreen>>,
    mut settings: ResMut<GameSettings>,
    mut commands: Commands,
    mut windows: Query<&mut Window>,
) {
    if state.listening {
        if action.just_pressed(&GameAction::Back) {
            state.listening = false;
            state.dirty = true;
        }
        return;
    }

    if action.just_pressed(&GameAction::Back) {
        next_state.set(GameScreen::SongSelect);
        return;
    }

    // Tab switching with Q/E
    if keys.just_pressed(KeyCode::KeyQ) {
        let idx = SettingsTab::ALL.iter().position(|&t| t == state.active_tab).unwrap();
        if idx > 0 {
            state.active_tab = SettingsTab::ALL[idx - 1];
            state.selected_row = 0;
            state.dirty = true;
        }
    }
    if keys.just_pressed(KeyCode::KeyE) {
        let idx = SettingsTab::ALL.iter().position(|&t| t == state.active_tab).unwrap();
        if idx < SettingsTab::ALL.len() - 1 {
            state.active_tab = SettingsTab::ALL[idx + 1];
            state.selected_row = 0;
            state.dirty = true;
        }
    }

    // Row navigation
    let row_count = state.active_tab.row_count();
    if action.just_pressed(&GameAction::Up) {
        if state.selected_row > 0 {
            state.selected_row -= 1;
        } else {
            state.selected_row = row_count - 1;
        }
        state.dirty = true;
    }
    if action.just_pressed(&GameAction::Down) {
        state.selected_row = (state.selected_row + 1) % row_count;
        state.dirty = true;
    }

    // Value adjustment / action
    let tab = state.active_tab;
    let row = state.selected_row;
    let rows = rows_for_tab(tab);
    let row_def = &rows[row];

    match row_def.kind {
        RowKind::KeyBind(_) => {
            if action.just_pressed(&GameAction::Confirm) {
                state.listening = true;
                state.dirty = true;
            }
        }
        RowKind::ResetBindings => {
            if action.just_pressed(&GameAction::Confirm) {
                commands.insert_resource(default_input_map());
                info!("Input bindings reset to defaults");
                state.dirty = true;
            }
        }
        RowKind::Slider { min, max, step } => {
            if action.just_pressed(&GameAction::Left) {
                let val = get_slider_value(tab, row, &settings);
                let new_val = (val - step).max(min);
                set_slider_value(tab, row, &mut settings, new_val);
                state.dirty = true;
            }
            if action.just_pressed(&GameAction::Right) {
                let val = get_slider_value(tab, row, &settings);
                let new_val = (val + step).min(max);
                set_slider_value(tab, row, &mut settings, new_val);
                state.dirty = true;
            }
        }
        RowKind::Offset { min, max, step } => {
            if action.just_pressed(&GameAction::Left) {
                let val = get_offset_value(tab, row, &settings);
                let new_val = (val - step).max(min);
                set_offset_value(tab, row, &mut settings, new_val);
                state.dirty = true;
            }
            if action.just_pressed(&GameAction::Right) {
                let val = get_offset_value(tab, row, &settings);
                let new_val = (val + step).min(max);
                set_offset_value(tab, row, &mut settings, new_val);
                state.dirty = true;
            }
        }
        RowKind::Toggle => {
            if action.just_pressed(&GameAction::Confirm)
                || action.just_pressed(&GameAction::Left)
                || action.just_pressed(&GameAction::Right)
            {
                let val = get_toggle_value(tab, row, &settings);
                set_toggle_value(tab, row, &mut settings, !val);
                // Apply fullscreen immediately
                if tab == SettingsTab::Display && row == 0 {
                    if let Ok(mut window) = windows.single_mut() {
                        window.mode = if !val {
                            bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                        } else {
                            bevy::window::WindowMode::Windowed
                        };
                    }
                }
                state.dirty = true;
            }
        }
        RowKind::NavAction => {
            if action.just_pressed(&GameAction::Confirm) {
                // Navigate to calibration screen
                next_state.set(GameScreen::Calibration);
            }
        }
    }
}

fn capture_remap(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SettingsState>,
    mut input_map: ResMut<InputMap<GameAction>>,
) {
    if !state.listening || state.active_tab != SettingsTab::Input {
        return;
    }

    let rows = rows_for_tab(SettingsTab::Input);
    let RowKind::KeyBind(action) = rows[state.selected_row].kind else {
        return;
    };

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

        input_map.clear_action(&action);
        input_map.insert(action, key);
        info!("Rebound {} -> {:?}", rows[state.selected_row].label, key);
        state.listening = false;
        state.dirty = true;
        return;
    }
}

// --- UI Update ---

fn update_settings_ui(
    mut state: ResMut<SettingsState>,
    input_map: Res<InputMap<GameAction>>,
    settings: Res<GameSettings>,
    // Tab bar
    tab_buttons: Query<(&TabButton, &Children)>,
    mut tab_underlines: Query<(&TabUnderline, &mut Node, &mut BackgroundColor), Without<SliderFill>>,
    mut tab_texts: Query<&mut TextColor, (Without<RowLabelText>, Without<RowValueText>, Without<TabUnderline>)>,
    // Panels
    mut tab_panels: Query<(&TabPanel, &mut Node), (Without<TabUnderline>, Without<SliderFill>, Without<SettingRow>)>,
    // Rows
    mut row_nodes: Query<(&SettingRow, &mut BackgroundColor, &mut BorderColor), (Without<TabButton>, Without<TabUnderline>, Without<SliderFill>, Without<SliderTrack>)>,
    mut row_labels: Query<(&RowLabelText, &mut TextColor), (Without<RowValueText>,)>,
    mut row_values: Query<(&RowValueText, &mut Text, &mut TextColor), (Without<RowLabelText>,)>,
    mut slider_fills: Query<(&SliderFill, &mut Node), (Without<TabUnderline>, Without<TabPanel>, Without<SettingRow>)>,
) {
    if !state.dirty && !input_map.is_changed() && !settings.is_changed() {
        return;
    }
    state.dirty = false;

    let active_tab = state.active_tab;
    let selected_row = state.selected_row;

    // Update tab text colors
    for (tab_btn, children) in &tab_buttons {
        let is_active = tab_btn.0 == active_tab;
        for child in children.iter() {
            if let Ok(mut color) = tab_texts.get_mut(child) {
                *color = TextColor(if is_active { ACCENT_CYAN } else { HINT_COLOR });
            }
        }
    }

    // Update tab underlines
    for (underline, mut node, mut bg) in &mut tab_underlines {
        let is_active = underline.0 == active_tab;
        node.height = Val::Px(if is_active { 3.0 } else { 1.0 });
        *bg = BackgroundColor(if is_active { TAB_UNDERLINE_ACTIVE } else { TAB_UNDERLINE_INACTIVE });
    }

    // Show/hide tab panels
    for (panel, mut node) in &mut tab_panels {
        node.display = if panel.0 == active_tab { Display::Flex } else { Display::None };
    }

    // Update rows
    for (row_marker, mut bg, mut border) in &mut row_nodes {
        if row_marker.0 != active_tab {
            continue;
        }
        let is_selected = row_marker.1 == selected_row;
        *bg = BackgroundColor(if is_selected { SELECTED_BG } else { Color::NONE });
        *border = BorderColor::all(if is_selected { SELECTED_BORDER } else { PANEL_BORDER_DIM });
    }

    for (label, mut color) in &mut row_labels {
        if label.0 != active_tab {
            continue;
        }
        let is_selected = label.1 == selected_row;
        *color = TextColor(if is_selected { ACCENT_CYAN } else { TITLE_COLOR });
    }

    // Update value displays
    for (value_marker, mut text, mut color) in &mut row_values {
        let tab = value_marker.0;
        let idx = value_marker.1;
        let row_defs = rows_for_tab(tab);
        if idx >= row_defs.len() {
            continue;
        }
        let row_def = &row_defs[idx];

        match row_def.kind {
            RowKind::KeyBind(action) => {
                let is_listening = state.listening && tab == active_tab && idx == selected_row;
                if is_listening {
                    **text = "[PRESS KEY]".to_string();
                    *color = TextColor(LISTENING_COLOR);
                } else {
                    **text = binding_display_name(&input_map, &action);
                    *color = TextColor(HINT_COLOR);
                }
            }
            RowKind::Slider { .. } => {
                let val = get_slider_value(tab, idx, &settings);
                **text = format_slider_value(tab, idx, val);
                *color = TextColor(VALUE_COLOR);
            }
            RowKind::Offset { .. } => {
                let val = get_offset_value(tab, idx, &settings);
                let sign = if val >= 0 { "+" } else { "" };
                **text = format!("{sign}{val} ms");
                *color = TextColor(VALUE_COLOR);
            }
            RowKind::Toggle => {
                let on = get_toggle_value(tab, idx, &settings);
                **text = if on { "ON" } else { "OFF" }.to_string();
                *color = TextColor(if on { TOGGLE_ON } else { TOGGLE_OFF });
            }
            RowKind::ResetBindings | RowKind::NavAction => {}
        }
    }

    // Update slider fills
    for (fill, mut node) in &mut slider_fills {
        let tab = fill.0;
        let idx = fill.1;
        let row_defs = rows_for_tab(tab);
        if idx >= row_defs.len() {
            continue;
        }
        if let RowKind::Slider { min, max, .. } = row_defs[idx].kind {
            let val = get_slider_value(tab, idx, &settings);
            let pct = ((val - min) / (max - min) * 100.0).clamp(0.0, 100.0);
            node.width = Val::Percent(pct);
        }
    }
}

fn save_on_exit(settings: Res<GameSettings>) {
    settings.save();
}

/// Rebuild the default input map (mirrors `GameAction::default_input_map` in action.rs).
fn default_input_map() -> InputMap<GameAction> {
    use GameAction::*;
    let mut map = InputMap::default();

    map.insert(Tap, KeyCode::Space);
    map.insert(Tap, GamepadButton::South);

    map.insert(Confirm, KeyCode::Space);
    map.insert(Confirm, KeyCode::Enter);
    map.insert(Confirm, GamepadButton::South);

    map.insert(Back, KeyCode::Escape);
    map.insert(Back, GamepadButton::East);

    map.insert(Up, KeyCode::ArrowUp);
    map.insert(Down, KeyCode::ArrowDown);
    map.insert(Left, KeyCode::ArrowLeft);
    map.insert(Right, KeyCode::ArrowRight);

    map.insert(Up, GamepadButton::DPadUp);
    map.insert(Down, GamepadButton::DPadDown);
    map.insert(Left, GamepadButton::DPadLeft);
    map.insert(Right, GamepadButton::DPadRight);

    map.insert(Up, GamepadControlDirection::LEFT_UP);
    map.insert(Down, GamepadControlDirection::LEFT_DOWN);
    map.insert(Left, GamepadControlDirection::LEFT_LEFT);
    map.insert(Right, GamepadControlDirection::LEFT_RIGHT);

    map
}
