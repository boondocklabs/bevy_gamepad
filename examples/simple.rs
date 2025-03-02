use bevy::{prelude::*, window::PrimaryWindow};
use bevy_gamepad::GamepadPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GamepadPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, gizmos)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn gizmos(
    query: Query<&Gamepad>,
    mut gizmos: Gizmos,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    for pad in query.iter() {
        gizmos.circle_2d(
            Isometry2d::from_translation(pad.left_stick() * window.size() / 2.0),
            10.0,
            Color::WHITE,
        );
    }
}

/*
fn gamepad_input(gamepad: Query<&Gamepad, Changed<Gamepad>>, mut gizmos: Gizmos) {
    for pad in gamepad {
        for (axis, value) in pad.analog().all_axes_and_values() {
            //info!(?axis, ?value);
        }
    }
}
*/
