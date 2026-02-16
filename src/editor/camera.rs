use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use super::EditorState;

const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 5.0;

/// Handles camera zooming (scroll wheel) in the editor viewport.
pub fn editor_camera_system(
    state: Res<EditorState>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut camera_q: Query<&mut Projection>,
) {
    if state.egui_wants_pointer {
        scroll_events.clear();
        return;
    }

    let Ok(mut projection) = camera_q.single_mut() else {
        return;
    };

    for event in scroll_events.read() {
        let delta = match event.unit {
            MouseScrollUnit::Line => event.y * ZOOM_SPEED,
            MouseScrollUnit::Pixel => event.y * ZOOM_SPEED * 0.01,
        };

        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = (ortho.scale - delta).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}

/// Convert a screen position to world coordinates using the camera.
pub fn screen_to_world(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    screen_pos: Vec2,
) -> Option<Vec2> {
    camera
        .viewport_to_world_2d(camera_transform, screen_pos)
        .ok()
}
