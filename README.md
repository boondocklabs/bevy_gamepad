# Apple Game Controller Framework Integration plugin for Bevy

This project provides integration for Apple Game Controllers with the Bevy game engine, as an alternative to the builtin Gilrs based gamepad interface.
It enables Bevy to detect, connect, and handle input events from Apple-compatible game controllers using `objc2` and `objc2_game_controller`.

Targets Bevy **0.20** and Rust **1.97.1** or newer on macOS. iOS support is
currently untested. Other platforms are not supported by this Apple framework backend.

## Supported Controllers

The Game Controller framework limits support of controllers to those supported by Apple.

### Known working controllers
- Nintendo Switch Pro Controller
- Nintendo Switch JoyCon  - Both left and right JoyCons must be paired and will show up as a single gamepad
- 8BitDo Ultimate Bluetooth (Switch Pro compatible)
- Xbox
- DualShock 4
- DualSense


## Features
- Detects gamepad connections and disconnections using `Notification Center framework`
- Supports multiple gamepads
- Assigns `playerIndex` enabling LED player displays on controllers
- Maps gamepad buttons and axes to Bevy's input system
- Uses Bevy's event system to handle gamepad interactions
- Asyncronous change detection handled by GC framework
- Rumble through CoreHaptics


## Installation
To use this integration in your Bevy project, add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
bevy_gamepad = "0.1"
```

### Running alongside the internal gilrs plugin

Both backends can run together, but each creates its own `Gamepad` entities.
A controller detected by both can therefore appear twice. Filter by
`AppleGamepad` when you need this backend specifically, or disable gilrs to
avoid duplicate input. Keeping gilrs enabled can cover HID devices that
Apple's framework does not recognise.

Use `AppleRumbleRequest` for entities marked `AppleGamepad` and Bevy's
`GamepadRumbleRequest` for gilrs entities. In Bevy 0.20, gilrs reports an
unknown gamepad instead of panicking, but standard rumble requests still
do not drive this plugin's CoreHaptics backend.

If you would rather not run both, disable gilrs when building the plugin group:

```rust
DefaultPlugins.build().disable::<bevy::gilrs::GilrsPlugin>()
```

or drop bevy's `gilrs` feature entirely:

```toml
bevy = { version = "0.20", default-features = false, features = [...] }
```

## Usage
### Add the Plugin to Your Bevy App
```rust
use bevy::prelude::*;
use bevy_gamepad::GamepadPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the Gamepad Plugin
        .add_plugins(GamepadPlugin)
        .run();
}
```

### Rumble

Every gamepad this plugin creates carries an `AppleGamepad` marker, which is
also how an app running both backends tells them apart. Shake one by writing
an `AppleRumbleRequest`:

```rust
use bevy_gamepad::{AppleGamepad, AppleRumbleRequest};
use std::time::Duration;

fn rumble_on_hit(
    pads: Query<Entity, With<AppleGamepad>>,
    mut rumbles: MessageWriter<AppleRumbleRequest>,
) {
    for gamepad in pads.iter() {
        rumbles.write(AppleRumbleRequest::Add {
            gamepad,
            duration: Duration::from_millis(90),
            strong_motor: 0.7,
            weak_motor: 0.9,
        });
    }
}
```

Apple exposes no simple "set motor strength" call: rumble is CoreHaptics, so
this asks the controller's `GCDeviceHaptics` for an engine bound to a
*locality* (a physical actuator), builds a pattern and plays it. The two
motors map onto the two handles — the strong, low-frequency motor is the left
handle and the weak, high-frequency one the right. Engines are expensive to
build, so they are cached per gamepad for the life of the controller.

Controllers that expose no haptics are skipped silently; run with
`RUST_LOG=bevy_gamepad=debug` to see which.

## Development and CI

On macOS with Rust 1.97.1 or newer:

```sh
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
```

GitHub Actions runs these checks on macOS for pushes, pull requests, and
manual dispatches. Tests inject platform messages without controller hardware.
To check real connections, input, and CoreHaptics, run
`cargo run --locked --example viewer` with a supported controller; CI does not
validate physical devices or haptic output.
