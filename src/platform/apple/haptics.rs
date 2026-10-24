//! Rumble for controllers supplied by Apple's GameController framework.
//!
//! Apple exposes no simple "set motor strength" call. Rumble is CoreHaptics:
//! you ask the controller's [`GCDeviceHaptics`] for an engine bound to a
//! *locality* (a physical actuator), build a pattern, and play it. Bevy's two
//! motors map onto the two handles — the strong, low-frequency motor is the
//! left handle and the weak one the right.
//!
//! # Backend-specific rumble
//!
//! [`bevy_input::gamepad::GamepadRumbleRequest`] targets Bevy's gilrs backend.
//! Bevy 0.20 handles unknown gilrs entities without panicking, but does not
//! route those requests to CoreHaptics. Keep [`AppleRumbleRequest`] for this
//! backend so applications can select the appropriate rumble implementation.

use bevy_ecs::prelude::*;
use bevy_log::{debug, warn};
use bevy_platform::collections::HashMap;
use objc2::rc::Retained;
use objc2::{AnyThread, msg_send};
use objc2_core_haptics::{
    CHHapticEngine, CHHapticEvent, CHHapticEventParameter, CHHapticEventParameterIDHapticIntensity,
    CHHapticEventTypeHapticContinuous, CHHapticPattern, CHHapticPatternPlayer,
};
use objc2_foundation::NSArray;
use objc2_game_controller::{
    GCController, GCDeviceHaptics, GCHapticsLocalityLeftHandle, GCHapticsLocalityRightHandle,
};
use std::time::Duration;

use crate::{AppleGamepad, GamepadPlatformHandler};

/// Shake a gamepad supplied by this plugin.
///
/// Mirrors [`bevy_input::gamepad::GamepadRumbleRequest`]; see the module note
/// for why it is a separate type.
#[derive(Message, Debug, Clone, Copy)]
pub enum AppleRumbleRequest {
    /// Play both motors for `duration`. Intensities are 0..=1.
    Add {
        gamepad: Entity,
        duration: Duration,
        /// Low-frequency motor, played on the left handle.
        strong_motor: f32,
        /// High-frequency motor, played on the right handle.
        weak_motor: f32,
    },
    /// Silence a gamepad immediately.
    Stop { gamepad: Entity },
}

impl AppleRumbleRequest {
    pub fn gamepad(&self) -> Entity {
        match self {
            Self::Add { gamepad, .. } | Self::Stop { gamepad } => *gamepad,
        }
    }
}

/// One engine per actuator. Engines are expensive to build and are kept for
/// the life of the controller rather than rebuilt per shake.
struct Actuators {
    strong: Option<Retained<CHHapticEngine>>,
    weak: Option<Retained<CHHapticEngine>>,
}

/// Live haptic engines, keyed by gamepad entity.
///
/// A **non-send** resource: `CHHapticEngine` is an Objective-C object and is
/// not `Send`, so Bevy keeps this (and the system touching it) on the main
/// thread — which is also where CoreHaptics wants to be driven from.
#[derive(Default)]
pub(crate) struct HapticEngines {
    engines: HashMap<Entity, Actuators>,
}

impl HapticEngines {
    /// The engines for `gamepad`, built on first use.
    fn actuators(&mut self, gamepad: Entity, controller: &GCController) -> &Actuators {
        self.engines.entry(gamepad).or_insert_with(|| {
            let haptics: Option<Retained<GCDeviceHaptics>> = unsafe { controller.haptics() };
            let Some(haptics) = haptics else {
                debug!("gamepad {gamepad} exposes no haptics");
                return Actuators {
                    strong: None,
                    weak: None,
                };
            };
            let engine = |locality: &objc2_game_controller::GCHapticsLocality| {
                // Sent by hand: objc2-game-controller 0.3.2 gates
                // `createEngineWithLocality:` to iOS/tvOS/visionOS, but
                // Apple's own header says `API_AVAILABLE(macos(11.0), ...)`,
                // so the generated binding is missing on the one platform
                // this crate targets. The root cause was `objc2-core-haptics`
                // being marked macOS-unavailable, which Apple's docs state
                // incorrectly. Fixed upstream in objc2 9d7f436 (issue #853),
                // landing in frameworks v0.4 — drop this and call the
                // generated method once that releases.
                let engine: Option<Retained<CHHapticEngine>> =
                    unsafe { msg_send![&*haptics, createEngineWithLocality: locality] };
                let engine = engine?;
                // An engine must be started before it will play anything.
                if let Err(error) = unsafe { engine.startAndReturnError() } {
                    warn!("could not start haptic engine: {error:?}");
                    return None;
                }
                Some(engine)
            };
            Actuators {
                strong: engine(unsafe { GCHapticsLocalityLeftHandle }),
                weak: engine(unsafe { GCHapticsLocalityRightHandle }),
            }
        })
    }
}

