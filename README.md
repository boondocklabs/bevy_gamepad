# Apple Game Controller Framework Integration plugin for Bevy

> **Fork note.** This is a fork of [boondocklabs/bevy_gamepad](https://github.com/boondocklabs/bevy_gamepad)
> (MIT), updated for **Bevy 0.19** and fixed for controllers that were already
> paired before the app launched. Changes over upstream 0.1.3:
>
> - **Bevy 0.15 → 0.19**: buffered events became messages (`EventWriter` →
>   `MessageWriter`, `send` → `write`), `bevy_utils::HashMap` moved to
>   `bevy_platform::collections`, and `bevy_input`'s gamepad module is now
>   behind its `gamepad` feature.
> - **Adopts already-connected controllers.** `GCControllerDidConnectNotification`
>   only fires for pads connected *after* launch, and `GCController::controllers()`
>   is empty until the run loop has turned over — so a pad paired beforehand was
>   never picked up at all. A per-frame reconcile pass now adopts any controller
>   whose player index is still unset.
> - **Identifies controllers by `productCategory`, not just `vendorName`.**
>   `vendorName` is whatever the manufacturer chose: a Switch-layout 8BitDo
>   reports `"8Bitdo 2"`, matched no arm, and silently fell back to a layout with
>   the face buttons in the wrong places. `productCategory` is Apple's normalised
>   classification (`"Switch Pro Controller"`).
> - **No panics across the FFI boundary.** Channel sends inside Objective-C
>   callbacks no longer `unwrap()`; unwinding out of a callback is UB, and the
>   receiver is legitimately gone during shutdown.
> - **One bad event no longer drops the rest.** `platform_system` used `return`
>   where it meant `continue`, discarding every input still queued behind an
>   event for an unknown gamepad.
>
> Verified on macOS 26 with an 8BitDo pad in Switch Pro mode — the exact case
> where `gilrs` reads nothing, because it consumes OS-*parsed* HID elements and
> a Switch pad's report descriptor does not describe the reports it sends.
>
> **Note:** this must *replace* `bevy_gilrs`, not accompany it. `bevy_gilrs`'
> rumble system unwraps the gilrs id of whatever gamepad a rumble targets and
> panics on entities created by another backend. Disable it with
> `DefaultPlugins.build().disable::<bevy::gilrs::GilrsPlugin>()`.


This project provides integration for Apple Game Controllers with the Bevy game engine, as an alternative to the builtin Gilrs based gamepad interface.
It enables Bevy to detect, connect, and handle input events from Apple-compatible game controllers using `objc2` and `objc2_game_controller`.

This should also work on iOS devices, but is currently untested.

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


## Installation
To use this integration in your Bevy project, add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
bevy_gamepad = "0.2"
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
