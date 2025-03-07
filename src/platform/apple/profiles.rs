use bevy_input::gamepad::{GamepadAxis, GamepadButton};
use objc2::rc::Retained;
use objc2_game_controller::{
    GCControllerButtonInput, GCControllerDirectionPad, GCControllerElement,
    GCDualSenseAdaptiveTrigger, GCDualSenseGamepad, GCDualShockGamepad, GCExtendedGamepad,
    GCXboxGamepad,
};

use crate::profile::{ButtonChange, Changed, DPadChange, Profile};

pub(crate) trait ApplePlatformProfile: Profile {
    /// Called from the change observer callback registered on the gamepad
    /// The default implementation will call the trait impl methods `button_changed()` and `axis_changed()`
    /// which are implemented on each gamepad profile to handle mappings to Bevy [`GamepadButton`]
    fn element_changed(&self, event: &GCControllerElement) -> Option<Changed> {
        if let Some(button) = event.downcast_ref::<GCControllerButtonInput>() {
            self.button_changed(button).map(Changed::Button)
        } else if let Some(axis) = event.downcast_ref::<GCControllerDirectionPad>() {
            self.axis_changed(axis)
        } else {
            None
        }
    }

    fn button_changed(&self, button: &GCControllerButtonInput) -> Option<ButtonChange>;
    fn axis_changed(&self, axis: &GCControllerDirectionPad) -> Option<Changed>;
}

pub struct DualSenseProfile(pub Retained<GCDualSenseGamepad>);
impl Profile for DualSenseProfile {}

impl ApplePlatformProfile for DualSenseProfile {
    fn button_changed(&self, button: &GCControllerButtonInput) -> Option<ButtonChange> {
        unsafe {
            if button == &*self.0.buttonA() {
                return Some(ButtonChange::new(GamepadButton::South, button.value()));
            }
            if button == &*self.0.buttonB() {
                return Some(ButtonChange::new(GamepadButton::East, button.value()));
            }
            if button == &*self.0.buttonX() {
                return Some(ButtonChange::new(GamepadButton::West, button.value()));
            }
            if button == &*self.0.buttonY() {
                return Some(ButtonChange::new(GamepadButton::North, button.value()));
            }
            if button == &*self.0.buttonMenu() {
                return Some(ButtonChange::new(GamepadButton::Start, button.value()));
            }
            if let Some(options) = self.0.buttonOptions() {
                if button == &*options {
                    return Some(ButtonChange::new(GamepadButton::Select, button.value()));
                }
            }
            if let Some(left_thumb) = self.0.leftThumbstickButton() {
                if button == &*left_thumb {
                    return Some(ButtonChange::new(GamepadButton::LeftThumb, button.value()));
                }
            }
            if let Some(right_thumb) = self.0.rightThumbstickButton() {
                if button == &*right_thumb {
                    return Some(ButtonChange::new(GamepadButton::RightThumb, button.value()));
                }
            }

            if button == &*self.0.rightShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger,
                    button.value(),
                ));
            }
            if button == &*self.0.leftShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger,
                    button.value(),
                ));
            }

            if button == &*self.0.touchpadButton() {
                return Some(ButtonChange::new(GamepadButton::C, button.value()));
            }

            if let Some(trigger) = button.downcast_ref::<GCDualSenseAdaptiveTrigger>() {
                if trigger == &*self.0.rightTrigger() {
                    return Some(ButtonChange::new(
                        GamepadButton::RightTrigger2,
                        button.value(),
                    ));
                }
                if trigger == &*self.0.leftTrigger() {
                    return Some(ButtonChange::new(
                        GamepadButton::LeftTrigger2,
                        button.value(),
                    ));
                }
            }
        }

        None
    }

    fn axis_changed(&self, axis: &GCControllerDirectionPad) -> Option<Changed> {
        unsafe {
            if axis == &*self.0.leftThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::LeftStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::LeftStickY,
                    y_value: axis.yAxis().value(),
                });
            }
            if axis == &*self.0.rightThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::RightStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::RightStickY,
                    y_value: axis.yAxis().value(),
                });
            }

            if axis == &*self.0.dpad() {
                return Some(Changed::DPad(DPadChange::new(
                    axis.up().value(),
                    axis.down().value(),
                    axis.left().value(),
                    axis.right().value(),
                )));
            }

            if let Some(trigger) = axis.downcast_ref::<GCDualSenseAdaptiveTrigger>() {
                if trigger == &*self.0.rightTrigger() {
                    return Some(Changed::SingleAxis {
                        axis: GamepadAxis::RightZ,
                        value: trigger.value(),
                    });
                }

                if trigger == &*self.0.leftTrigger() {
                    return Some(Changed::SingleAxis {
                        axis: GamepadAxis::LeftZ,
                        value: trigger.value(),
                    });
                }
            }

            None
        }
    }
}

