use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::{
    gamepad::{
        GamepadConnection, GamepadConnectionEvent, GamepadInput, RawGamepadAxisChangedEvent,
        RawGamepadButtonChangedEvent, RawGamepadEvent,
    },
    prelude::*,
};
use bevy_log::prelude::*;

use block2::StackBlock;
use crossbeam::channel::{Receiver, unbounded};
use objc2::rc::Retained;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectNSScriptClassDescription};
use objc2_game_controller::{
    GCController, GCControllerDidConnectNotification, GCControllerDidDisconnectNotification,
    GCControllerPlayerIndex, GCDevice, GCDevicePhysicalInput as _, GCExtendedGamepad,
};
use std::{
    ptr::NonNull,
    sync::{Arc, atomic::AtomicUsize},
};

use crate::{GamepadId, GamepadIndex};

#[derive(Event)]
pub(crate) enum AppleGameControllerEvent {
    Connected {
        id: GamepadId,
        connection: GamepadConnection,
    },

    Disconnected {
        id: GamepadId,
    },
}

#[derive(Resource)]
pub(crate) struct ConnectionEventChannel<T: Event> {
    rx: Receiver<T>,
}

pub struct AppleGameController;

impl AppleGameController {
    pub fn gamepad_connection_system(
        mut commands: Commands,
        channel: Res<ConnectionEventChannel<AppleGameControllerEvent>>,
        mut writer: EventWriter<GamepadConnectionEvent>,
        mut gamepad_index: ResMut<GamepadIndex>,
    ) {
        while let Ok(event) = channel.rx.try_recv() {
            match event {
                AppleGameControllerEvent::Connected { id, connection } => {
                    let entity = commands.spawn_empty().id();
                    gamepad_index.gamepad.insert(id, entity);
                    writer.send(GamepadConnectionEvent {
                        gamepad: entity,
                        connection,
                    });
                }
                AppleGameControllerEvent::Disconnected { id } => {
                    if let Some(entity) = gamepad_index.gamepad.remove(&id) {
                        writer.send(GamepadConnectionEvent {
                            gamepad: entity,
                            connection: GamepadConnection::Disconnected,
                        });
                    }
                }
            }
        }
    }

    unsafe fn button_value(pad: &Retained<GCExtendedGamepad>, button: &GamepadButton) -> f32 {
        unsafe {
            match button {
                GamepadButton::South => pad.buttonB().value(),
                GamepadButton::East => pad.buttonA().value(),
                GamepadButton::North => pad.buttonX().value(),
                GamepadButton::West => pad.buttonY().value(),
                GamepadButton::C => 0.0,
                GamepadButton::Z => 0.0,
                GamepadButton::LeftTrigger => pad.leftTrigger().value(),
                GamepadButton::LeftTrigger2 => pad.leftShoulder().value(),
                GamepadButton::RightTrigger => pad.rightTrigger().value(),
                GamepadButton::RightTrigger2 => pad.rightShoulder().value(),
                GamepadButton::Select => pad.buttonOptions().map(|b| b.value()).unwrap_or(0.0),
                GamepadButton::Start => pad.buttonMenu().value(),
                GamepadButton::Mode => pad.buttonHome().map(|b| b.value()).unwrap_or(0.0),
                GamepadButton::LeftThumb => pad.leftThumbstickButton().unwrap().value(),
                GamepadButton::RightThumb => pad.rightThumbstickButton().unwrap().value(),
                GamepadButton::DPadUp => pad.dpad().up().value(),
                GamepadButton::DPadDown => pad.dpad().down().value(),
                GamepadButton::DPadLeft => pad.dpad().left().value(),
                GamepadButton::DPadRight => pad.dpad().right().value(),
                GamepadButton::Other(other) => unimplemented!("Other button {other}"),
            }
        }
    }

    unsafe fn axis_value(pad: &Retained<GCExtendedGamepad>, axis: &GamepadAxis) -> f32 {
        unsafe {
            match axis {
                GamepadAxis::LeftStickX => pad.leftThumbstick().xAxis().value(),
                GamepadAxis::LeftStickY => pad.leftThumbstick().yAxis().value(),
                GamepadAxis::LeftZ => pad.leftShoulder().value(),
                GamepadAxis::RightStickX => pad.rightThumbstick().xAxis().value(),
                GamepadAxis::RightStickY => pad.rightThumbstick().yAxis().value(),
                GamepadAxis::RightZ => pad.rightShoulder().value(),
                GamepadAxis::Other(other) => unimplemented!("Other axis {other}"),
            }
        }
    }

