use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<GameAction>::default())
            .init_resource::<ActionState<GameAction>>()
            .insert_resource(GameAction::default_input_map());
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum GameAction {
    Tap,
    Confirm,
    Back,
    Up,
    Down,
    Left,
    Right,
}

impl GameAction {
    fn default_input_map() -> InputMap<Self> {
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
}
