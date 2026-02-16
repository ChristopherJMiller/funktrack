use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::GameAction;
use crate::audio::KiraContext;
use crate::state::GameScreen;

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            detect_pause.run_if(in_state(GameScreen::Playing)),
        )
        .add_systems(OnEnter(GameScreen::Paused), (pause_audio, spawn_pause_ui))
        .add_systems(
            Update,
            handle_pause_input.run_if(in_state(GameScreen::Paused)),
        )
        .add_systems(OnExit(GameScreen::Paused), resume_audio);
    }
}

// --- Y2K Future Punk pause palette ---

const BACKDROP: Color = Color::srgba(0.02, 0.01, 0.06, 0.75);
const PANEL_BG: Color = Color::srgba(0.06, 0.03, 0.12, 0.95);
const PANEL_BORDER: Color = Color::srgb(0.6, 0.2, 1.0);
const TEXT_PRIMARY: Color = Color::srgb(0.92, 0.96, 1.0);
const TEXT_MUTED: Color = Color::srgb(0.4, 0.35, 0.5);
const BUTTON_BG: Color = Color::srgba(0.12, 0.06, 0.2, 0.8);
const BUTTON_HOVER: Color = Color::srgba(0.2, 0.1, 0.35, 0.9);
const BUTTON_BORDER: Color = Color::srgb(0.5, 0.2, 0.8);

// --- Marker components ---

#[derive(Component)]
struct PauseResumeButton;

#[derive(Component)]
struct PauseQuitButton;

// --- Systems ---

fn detect_pause(
    action: Res<ActionState<GameAction>>,
    mut next_state: ResMut<NextState<GameScreen>>,
) {
    if action.just_pressed(&GameAction::Back) {
        info!("Pausing game");
        next_state.set(GameScreen::Paused);
    }
}

fn pause_audio(mut ctx: NonSendMut<KiraContext>) {
    if let Some(ref mut clock) = ctx.clock {
        clock.pause();
    }
    if let Some(ref mut sound) = ctx.sound {
        let _ = sound.pause(Default::default());
    }
}

fn resume_audio(
    mut ctx: NonSendMut<KiraContext>,
    state: Res<State<GameScreen>>,
    next_state: Res<NextState<GameScreen>>,
) {
    // Only resume audio if we're going back to Playing (not quitting)
    let going_to_playing = match next_state.as_ref() {
        NextState::Unchanged => *state.get() == GameScreen::Playing,
        NextState::Pending(s) | NextState::PendingIfNeq(s) => {
            *s == GameScreen::Playing
        }
    };
    if !going_to_playing {
        return;
    }

    if let Some(ref mut clock) = ctx.clock {
        clock.start();
    }
    if let Some(ref mut sound) = ctx.sound {
        let _ = sound.resume(Default::default());
    }
}

fn spawn_pause_ui(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(GameScreen::Paused),
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
            backdrop
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect {
                            top: Val::Px(32.0),
                            bottom: Val::Px(28.0),
                            left: Val::Px(48.0),
                            right: Val::Px(48.0),
                        },
                        row_gap: Val::Px(24.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        min_width: Val::Px(280.0),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                    BorderColor::all(PANEL_BORDER),
                ))
                .with_children(|panel: &mut ChildSpawnerCommands| {
                    // PAUSED header
                    panel.spawn((
                        Text::new("PAUSED"),
                        TextFont {
                            font_size: 48.0,
                            ..default()
                        },
                        TextColor(TEXT_PRIMARY),
                    ));

                    // Buttons container
                    panel
                        .spawn((Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(12.0),
                            ..default()
                        },))
                        .with_children(|buttons: &mut ChildSpawnerCommands| {
                            spawn_pause_button(
                                buttons,
                                "RESUME",
                                PauseResumeButton,
                            );
                            spawn_pause_button(
                                buttons,
                                "QUIT TO MENU",
                                PauseQuitButton,
                            );
                        });

                    // Hint
                    panel.spawn((
                        Text::new("[ESC] Resume  /  [A / SPACE] Select"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(TEXT_MUTED),
                    ));
                });
        });
}

fn spawn_pause_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
) {
    parent
        .spawn((
            marker,
            Button,
            Node {
                padding: UiRect {
                    top: Val::Px(10.0),
                    bottom: Val::Px(10.0),
                    left: Val::Px(32.0),
                    right: Val::Px(32.0),
                },
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                min_width: Val::Px(200.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BUTTON_BG),
            BorderColor::all(BUTTON_BORDER),
        ))
        .with_children(|btn: &mut ChildSpawnerCommands| {
            btn.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(TEXT_PRIMARY),
            ));
        });
}

fn handle_pause_input(
    action: Res<ActionState<GameAction>>,
    mut next_state: ResMut<NextState<GameScreen>>,
    resume_q: Query<&Interaction, (Changed<Interaction>, With<PauseResumeButton>)>,
    quit_q: Query<&Interaction, (Changed<Interaction>, With<PauseQuitButton>)>,
    mut button_bgs: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Or<(With<PauseResumeButton>, With<PauseQuitButton>)>,
            Changed<Interaction>,
        ),
    >,
) {
    // Keyboard/gamepad: Back (Escape/East) resumes
    if action.just_pressed(&GameAction::Back) {
        info!("Resuming game");
        next_state.set(GameScreen::Playing);
        return;
    }

    // Mouse hover effects
    for (interaction, mut bg) in &mut button_bgs {
        match interaction {
            Interaction::Hovered => *bg = BackgroundColor(BUTTON_HOVER),
            _ => *bg = BackgroundColor(BUTTON_BG),
        }
    }

    // Resume button click
    for interaction in &resume_q {
        if *interaction == Interaction::Pressed {
            info!("Resuming game");
            next_state.set(GameScreen::Playing);
            return;
        }
    }

    // Quit button click
    for interaction in &quit_q {
        if *interaction == Interaction::Pressed {
            info!("Quitting to song select");
            next_state.set(GameScreen::Results);
            return;
        }
    }
}
