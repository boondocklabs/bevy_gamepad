mod profiles;

use bevy_app::App;
use bevy_input::gamepad::GamepadConnection;
use bevy_log::{error, info, trace, warn};
use block2::StackBlock;
use crossbeam::channel::Sender;
use objc2::rc::Retained;
use objc2_foundation::{NSNotification, NSNotificationCenter};
use objc2_game_controller::{
    GCController, GCControllerDidConnectNotification, GCControllerDidDisconnectNotification,
    GCControllerPlayerIndex, GCDevice, GCDualSenseGamepad, GCDualShockGamepad, GCExtendedGamepad,
    GCMicroGamepad, GCXboxGamepad,
};
use profiles::{
    ApplePlatformProfile, DualSenseProfile, DualShockProfile, GenericProfile, XboxProfile,
};
use std::{ptr::NonNull, sync::Arc};

use super::{GamepadPlatformEvent, Platform};
use crate::{GamepadId, error::GamepadError};

pub struct AppleGameControllerPlatform {
    /// Apple Notification Center
    notification_center: Retained<NSNotificationCenter>,
}

impl AppleGameControllerPlatform {
    fn register_notifications(&self, tx: Sender<GamepadPlatformEvent>) -> Result<(), GamepadError> {
        self.register_connect_notification(tx.clone());
        self.register_disconnect_notification(tx);

        Ok(())
    }

    fn get_next_player_index() -> GCControllerPlayerIndex {
        unsafe {
            let mut players = GCController::controllers()
                .iter()
                .map(|controller| controller.playerIndex().0)
                .collect::<Vec<isize>>();

            // Ignore player index < 0 to filter unassigned controllers
            players.retain(|i| *i >= 0);
            players.sort();

            // See if there's a gap in the indexes
            for i in 0..players.len() {
                if players[i] as usize != i {
                    return GCControllerPlayerIndex(i as isize);
                }
            }

            // Otherwise return the next index
            GCControllerPlayerIndex(players.len() as isize)
        }
    }

    fn init_gamepad(
        id: GamepadId,
        tx: Sender<GamepadPlatformEvent>,
        gamepad: Retained<GCExtendedGamepad>,
    ) {
        // Create a profile for a specific type of connected gamepad.
        // The profiles wrap the GCDualSenseGamepad, GCXboxGamepad etc to handle the mappings
        // back to bevy GamepadButton/GamepadAxis types
        let profile = Arc::new(Self::get_gamepad_profile(gamepad.clone()));

        // Setup a change handler on the gamepad
        unsafe {
            let value_changed = StackBlock::new(
                move |gamepad: NonNull<GCExtendedGamepad>,
                      event: NonNull<objc2_game_controller::GCControllerElement>| {
                    let _gamepad = gamepad.as_ref();
                    let event = event.as_ref();

                    trace!(?event, "Change event");

                    if let Some(change) = profile.element_changed(event) {
                        tx.send(GamepadPlatformEvent::InputChanged { id, change })
                            .unwrap();
                    } else {
                        warn!(?event, "Unhandled change event in gamepad platform driver");
                    }
                },
            );

            gamepad.setValueChangedHandler(&*value_changed as *const _ as *mut _);
        }
    }

    fn get_gamepad_profile(gamepad: Retained<GCExtendedGamepad>) -> Box<dyn ApplePlatformProfile> {
        match gamepad.downcast::<GCDualSenseGamepad>() {
            Ok(gamepad) => Box::new(DualSenseProfile(gamepad)),
            Err(gamepad) => match gamepad.downcast::<GCDualShockGamepad>() {
                Ok(gamepad) => Box::new(DualShockProfile(gamepad)),
                Err(gamepad) => match gamepad.downcast::<GCXboxGamepad>() {
                    Ok(gamepad) => Box::new(XboxProfile(gamepad)),
                    Err(gamepad) => match gamepad.downcast::<GCMicroGamepad>() {
                        Ok(_) => todo!(),
                        Err(gamepad) => Box::new(GenericProfile(gamepad)),
                    },
                },
            },
        }
    }