pub struct DualShockProfile(pub Retained<GCDualShockGamepad>);
impl Profile for DualShockProfile {}

impl ApplePlatformProfile for DualShockProfile {
    fn button_changed(&self, button: &GCControllerButtonInput) -> Option<ButtonChange> {
        unsafe {
            if button == &*self.0.buttonA() {
                return Some(ButtonChange::new(GamepadButton::South, button.value()));
            }
            if button == &*self.0.buttonB() {
                return Some(ButtonChange::new(GamepadButton::East, button.value()));
            }
            if button == &*self.0.buttonX() {
                return Some(ButtonChange::new(GamepadButton::West, button.value()));
            }
            if button == &*self.0.buttonY() {
                return Some(ButtonChange::new(GamepadButton::North, button.value()));
            }
            if button == &*self.0.buttonMenu() {
                return Some(ButtonChange::new(GamepadButton::Start, button.value()));
            }
            if let Some(options) = self.0.buttonOptions() {
                if button == &*options {
                    return Some(ButtonChange::new(GamepadButton::Select, button.value()));
                }
            }
            if let Some(left_thumb) = self.0.leftThumbstickButton() {
                if button == &*left_thumb {
                    return Some(ButtonChange::new(GamepadButton::LeftThumb, button.value()));
                }
            }
            if let Some(right_thumb) = self.0.rightThumbstickButton() {
                if button == &*right_thumb {
                    return Some(ButtonChange::new(GamepadButton::RightThumb, button.value()));
                }
            }
            if button == &*self.0.rightShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger,
                    button.value(),
                ));
            }
            if button == &*self.0.leftShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger,
                    button.value(),
                ));
            }

            if let Some(touchpad_button) = self.0.touchpadButton() {
                if button == &*touchpad_button {
                    return Some(ButtonChange::new(GamepadButton::C, button.value()));
                }
            }

            if button == &*self.0.rightTrigger() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger2,
                    button.value(),
                ));
            }
            if button == &*self.0.leftTrigger() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger2,
                    button.value(),
                ));
            }
        }
        None
    }

    fn axis_changed(&self, axis: &GCControllerDirectionPad) -> Option<Changed> {
        unsafe {
            if axis == &*self.0.leftThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::LeftStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::LeftStickY,
                    y_value: axis.yAxis().value(),
                });
            }
            if axis == &*self.0.rightThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::RightStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::RightStickY,
                    y_value: axis.yAxis().value(),
                });
            }

            if axis == &*self.0.dpad() {
                return Some(Changed::DPad(DPadChange::new(
                    axis.up().value(),
                    axis.down().value(),
                    axis.left().value(),
                    axis.right().value(),
                )));
            }

            None
        }
    }
}

pub struct XboxProfile(pub Retained<GCXboxGamepad>);
impl Profile for XboxProfile {}

