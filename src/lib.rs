use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{
    GamepadButton, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy_platform::collections::HashMap;
use crossbeam::channel::{Receiver, unbounded};
use platform::{AppleGameControllerPlatform, GamepadPlatformEvent, Platform as _};

mod error;
mod platform;
mod profile;

pub struct GamepadPlugin;

/// Marks a gamepad entity created by this plugin rather than by Bevy's
/// built-in gilrs backend.
///
/// Both backends can run side by side — gilrs still covers the plain-HID
/// devices Apple's framework does not recognise — which leaves apps needing
/// to tell them apart. Anything routed through gilrs (rumble, most notably)
/// must skip these entities: gilrs looks up its own id for the gamepad a
/// request names and will not find one.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AppleGamepad;

#[derive(Resource)]
pub(crate) struct GamepadPlatformHandler {
    /// Receive channel to receive events from platform drivers
    rx: Receiver<GamepadPlatformEvent>,

    /// Index of gamepad player index to Bevy [`bevy_input::gamepad::Gamepad`] entity
    pub(crate) index: HashMap<GamepadId, Entity>,
}

pub use platform::apple::haptics::AppleRumbleRequest;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, platform_system)
            .add_message::<AppleRumbleRequest>()
            .init_non_send::<platform::apple::haptics::HapticEngines>()
            .add_systems(PostUpdate, platform::apple::haptics::drive_rumble);

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
    mut connection_writer: MessageWriter<GamepadConnectionEvent>,
    mut gamepad_events: MessageWriter<RawGamepadEvent>,
) {
    while let Ok(event) = handler.rx.try_recv() {
        match event {
            // Handle any errors sent over the channel from the platform driver
            GamepadPlatformEvent::Error(gamepad_error) => {
                bevy_log::error!("{gamepad_error}");
            }

            GamepadPlatformEvent::Connected { id, connection } => {
                let entity = commands.spawn(AppleGamepad).id();
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
                // `continue`, not `return`: an event for an unknown gamepad
                // must skip only itself, or one stale id silently discards
                // every input still queued behind it this frame.
                let Some(gamepad) = handler.index.get(&id) else {
                    continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::message::Messages;
    use bevy_input::gamepad::{GamepadAxis, GamepadConnection};
    use crossbeam::channel::Sender;
    use profile::{ButtonChange, Changed, DPadChange};

    // Exercise the Bevy message bridge without registering Objective-C callbacks
    // or requiring a physical controller or window/run loop.
    fn bridge() -> (App, Sender<GamepadPlatformEvent>) {
        let (tx, rx) = unbounded();
        let mut app = App::new();
        app.add_message::<GamepadConnectionEvent>()
            .add_message::<RawGamepadEvent>()
            .insert_resource(GamepadPlatformHandler {
                rx,
                index: HashMap::default(),
            })
            .add_systems(Update, platform_system);
        (app, tx)
    }

    fn connect(tx: &Sender<GamepadPlatformEvent>, id: GamepadId) {
        tx.send(GamepadPlatformEvent::Connected {
            id,
            connection: GamepadConnection::Connected {
                name: "Test controller".into(),
                vendor_id: None,
                product_id: None,
            },
        })
        .unwrap();
    }

    #[test]
    fn connections_create_distinct_marked_entities_and_route_disconnects() {
        let (mut app, tx) = bridge();
        connect(&tx, 0);
        connect(&tx, 1);
        tx.send(GamepadPlatformEvent::Disconnected { id: 0 })
            .unwrap();
        tx.send(GamepadPlatformEvent::Disconnected { id: 99 })
            .unwrap();
        app.update();

        let handler = app.world().resource::<GamepadPlatformHandler>();
        let first = handler.index[&0];
        let second = handler.index[&1];
        assert_ne!(first, second);
        assert!(app.world().get::<AppleGamepad>(first).is_some());
        assert!(app.world().get::<AppleGamepad>(second).is_some());
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<GamepadConnectionEvent>>()
            .drain()
            .collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].gamepad, first);
        assert_eq!(events[1].gamepad, second);
        assert_eq!(events[2].gamepad, first);
        assert!(matches!(
            events[2].connection,
            GamepadConnection::Disconnected
        ));
    }

    #[test]
    fn unknown_input_does_not_discard_later_button_or_axis_messages() {
        let (mut app, tx) = bridge();
        connect(&tx, 7);
        for (id, change) in [
            (
                99,
                Changed::Button(ButtonChange::new(GamepadButton::South, 1.0)),
            ),
            (
                7,
                Changed::Button(ButtonChange::new(GamepadButton::South, 0.75)),
            ),
            (
                7,
                Changed::DualAxis {
                    x_axis: GamepadAxis::LeftStickX,
                    x_value: -0.5,
                    y_axis: GamepadAxis::LeftStickY,
                    y_value: 0.25,
                },
            ),
            (
                7,
                Changed::SingleAxis {
                    axis: GamepadAxis::RightStickX,
                    value: 0.5,
                },
            ),
        ] {
            tx.send(GamepadPlatformEvent::InputChanged { id, change })
                .unwrap();
        }
        app.update();
        let entity = app.world().resource::<GamepadPlatformHandler>().index[&7];
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<RawGamepadEvent>>()
            .drain()
            .collect();
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], RawGamepadEvent::Button(e)
            if e.gamepad == entity && e.button == GamepadButton::South && e.value == 0.75));
        for (event, axis, value) in [
            (&events[1], GamepadAxis::LeftStickX, -0.5),
            (&events[2], GamepadAxis::LeftStickY, 0.25),
            (&events[3], GamepadAxis::RightStickX, 0.5),
        ] {
            assert!(matches!(event, RawGamepadEvent::Axis(e)
                if e.gamepad == entity && e.axis == axis && e.value == value));
        }
    }

    #[test]
    fn bevy_input_plugin_consumes_bridged_controller_messages() {
        let (mut app, tx) = bridge();
        app.add_plugins(bevy_input::InputPlugin);
        connect(&tx, 0);
        for change in [
            Changed::Button(ButtonChange::new(GamepadButton::South, 1.0)),
            Changed::SingleAxis {
                axis: GamepadAxis::LeftStickX,
                value: 0.8,
            },
        ] {
            tx.send(GamepadPlatformEvent::InputChanged { id: 0, change })
                .unwrap();
        }
        // The test bridge runs in Update, after InputPlugin's PreUpdate systems.
        app.update();
        app.update();
        let entity = app.world().resource::<GamepadPlatformHandler>().index[&0];
        let gamepad = app
            .world()
            .get::<bevy_input::gamepad::Gamepad>(entity)
            .unwrap();
        assert!(gamepad.pressed(GamepadButton::South));
        assert!(gamepad.left_stick().x > 0.5);
        assert!(app.world().get::<AppleGamepad>(entity).is_some());
    }

    #[test]
    fn dpad_emits_all_four_buttons_including_releases() {
        let (mut app, tx) = bridge();
        connect(&tx, 0);
        tx.send(GamepadPlatformEvent::InputChanged {
            id: 0,
            change: Changed::DPad(DPadChange::new(1.0, 0.0, 0.5, 0.0)),
        })
        .unwrap();
        app.update();
        let entity = app.world().resource::<GamepadPlatformHandler>().index[&0];
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<RawGamepadEvent>>()
            .drain()
            .collect();
        assert_eq!(events.len(), 4);
        for (event, (button, value)) in events.iter().zip([
            (GamepadButton::DPadUp, 1.0),
            (GamepadButton::DPadDown, 0.0),
            (GamepadButton::DPadLeft, 0.5),
            (GamepadButton::DPadRight, 0.0),
        ]) {
            assert!(matches!(event, RawGamepadEvent::Button(e)
                if e.gamepad == entity && e.button == button && e.value == value));
        }
    }
}