    fn register_connect_notification(&self, tx: Sender<GamepadPlatformEvent>) {
        unsafe {
            self.notification_center
                .addObserverForName_object_queue_usingBlock(
                    Some(GCControllerDidConnectNotification),
                    None,
                    None,
                    &StackBlock::new(move |notification: NonNull<NSNotification>| {
                        let Some(object) = notification.as_ref().object() else {
                            if let Err(e) =
                                tx.send(GamepadPlatformEvent::Error(GamepadError::Platform(
                                    "Failed to get object from NSNotification".into(),
                                )))
                            {
                                error!("Failed to send to controller event channel: {e}");
                            }
                            return;
                        };

                        let Some(controller) = object.downcast_ref::<GCController>() else {
                            if let Err(e) =
                                tx.send(GamepadPlatformEvent::Error(GamepadError::Platform(
                                    "Failed to downcast to GCController from NSNotification object"
                                        .into(),
                                )))
                            {
                                error!("Failed to send to controller event channel: {e}");
                            }
                            return;
                        };

                        let Some(gamepad) = controller.extendedGamepad() else {
                            if let Err(e) =
                                tx.send(GamepadPlatformEvent::Error(GamepadError::Platform(
                                    "Failed to get GCExtendedGamepad from GCController".into(),
                                )))
                            {
                                error!("Failed to send to controller event channel: {e}");
                            }
                            return;
                        };

                        // Set the player index to -1 to mark it as an unassigned player
                        // This is to filter the controller when finding a new player index
                        controller.setPlayerIndex(GCControllerPlayerIndex(-1));

                        // Now find the next player index by scanning existing controllers
                        controller.setPlayerIndex(Self::get_next_player_index());

                        Self::init_gamepad(
                            controller.playerIndex().0 as usize,
                            tx.clone(),
                            gamepad,
                        );

                        let vendor_name = controller
                            .vendorName()
                            .map(|name| name.to_string())
                            .unwrap_or(String::from("Unknown Apple Gamepad"));

                        info!(
                            name = vendor_name,
                            index = controller.playerIndex().0,
                            "Connected"
                        );

                        tx.send(GamepadPlatformEvent::Connected {
                            id: controller.playerIndex().0 as usize,
                            connection: GamepadConnection::Connected {
                                name: format!(
                                    "{vendor_name} {}",
                                    controller.playerIndex().0 as GamepadId
                                ),
                                vendor_id: None,
                                product_id: None,
                            },
                        })
                        .unwrap();
                    }),
                );
        }
    }

    fn register_disconnect_notification(&self, tx: Sender<GamepadPlatformEvent>) {
        unsafe {
            self.notification_center
                .addObserverForName_object_queue_usingBlock(
                    Some(GCControllerDidDisconnectNotification),
                    None,
                    None,
                    &StackBlock::new(move |notification: NonNull<NSNotification>| {
                        let Some(object) = notification.as_ref().object() else {
                            return;
                        };

                        if let Some(controller) = object.downcast_ref::<GCController>() {
                            let id = controller.playerIndex().0 as GamepadId;

                            let vendor_name = controller
                                .vendorName()
                                .map(|name| name.to_string())
                                .unwrap_or(String::from("Unknown Apple Gamepad"));

                            info!(
                                name = vendor_name,
                                index = controller.playerIndex().0,
                                "Disconnected"
                            );

                            if let Err(e) = tx.send(GamepadPlatformEvent::Disconnected { id }) {
                                error!("Failed to send to controller event channel: {e}");
                            }
                        }
                    }),
                );
        }
    }
}

impl Platform for AppleGameControllerPlatform {
    type Handle = Retained<GCExtendedGamepad>;

    fn new(_app: &mut App, tx: Sender<GamepadPlatformEvent>) -> Result<Self, GamepadError> {
        // Get the default notification center
        let notification_center = unsafe { NSNotificationCenter::defaultCenter() };

        let driver = Self {
            notification_center,
        };

        // Register gamepad connection/disconnection notifications with notification center
        driver.register_notifications(tx)?;

        unsafe {
            GCController::setShouldMonitorBackgroundEvents(true);
            GCController::startWirelessControllerDiscoveryWithCompletionHandler(None);
        }

        Ok(driver)
    }
}
