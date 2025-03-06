use bevy_app::App;
use bevy_input::gamepad::GamepadConnection;
use crossbeam::channel::Sender;

use crate::{GamepadId, error::GamepadError, profile::Changed};
mod apple;

pub use apple::AppleGameControllerPlatform;

/// Platform trait abstracts underlying platform gamepad interface
/// * Currently only supports Apple Game Controller framework in [`AppleGameControllerPlatform`]
pub trait Platform: Sized {
    /// The type of handle to access a gamepad in [`GamepadHandle`]
    type Handle;

    /// Initialize a new platform driver. Called during plugin initialization.
    /// The App struct is passed if the platform implementation needs to insert
    /// resources into the World.
    fn new(app: &mut App, tx: Sender<GamepadPlatformEvent>) -> Result<Self, GamepadError>;
}

#[derive(Debug)]
pub enum GamepadPlatformEvent {
    Error(GamepadError),
    Connected {
        id: GamepadId,
        connection: GamepadConnection,
    },

    Disconnected {
        id: GamepadId,
    },

    InputChanged {
        id: GamepadId,
        change: Changed,
    },
}
