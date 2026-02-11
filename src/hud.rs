use bevy::prelude::*;

use crate::GameSet;
use crate::scoring::{ChainTier, ScoreState};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, update_hud.in_set(GameSet::Render));
    }
}

// --- Y2K Future Punk HUD palette ---

/// Deep indigo-black backdrop — CRT monitor off-black
const HUD_BG: Color = Color::srgba(0.04, 0.02, 0.08, 0.85);
/// Inner panel — slightly elevated surface
const HUD_PANEL: Color = Color::srgba(0.08, 0.04, 0.14, 0.6);
/// Score digits — hot white with slight cyan cast
const SCORE_COLOR: Color = Color::srgb(0.92, 0.96, 1.0);
/// Label text — muted lavender
const LABEL_COLOR: Color = Color::srgb(0.55, 0.45, 0.65);
/// Chain: Normal tier — ghostly white
const CHAIN_NORMAL: Color = Color::srgb(0.8, 0.78, 0.82);
/// Chain: FEVER tier — electric cyan (matches COOL judgment)
const CHAIN_FEVER: Color = Color::srgb(0.0, 0.9, 1.0);
/// Chain: TRANCE tier — molten gold
const CHAIN_TRANCE: Color = Color::srgb(1.0, 0.85, 0.15);
/// GREAT count color — neon green
const GREAT_HUD: Color = Color::srgb(0.0, 1.0, 0.4);
/// COOL count color — cyan
const COOL_HUD: Color = Color::srgb(0.0, 0.7, 1.0);
/// GOOD count color — amber
const GOOD_HUD: Color = Color::srgb(1.0, 0.85, 0.0);
/// MISS count color — magenta-red
const MISS_HUD: Color = Color::srgb(1.0, 0.15, 0.3);
/// Accent border — neon purple edge glow
const ACCENT_BORDER: Color = Color::srgb(0.6, 0.2, 1.0);

// --- Font sizes ---

const SCORE_FONT: f32 = 42.0;
const CHAIN_FONT: f32 = 28.0;
const CHAIN_LABEL_FONT: f32 = 13.0;
const GRADE_COUNT_FONT: f32 = 18.0;
const GRADE_LABEL_FONT: f32 = 11.0;

// --- Marker components ---

#[derive(Component)]
struct HudScoreText;

#[derive(Component)]
struct HudChainText;

#[derive(Component)]
struct HudChainLabel;

#[derive(Component)]
struct HudGreatCount;

#[derive(Component)]
struct HudCoolCount;

#[derive(Component)]
struct HudGoodCount;

#[derive(Component)]
struct HudMissCount;

// --- Systems ---

fn setup_hud(mut commands: Commands) {
    // Root container — top-right corner, absolute positioned
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                right: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(HUD_BG),
            BorderColor::all(ACCENT_BORDER),
        ))
        .with_children(|root: &mut ChildSpawnerCommands| {
            // --- Score display ---
            root.spawn((
                Text::new("0"),
                TextFont {
                    font_size: SCORE_FONT,
                    ..default()
                },
                TextColor(SCORE_COLOR),
                TextLayout {
                    justify: Justify::Right,
                    ..default()
                },
                HudScoreText,
            ));

            // --- Chain section ---
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(2.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ))
            .with_children(|chain_section: &mut ChildSpawnerCommands| {
                // Chain label
                chain_section.spawn((
                    Text::new("CHAIN"),
                    TextFont {
                        font_size: CHAIN_LABEL_FONT,
                        ..default()
                    },
                    TextColor(LABEL_COLOR),
                    HudChainLabel,
                ));

                // Chain value
                chain_section.spawn((
                    Text::new("0"),
                    TextFont {
                        font_size: CHAIN_FONT,
                        ..default()
                    },
                    TextColor(CHAIN_NORMAL),
                    HudChainText,
                ));
            });

            // --- Divider line (thin accent bar) ---
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(ACCENT_BORDER.with_alpha(0.4)),
            ));

            // --- Grade distribution panel ---
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(HUD_PANEL),
            ))
            .with_children(|grades: &mut ChildSpawnerCommands| {
                spawn_grade_row(grades, "GREAT", GREAT_HUD, HudGreatCount);
                spawn_grade_row(grades, "COOL", COOL_HUD, HudCoolCount);
                spawn_grade_row(grades, "GOOD", GOOD_HUD, HudGoodCount);
                spawn_grade_row(grades, "MISS", MISS_HUD, HudMissCount);
            });
        });
}

fn spawn_grade_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    color: Color,
    marker: impl Component,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|row: &mut ChildSpawnerCommands| {
            // Label
            row.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: GRADE_LABEL_FONT,
                    ..default()
                },
                TextColor(color.with_alpha(0.7)),
            ));

            // Count value
            row.spawn((
                Text::new("0"),
                TextFont {
                    font_size: GRADE_COUNT_FONT,
                    ..default()
                },
                TextColor(color),
                marker,
            ));
        });
}

fn update_hud(
    state: Option<Res<ScoreState>>,
    mut score_q: Query<&mut Text, (With<HudScoreText>, Without<HudChainText>)>,
    mut chain_q: Query<
        (&mut Text, &mut TextColor),
        (With<HudChainText>, Without<HudScoreText>, Without<HudChainLabel>),
    >,
    mut chain_label_q: Query<
        &mut TextColor,
        (With<HudChainLabel>, Without<HudChainText>, Without<HudScoreText>),
    >,
    mut great_q: Query<&mut Text, (With<HudGreatCount>, Without<HudScoreText>, Without<HudChainText>)>,
    mut cool_q: Query<&mut Text, (With<HudCoolCount>, Without<HudScoreText>, Without<HudChainText>, Without<HudGreatCount>)>,
    mut good_q: Query<&mut Text, (With<HudGoodCount>, Without<HudScoreText>, Without<HudChainText>, Without<HudGreatCount>, Without<HudCoolCount>)>,
    mut miss_q: Query<&mut Text, (With<HudMissCount>, Without<HudScoreText>, Without<HudChainText>, Without<HudGreatCount>, Without<HudCoolCount>, Without<HudGoodCount>)>,
) {
    let Some(state) = state else { return };

    // Score
    if let Ok(mut text) = score_q.single_mut() {
        **text = format!("{}", state.score);
    }

    // Chain + tier color
    if let Ok((mut text, mut color)) = chain_q.single_mut() {
        **text = format!("{}", state.chain);
        let tier_color = match state.chain_tier() {
            ChainTier::Normal => CHAIN_NORMAL,
            ChainTier::Fever => CHAIN_FEVER,
            ChainTier::Trance => CHAIN_TRANCE,
        };
        *color = TextColor(tier_color);
    }

    // Chain label also gets tier color
    if let Ok(mut color) = chain_label_q.single_mut() {
        let tier_color = match state.chain_tier() {
            ChainTier::Normal => LABEL_COLOR,
            ChainTier::Fever => CHAIN_FEVER.with_alpha(0.6),
            ChainTier::Trance => CHAIN_TRANCE.with_alpha(0.6),
        };
        *color = TextColor(tier_color);
    }

    // Grade counts
    if let Ok(mut text) = great_q.single_mut() {
        **text = format!("{}", state.great_count);
    }
    if let Ok(mut text) = cool_q.single_mut() {
        **text = format!("{}", state.cool_count);
    }
    if let Ok(mut text) = good_q.single_mut() {
        **text = format!("{}", state.good_count);
    }
    if let Ok(mut text) = miss_q.single_mut() {
        **text = format!("{}", state.miss_count);
    }
}