/// Build and play a single continuous buzz on one actuator.
fn play(engine: &CHHapticEngine, intensity: f32, seconds: f64) {
    if intensity <= 0.0 {
        return;
    }
    unsafe {
        let parameter = CHHapticEventParameter::initWithParameterID_value(
            CHHapticEventParameter::alloc(),
            CHHapticEventParameterIDHapticIntensity,
            intensity.clamp(0.0, 1.0),
        );
        let event = CHHapticEvent::initWithEventType_parameters_relativeTime_duration(
            CHHapticEvent::alloc(),
            CHHapticEventTypeHapticContinuous,
            &NSArray::from_retained_slice(&[parameter]),
            0.0,
            seconds,
        );
        let pattern = match CHHapticPattern::initWithEvents_parameters_error(
            CHHapticPattern::alloc(),
            &NSArray::from_retained_slice(&[event]),
            &NSArray::new(),
        ) {
            Ok(pattern) => pattern,
            Err(error) => return warn!("could not build haptic pattern: {error:?}"),
        };
        let player = match engine.createPlayerWithPattern_error(&pattern) {
            Ok(player) => player,
            Err(error) => return warn!("could not create haptic player: {error:?}"),
        };
        if let Err(error) = player.startAtTime_error(0.0) {
            warn!("could not start haptic player: {error:?}");
        }
    }
}

/// Find the live `GCController` behind a gamepad entity.
fn controller_for(
    handler: &GamepadPlatformHandler,
    gamepad: Entity,
) -> Option<Retained<GCController>> {
    let id = handler
        .index
        .iter()
        .find(|(_, entity)| **entity == gamepad)
        .map(|(id, _)| *id)?;
    unsafe { GCController::controllers() }
        .iter()
        .find(|controller| unsafe { controller.playerIndex() }.0 as usize == id)
}

/// Drives [`AppleRumbleRequest`]s onto the controller's actuators.
pub(crate) fn drive_rumble(
    mut engines: NonSendMut<HapticEngines>,
    mut requests: MessageReader<AppleRumbleRequest>,
    handler: Res<GamepadPlatformHandler>,
    ours: Query<(), With<AppleGamepad>>,
) {
    for request in requests.read() {
        let gamepad = request.gamepad();
        // Only pads this plugin supplies; anything else belongs to gilrs.
        if ours.get(gamepad).is_err() {
            continue;
        }
        let Some(controller) = controller_for(&handler, gamepad) else {
            debug!("no live controller behind gamepad {gamepad}");
            continue;
        };
        match *request {
            AppleRumbleRequest::Add {
                duration,
                strong_motor,
                weak_motor,
                ..
            } => {
                let seconds = duration.as_secs_f64();
                let actuators = engines.actuators(gamepad, &controller);
                debug!(
                    "rumbling {gamepad} for {seconds:.2}s                      (strong {strong_motor:.2}{}, weak {weak_motor:.2}{})",
                    if actuators.strong.is_some() {
                        ""
                    } else {
                        " [no engine]"
                    },
                    if actuators.weak.is_some() {
                        ""
                    } else {
                        " [no engine]"
                    },
                );
                if let Some(engine) = &actuators.strong {
                    play(engine, strong_motor, seconds);
                }
                if let Some(engine) = &actuators.weak {
                    play(engine, weak_motor, seconds);
                }
            }
            // A pattern already carries its own duration, so stopping is only
            // needed to cut one short — dropping the players does that.
            AppleRumbleRequest::Stop { .. } => {
                engines.engines.remove(&gamepad);
            }
        }
    }
}