impl ApplePlatformProfile for XboxProfile {
    fn button_changed(&self, button: &GCControllerButtonInput) -> Option<ButtonChange> {
        unsafe {
            if button == &*self.0.buttonA() {
                return Some(ButtonChange::new(GamepadButton::South, button.value()));
            }
            if button == &*self.0.buttonB() {
                return Some(ButtonChange::new(GamepadButton::East, button.value()));
            }
            if button == &*self.0.buttonX() {
                return Some(ButtonChange::new(GamepadButton::West, button.value()));
            }
            if button == &*self.0.buttonY() {
                return Some(ButtonChange::new(GamepadButton::North, button.value()));
            }
            if button == &*self.0.buttonMenu() {
                return Some(ButtonChange::new(GamepadButton::Start, button.value()));
            }
            if let Some(options) = self.0.buttonOptions() {
                if button == &*options {
                    return Some(ButtonChange::new(GamepadButton::Select, button.value()));
                }
            }
            if let Some(left) = self.0.leftThumbstickButton() {
                if button == &*left {
                    return Some(ButtonChange::new(GamepadButton::LeftThumb, button.value()));
                }
            }
            if let Some(right) = self.0.rightThumbstickButton() {
                if button == &*right {
                    return Some(ButtonChange::new(GamepadButton::RightThumb, button.value()));
                }
            }
            if button == &*self.0.rightShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger,
                    button.value(),
                ));
            }
            if button == &*self.0.leftShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger,
                    button.value(),
                ));
            }

            if button == &*self.0.rightTrigger() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger2,
                    button.value(),
                ));
            }
            if button == &*self.0.leftTrigger() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger2,
                    button.value(),
                ));
            }
        }
        None
    }

    fn axis_changed(&self, axis: &GCControllerDirectionPad) -> Option<Changed> {
        unsafe {
            if axis == &*self.0.leftThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::LeftStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::LeftStickY,
                    y_value: axis.yAxis().value(),
                });
            }
            if axis == &*self.0.rightThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::RightStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::RightStickY,
                    y_value: axis.yAxis().value(),
                });
            }

            if axis == &*self.0.dpad() {
                return Some(Changed::DPad(DPadChange::new(
                    axis.up().value(),
                    axis.down().value(),
                    axis.left().value(),
                    axis.right().value(),
                )));
            }
            None
        }
    }
}

pub struct GenericProfile(pub Retained<GCExtendedGamepad>);
impl Profile for GenericProfile {}

impl ApplePlatformProfile for GenericProfile {
    fn button_changed(&self, button: &GCControllerButtonInput) -> Option<ButtonChange> {
        unsafe {
            if button == &*self.0.buttonA() {
                return Some(ButtonChange::new(GamepadButton::East, button.value()));
            }
            if button == &*self.0.buttonB() {
                return Some(ButtonChange::new(GamepadButton::South, button.value()));
            }
            if button == &*self.0.buttonX() {
                return Some(ButtonChange::new(GamepadButton::North, button.value()));
            }
            if button == &*self.0.buttonY() {
                return Some(ButtonChange::new(GamepadButton::West, button.value()));
            }
            if button == &*self.0.buttonMenu() {
                return Some(ButtonChange::new(GamepadButton::Start, button.value()));
            }
            if let Some(options) = self.0.buttonOptions() {
                if button == &*options {
                    return Some(ButtonChange::new(GamepadButton::Select, button.value()));
                }
            }
            if let Some(left_thumb) = self.0.leftThumbstickButton() {
                if button == &*left_thumb {
                    return Some(ButtonChange::new(GamepadButton::LeftThumb, button.value()));
                }
            }
            if let Some(right_thumb) = self.0.rightThumbstickButton() {
                if button == &*right_thumb {
                    return Some(ButtonChange::new(GamepadButton::RightThumb, button.value()));
                }
            }
            if button == &*self.0.rightShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger,
                    button.value(),
                ));
            }
            if button == &*self.0.leftShoulder() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger,
                    button.value(),
                ));
            }

            if button == &*self.0.rightTrigger() {
                return Some(ButtonChange::new(
                    GamepadButton::RightTrigger2,
                    button.value(),
                ));
            }
            if button == &*self.0.leftTrigger() {
                return Some(ButtonChange::new(
                    GamepadButton::LeftTrigger2,
                    button.value(),
                ));
            }

            if let Some(name) = button.localizedName() {
                if name.to_string() == "Share Button" {
                    return Some(ButtonChange::new(GamepadButton::C, button.value()));
                }
            }
        }
        None
    }

    fn axis_changed(&self, axis: &GCControllerDirectionPad) -> Option<Changed> {
        unsafe {
            if axis == &*self.0.leftThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::LeftStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::LeftStickY,
                    y_value: axis.yAxis().value(),
                });
            }
            if axis == &*self.0.rightThumbstick() {
                return Some(Changed::DualAxis {
                    x_axis: GamepadAxis::RightStickX,
                    x_value: axis.xAxis().value(),
                    y_axis: GamepadAxis::RightStickY,
                    y_value: axis.yAxis().value(),
                });
            }
            if axis == &*self.0.dpad() {
                return Some(Changed::DPad(DPadChange::new(
                    axis.up().value(),
                    axis.down().value(),
                    axis.left().value(),
                    axis.right().value(),
                )));
            }
            None
        }
    }
}
