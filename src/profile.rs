use bevy_input::gamepad::{GamepadAxis, GamepadButton};

pub trait Profile {}

/// A button change result returned from [`Profile`] mapped to bevy [`GamepadButton`]
#[derive(Debug)]
pub struct ButtonChange {
    button: GamepadButton,
    value: f32,
}

impl ButtonChange {
    pub fn new(button: GamepadButton, value: f32) -> Self {
        Self { button, value }
    }

    pub fn button(&self) -> GamepadButton {
        self.button
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}

#[derive(Debug)]
pub struct DPadChange {
    up: f32,
    down: f32,
    left: f32,
    right: f32,
}

impl DPadChange {
    pub fn new(up: f32, down: f32, left: f32, right: f32) -> Self {
        Self {
            up,
            down,
            left,
            right,
        }
    }

    pub fn up(&self) -> f32 {
        self.up
    }
    pub fn down(&self) -> f32 {
        self.down
    }
    pub fn left(&self) -> f32 {
        self.left
    }
    pub fn right(&self) -> f32 {
        self.right
    }
}

#[derive(Debug)]
pub enum Changed {
    Button(ButtonChange),
    DualAxis {
        x_axis: GamepadAxis,
        x_value: f32,
        y_axis: GamepadAxis,
        y_value: f32,
    },
    SingleAxis {
        axis: GamepadAxis,
        value: f32,
    },
    DPad(DPadChange),
}
