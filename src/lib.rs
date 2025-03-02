use bevy::{prelude::*, utils::HashMap};
use game_controller::{AppleGameController, AppleGameControllerEvent};

mod game_controller;

pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamepadIndex>();
        app.add_event::<AppleGameControllerEvent>();
        app.add_systems(First, AppleGameController::gamepad_connection_system);
        app.add_systems(
            PreUpdate,
            AppleGameController::poll_system.after(AppleGameController::gamepad_connection_system),
        );

        AppleGameController::setup(app);
    }
}

pub(crate) type GamepadId = usize;

#[derive(Resource, Default)]
pub(crate) struct GamepadIndex {
    gamepad: HashMap<GamepadId, Entity>,
}