    pub fn poll_system(
        gamepad_index: Res<GamepadIndex>,
        query: Query<&Gamepad>,
        mut events: EventWriter<RawGamepadEvent>,
    ) {
        unsafe {
            for controller in GCController::controllers() {
                if let Some(_state) = controller.input().nextInputState() {
                    // Get the ID of this controller
                    let id = controller.playerIndex().0 as GamepadId;

                    let Some(pad) = controller.extendedGamepad() else {
                        return;
                    };

                    // Lookup the GamepadId in the index to get Entity of the Gamepad component
                    if let Some(gamepad_entity) = gamepad_index.gamepad.get(&id) {
                        match query.get(*gamepad_entity) {
                            Ok(gamepad) => {
                                // Iterate each axis, and get the current value from the bevy Gamepad Component
                                // which will be compared against the new GCControllerInputState.
                                //
                                // Emit gamepad events for any axis which differs
                                for axis in gamepad.get_analog_axes() {
                                    if let Some(value) = gamepad.get(*axis) {
                                        match axis {
                                            GamepadInput::Axis(gamepad_axis) => {
                                                let new_value =
                                                    Self::axis_value(&pad, gamepad_axis);

                                                if new_value != value {
                                                    let axis_event = RawGamepadAxisChangedEvent {
                                                        gamepad: *gamepad_entity,
                                                        axis: *gamepad_axis,
                                                        value: new_value,
                                                    };

                                                    events.send(RawGamepadEvent::Axis(axis_event));
                                                }
                                            }
                                            GamepadInput::Button(gamepad_button) => {
                                                let new_value =
                                                    Self::button_value(&pad, gamepad_button);

                                                if new_value != value {
                                                    let button_event =
                                                        RawGamepadButtonChangedEvent {
                                                            gamepad: *gamepad_entity,
                                                            button: *gamepad_button,
                                                            value: new_value,
                                                        };

                                                    events.send(RawGamepadEvent::Button(
                                                        button_event,
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_e) => {
                                // Events may arrive before the Gamepad component is spawned.
                                // For now, just silently ignore the events
                                //error!(id, "Failed to get Gamepad component. {e}");
                            }
                        }
                    } else {
                        error!(
                            id,
                            "Received GCControllerInputState but controller does not exist in GamepadIndex resource"
                        );
                    }
                }
            }
        }
    }

    /// This must be called from the main thread or it will crash.
    pub fn setup(app: &mut App) {
        let (tx, rx) = unbounded();

        app.insert_resource(ConnectionEventChannel { rx });

        // Atomic to track the next controller ID. This will be assigned as the playerIndex of the Controller
        // and incremented in the connection callbacks. IDs are not reused on disconnect/reconnect
        // and may have an effect on controllers with LED displays that indicate the player number.
        let next_controller_id = Arc::new(AtomicUsize::new(0));

        unsafe {
            #[cfg(debug_assertions)]
            let value_changed = StackBlock::new(
                |gamepad: NonNull<GCExtendedGamepad>,
                 event: NonNull<objc2_game_controller::GCControllerElement>| {
                    let gamepad = gamepad.as_ref();
                    let event = event.as_ref();

                    debug!("Value changed callback {:?} {:?}", gamepad, event);
                },
            );

            #[cfg(debug_assertions)]
            let input_changed = StackBlock::new(
                |device: NonNull<
                    objc2::runtime::ProtocolObject<
                        dyn objc2_game_controller::GCDevicePhysicalInput,
                    >,
                >,
                 element: NonNull<
                    objc2::runtime::ProtocolObject<
                        dyn objc2_game_controller::GCPhysicalInputElement,
                    >,
                >| {
                    debug!(
                        "Input changed {:#?} {:#?}",
                        device.as_ref(),
                        element.as_ref()
                    );
                },
            );

            let connect_tx = tx.clone();
            let notification_center = NSNotificationCenter::defaultCenter();
            notification_center.addObserverForName_object_queue_usingBlock(
                Some(GCControllerDidConnectNotification),
                None,
                None,
                &StackBlock::new(move |notification: NonNull<NSNotification>| {
                    let Some(object) = notification.as_ref().object() else {
                        return;
                    };

                    if let Some(controller) = object.downcast_ref::<GCController>() {
                        // On debug builds, register a callback to log element value changes
                        #[cfg(debug_assertions)]
                        {
                            // Regester a change handler block on GCControllerLiveInput
                            let input = controller.input();
                            input.setElementValueDidChangeHandler(Some(&input_changed));

                            controller
                                .extendedGamepad()
                                .unwrap()
                                .setValueChangedHandler(&*value_changed as *const _ as *mut _);
                        }

                        let class_name = controller.className().to_string();
                        let vendor_name = controller
                            .vendorName()
                            .map(|name| name.to_string())
                            .unwrap_or(String::from("Unknown Gamepad"));

                        let next_index =
                            next_controller_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                        controller.setPlayerIndex(GCControllerPlayerIndex(next_index as isize));

                        let event = AppleGameControllerEvent::Connected {
                            id: controller.playerIndex().0 as GamepadId,
                            connection: GamepadConnection::Connected {
                                name: format!(
                                    "{class_name} {vendor_name} {}",
                                    controller.playerIndex().0 as GamepadId
                                ),
                                vendor_id: None,
                                product_id: None,
                            },
                        };

                        if let Err(e) = connect_tx.send(event) {
                            error!("Failed to send to controller event channel: {e}");
                        }
                    }
                }),
            );

            let disconnect_tx = tx.clone();
            notification_center.addObserverForName_object_queue_usingBlock(
                Some(GCControllerDidDisconnectNotification),
                None,
                None,
                &StackBlock::new(move |notification: NonNull<NSNotification>| {
                    let Some(object) = notification.as_ref().object() else {
                        return;
                    };

                    if let Some(controller) = object.downcast_ref::<GCController>() {
                        let id = controller.playerIndex().0 as GamepadId;
                        if let Err(e) =
                            disconnect_tx.send(AppleGameControllerEvent::Disconnected { id })
                        {
                            error!("Failed to send to controller event channel: {e}");
                        }
                    }
                }),
            );

            GCController::setShouldMonitorBackgroundEvents(true);
            GCController::startWirelessControllerDiscoveryWithCompletionHandler(None);
        }
    }
}
