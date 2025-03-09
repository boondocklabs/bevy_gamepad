use bevy::{color::palettes::tailwind::*, prelude::*, window::PrimaryWindow};
use bevy_gamepad::GamepadPlugin;
use bevy_input::gamepad::GamepadConnectionEvent;
use bevy_utils::HashMap;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GamepadPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update_layout)
        .add_systems(Update, update_text)
        .add_systems(Update, update)
        .run();
}

#[derive(Component)]
struct Container;

#[derive(Component, Debug)]
struct GamepadContainer {
    container: Entity,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            margin: UiRect::top(Val::Px(5.0)),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceEvenly,
            ..Default::default()
        },
        Container,
    ));
    commands.spawn(Text::default());
}

fn update_layout(
    mut commands: Commands,
    node: Single<Entity, With<Container>>,
    mut events: EventReader<GamepadConnectionEvent>,

    // Map of Gamepad Entity to column Node Entity for despawning when a gamepad is removed
    mut map: Local<HashMap<Entity, Entity>>,
) {
    for event in events.read() {
        match &event.connection {
            bevy_input::gamepad::GamepadConnection::Connected { name, .. } => {
                let mut content = None;
                commands.entity(*node).with_children(|container| {
                    let column = container
                        .spawn((Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },))
                        .with_children(|node| {
                            // Spawn a title
                            node.spawn((
                                Text::new(format!("{name}")),
                                TextFont {
                                    font_size: 14.0,
                                    ..Default::default()
                                },
                            ));

                            content = Some(
                                node.spawn((
                                    Text::default(),
                                    TextFont {
                                        font_size: 12.0,
                                        ..Default::default()
                                    },
                                ))
                                .id(),
                            );
                        })
                        .id();

                    // Insert the container for despawning
                    map.insert(event.gamepad, column);
                });

                // Insert GamepadContainer to the Gamepad bundle
                commands.entity(event.gamepad).insert(GamepadContainer {
                    container: content.unwrap(),
                });
            }
            bevy_input::gamepad::GamepadConnection::Disconnected => {
                if let Some(content) = map.remove(&event.gamepad) {
                    commands.entity(content).despawn_recursive();
                }
            }
        }
    }
}

fn update_text(mut commands: Commands, query: Query<(&Gamepad, &GamepadContainer)>) {
    for (gamepad, container) in query.iter() {
        if let Some(mut container) = commands.get_entity(container.container) {
            let mut text = format!(
                "DPad: {}\nLeft {}\nRight {}\n",
                gamepad.dpad(),
                gamepad.left_stick(),
                gamepad.right_stick(),
            );
            for axis in gamepad.get_analog_axes() {
                if let Some(value) = gamepad.get(*axis) {
                    text.push_str(&format!("{axis:?}: {:.3}\n", value));
                }
            }

            for button in gamepad.get_pressed() {
                text.push_str(&format!("{button:?}\n"));
            }
            container.insert(Text::new(text));
        }
    }
}

fn update(
    query: Query<&Gamepad>,
    mut gizmos: Gizmos,
    window: Single<&Window, With<PrimaryWindow>>,
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
    }
}
