# Apple Game Controller Framework Integration plugin for Bevy

This project provides integration for Apple Game Controllers with the Bevy game engine, as an alternative to the builtin Gilrs based gamepad interface.
It enables Bevy to detect, connect, and handle input events from Apple-compatible game controllers using `objc2` and `objc2_game_controller`.

This should also work on iOS devices, but is currently untested.

## Supported Controllers

The Game Controller framework limits support of controllers.
Apple silicon devices do not support wired USB gamepads, thus these are all over bluetooth connections.

- Nintendo Switch Pro Controller (tested)
- Nintendo Switch JoyCon
- 8BitDo Ultimate Bluetooth (Switch Pro compatible, tested)
- XBox
- DualShock 4
- DualSense


## Features
- Detects gamepad connections and disconnections using `Notification Center framework`
- Supports multiple gamepads
- Assigns `playerIndex` enabling LED player displays on controllers
- Maps gamepad buttons and axes to Bevy's input system
- Uses Bevy's event system to handle gamepad interactions


## Installation
To use this integration in your Bevy project, add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
bevy_gamepad = 0
```

### Disable internal Gilrs Gamepad plugin

Since gilrs is included by default, you either need to remove it from the features, or manually define the set of plugins loaded
during application setup rather than using `DefaultPlugins` which includes gilrs. This could be conditional depending on the build target,
only enabling the plugin when targeting MacOS/iOS.

The internal gilrs plugin can be disabled by setting `default-features = false` when importing `bevy`
into your project, and setting all required features explicitly without the `gilrs` feature flag.

```
bevy = { git = "https://github.com/bevyengine/bevy", version = "0.16.0-dev", default-features = false, features = [...]
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
