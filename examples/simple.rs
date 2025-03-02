use bevy::{color::palettes::tailwind::*, prelude::*, window::PrimaryWindow};
use bevy_gamepad::GamepadPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GamepadPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn(Text::default());
}

fn update(
    query: Query<&Gamepad>,
    mut gizmos: Gizmos,
    window: Single<&Window, With<PrimaryWindow>>,
    mut text: Single<&mut Text>,
) {
    for pad in query.iter() {
        gizmos.circle_2d(
            Isometry2d::from_translation(pad.left_stick() * window.size() / 2.0),
            10.0,
            GREEN_500,
        );

        gizmos.circle_2d(
            Isometry2d::from_translation(pad.right_stick() * window.size() / 2.0),
            10.0,
            CYAN_500,
        );

        text.0 = format!("dpad: {}", pad.dpad());

        for pressed in pad.get_pressed() {
            text.push_str(&format!("\n{pressed:?}"));
        }
    }
}
