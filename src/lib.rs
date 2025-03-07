use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{
    GamepadButton, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy_platform_support::collections::HashMap;
use crossbeam::channel::{Receiver, unbounded};
use platform::{AppleGameControllerPlatform, GamepadPlatformEvent, Platform as _};

mod error;
mod platform;
mod profile;

pub struct GamepadPlugin;

#[derive(Resource)]
struct GamepadPlatformHandler {
    /// Receive channel to receive events from platform drivers
    rx: Receiver<GamepadPlatformEvent>,

    /// Index of gamepad player index to bevy [`Gamepad`] entity
    index: HashMap<GamepadId, Entity>,
}

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, platform_system);

        let (tx, rx) = unbounded();

        app.insert_resource(GamepadPlatformHandler {
            rx,
            index: HashMap::default(),
        });

        AppleGameControllerPlatform::new(app, tx).unwrap();
    }
}

pub(crate) type GamepadId = usize;

fn platform_system(
    mut commands: Commands,
    mut handler: ResMut<GamepadPlatformHandler>,
    mut connection_writer: EventWriter<GamepadConnectionEvent>,
    mut gamepad_events: EventWriter<RawGamepadEvent>,
) {
    while let Ok(event) = handler.rx.try_recv() {
        match event {
            // Handle any errors sent over the channel from the platform driver
            GamepadPlatformEvent::Error(gamepad_error) => {
                bevy_log::error!("{gamepad_error}");
            }

            GamepadPlatformEvent::Connected { id, connection } => {
                let entity = commands.spawn_empty().id();
                handler.index.insert(id, entity);
                connection_writer.write(GamepadConnectionEvent {
                    gamepad: entity,
                    connection,
                });
            }

            GamepadPlatformEvent::Disconnected { id } => {
                if let Some(entity) = handler.index.get(&id) {
                    connection_writer.write(GamepadConnectionEvent {
                        gamepad: *entity,
                        connection: bevy_input::gamepad::GamepadConnection::Disconnected,
                    });
                }
            }

            GamepadPlatformEvent::InputChanged { id, change } => {
                let Some(gamepad) = handler.index.get(&id) else {
                    return;
                };

                match change {
                    profile::Changed::Button(button_change) => {
                        let event = RawGamepadButtonChangedEvent {
                            gamepad: *gamepad,
                            button: button_change.button(),
                            value: button_change.value(),
                        };

                        gamepad_events.write(RawGamepadEvent::Button(event));
                    }

                    profile::Changed::DPad(dpad_change) => {
                        let up_event = RawGamepadButtonChangedEvent {
                            gamepad: *gamepad,
                            button: GamepadButton::DPadUp,
                            value: dpad_change.up(),
                        };
                        let down_event = RawGamepadButtonChangedEvent {
                            gamepad: *gamepad,
                            button: GamepadButton::DPadDown,
                            value: dpad_change.down(),
                        };
                        let left_event = RawGamepadButtonChangedEvent {
                            gamepad: *gamepad,
                            button: GamepadButton::DPadLeft,
                            value: dpad_change.left(),
                        };
                        let right_event = RawGamepadButtonChangedEvent {
                            gamepad: *gamepad,
                            button: GamepadButton::DPadRight,
                            value: dpad_change.right(),
                        };

                        gamepad_events.write(RawGamepadEvent::Button(up_event));
                        gamepad_events.write(RawGamepadEvent::Button(down_event));
                        gamepad_events.write(RawGamepadEvent::Button(left_event));
                        gamepad_events.write(RawGamepadEvent::Button(right_event));
                    }

                    profile::Changed::DualAxis {
                        x_axis,
                        x_value,
                        y_axis,
                        y_value,
                    } => {
                        let x_event = RawGamepadAxisChangedEvent {
                            gamepad: *gamepad,
                            axis: x_axis,
                            value: x_value,
                        };

                        let y_event = RawGamepadAxisChangedEvent {
                            gamepad: *gamepad,
                            axis: y_axis,
                            value: y_value,
                        };

                        gamepad_events.write(RawGamepadEvent::Axis(x_event));
                        gamepad_events.write(RawGamepadEvent::Axis(y_event));
                    }

                    profile::Changed::SingleAxis { axis, value } => {
                        let event = RawGamepadAxisChangedEvent {
                            gamepad: *gamepad,
                            axis,
                            value,
                        };

                        gamepad_events.write(RawGamepadEvent::Axis(event));
                    }
                }
            }
        }
    }
}
