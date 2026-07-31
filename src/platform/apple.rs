mod profiles;

use bevy_app::App;
use bevy_input::gamepad::GamepadConnection;
use bevy_log::{debug, error, info, trace, warn};
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
    ApplePlatformProfile, DualSenseProfile, DualShockProfile, GenericProfile, SwitchProfile,
    XboxProfile,
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
                        // Never panic inside an Objective-C callback: unwinding
                        // across the FFI boundary is undefined behaviour, and the
                        // receiver is legitimately gone during shutdown.
                        let _ = tx.send(GamepadPlatformEvent::InputChanged { id, change });
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
                        Err(gamepad) => Self::select_vendor_profile(gamepad),
                    },
                },
            },
        }
    }

    /// Selects a profile for controllers which expose no dedicated interface.
    ///
    /// `productCategory` is checked before `vendorName` because it is Apple's
    /// *normalised* classification ("Switch Pro Controller"), whereas the
    /// vendor name is whatever the manufacturer chose — a Switch-layout
    /// 8BitDo reports "8Bitdo 2", matching no vendor arm and silently
    /// falling back to a layout with the face buttons in the wrong places.
    fn select_vendor_profile(
        gamepad: Retained<GCExtendedGamepad>,
    ) -> Box<dyn ApplePlatformProfile> {
        let (category, vendor_name) = unsafe {
            match gamepad
                .device()
                .and_then(|d| d.downcast::<GCController>().ok())
            {
                Some(controller) => (
                    Some(controller.productCategory().to_string()),
                    controller.vendorName().map(|name| name.to_string()),
                ),
                None => {
                    warn!("Downcast to GCController failed");
                    (None, None)
                }
            }
        };

        if let Some(category) = &category {
            debug!(category, "Matching gamepad from product category");
            if category.contains("Switch Pro") || category.contains("Joy-Con") {
                return Box::new(SwitchProfile(gamepad));
            }
        }

        match vendor_name.as_deref() {
            Some("Pro Controller") | Some("Joy-Con (L/R)") => Box::new(SwitchProfile(gamepad)),
            Some(name) => {
                debug!(name, "No specific profile; using the generic layout");
                Box::new(GenericProfile(gamepad))
            }
            None => {
                warn!("No vendor name or category from GCController; using the generic layout.");
                Box::new(GenericProfile(gamepad))
            }
        }
    }

    /// Adopt a controller: assign it a player index, wire up its change
    /// handler, and announce it. Shared by the connect notification and the
    /// reconcile pass so both paths behave identically.
    fn attach_controller(controller: &GCController, tx: &Sender<GamepadPlatformEvent>) {
        let Some(gamepad) = (unsafe { controller.extendedGamepad() }) else {
            let _ = tx.send(GamepadPlatformEvent::Error(GamepadError::Platform(
                "Failed to get GCExtendedGamepad from GCController".into(),
            )));
            return;
        };

        unsafe {
            // Mark unassigned first so this controller is excluded when
            // scanning for the lowest free index.
            controller.setPlayerIndex(GCControllerPlayerIndex(-1));
            controller.setPlayerIndex(Self::get_next_player_index());
        }

        let id = unsafe { controller.playerIndex() }.0 as GamepadId;
        Self::init_gamepad(id, tx.clone(), gamepad);

        let vendor_name = unsafe { controller.vendorName() }
            .map(|name| name.to_string())
            .unwrap_or_else(|| String::from("Unknown Apple Gamepad"));

        info!(name = vendor_name, index = id, "Connected");

        let _ = tx.send(GamepadPlatformEvent::Connected {
            id,
            connection: GamepadConnection::Connected {
                name: format!("{vendor_name} {id}"),
                vendor_id: None,
                product_id: None,
            },
        });
    }

    /// Adopt any controller the framework knows about that we have not taken
    /// yet, identified by its player index still being unset.
    ///
    /// This is what makes a pad that was *already paired before launch* work.
    /// `GCControllerDidConnectNotification` only fires for controllers that
    /// connect afterwards, and `GCController::controllers()` is empty until
    /// the run loop has turned over — so a one-shot scan at plugin build time
    /// finds nothing. Reconciling every frame is cheap and covers both.
    fn attach_new_controllers(tx: &Sender<GamepadPlatformEvent>) {
        for controller in unsafe { GCController::controllers() }.iter() {
            if unsafe { controller.playerIndex() }.0 >= 0 {
                continue;
            }
            Self::attach_controller(&controller, tx);
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

                        Self::attach_controller(controller, &tx);
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

    fn new(app: &mut App, tx: Sender<GamepadPlatformEvent>) -> Result<Self, GamepadError> {
        // Get the default notification center
        let notification_center = NSNotificationCenter::defaultCenter();

        let driver = Self {
            notification_center,
        };

        // Pick up controllers that were already paired when the app launched;
        // those never raise a connect notification.
        let scan_tx = tx.clone();
        app.add_systems(bevy_app::PreUpdate, move || {
            Self::attach_new_controllers(&scan_tx);
        });

        // Register gamepad connection/disconnection notifications with notification center
        driver.register_notifications(tx.clone())?;

        unsafe {
            GCController::setShouldMonitorBackgroundEvents(true);
            GCController::startWirelessControllerDiscoveryWithCompletionHandler(None);
        }

        Ok(driver)
    }
}
