# Changelog

## 0.2.3

### Fixed

- Build API documentation on docs.rs for Apple Silicon instead of Linux.

## 0.2.2

### Added

- Add tvOS support for Apple Game Controller input. tvOS applications need a Bevy build with tvOS support.
- Add Siri Remote support through the micro gamepad profile. Touch surface directions map to the D-pad, click to South, Play/Pause to West, and Menu to Start.
- Mark Siri Remote entities with `AppleRemote` so games can choose remote-specific controls and prompts.
