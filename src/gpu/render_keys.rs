//! Render-mode hotkeys: `t` cycles the pixel-to-cell strategy and `g`
//! toggles the edge overlay on every 3d camera.

use bevy_ecs::prelude::{Commands, Entity, Has, MessageReader, Query};
use plurimus::render3d::{EdgeOverlay, LuminanceRamp, RAMP_SHADING, Strategy3d};
use plurimus::term::bevy_input::keyboard::{KeyCode, KeyboardInput};

pub fn toggle_strategy(
    mut keys: MessageReader<KeyboardInput>,
    mut cameras: Query<(Entity, &mut Strategy3d, Has<EdgeOverlay>)>,
    mut commands: Commands,
) {
    for input in keys.read() {
        if input.repeat || !input.state.is_pressed() {
            continue;
        }
        match input.key_code {
            KeyCode::KeyT => {
                for (_, mut strategy, _) in &mut cameras {
                    *strategy = next_strategy(*strategy);
                }
            }
            KeyCode::KeyG => toggle_edges(&cameras, &mut commands),
            _ => {}
        }
    }
}

fn next_strategy(strategy: Strategy3d) -> Strategy3d {
    match strategy {
        Strategy3d::Halfblocks => Strategy3d::Luminance(LuminanceRamp::new(RAMP_SHADING)),
        Strategy3d::Luminance(ramp) if ramp.characters == RAMP_SHADING => {
            Strategy3d::Luminance(LuminanceRamp::default())
        }
        Strategy3d::Luminance(_) => Strategy3d::Braille,
        _ => Strategy3d::Halfblocks,
    }
}

fn toggle_edges(
    cameras: &Query<(Entity, &mut Strategy3d, Has<EdgeOverlay>)>,
    commands: &mut Commands,
) {
    for (camera, _, has_overlay) in cameras.iter() {
        if has_overlay {
            commands.entity(camera).remove::<EdgeOverlay>();
        } else {
            commands.entity(camera).insert(EdgeOverlay::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, Update};
    use plurimus::core::{CorePlugin, TerminalSize};
    use plurimus::term::TermPlugin;

    use super::*;
    use crate::test_input::{character, press};

    fn render_keys_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((CorePlugin, TermPlugin));
        app.insert_resource(TerminalSize::new(10, 4));
        app.add_systems(Update, toggle_strategy);
        let camera = app.world_mut().spawn(Strategy3d::default()).id();
        (app, camera)
    }

    fn type_key(app: &mut App, text: &str) {
        app.world_mut().write_message(press(character(text)));
        app.update();
    }

    #[test]
    fn t_cycles_the_strategy_in_either_case() {
        let (mut app, camera) = render_keys_app();

        let mut seen = Vec::new();
        for text in ["t", "T", "t", "T"] {
            type_key(&mut app, text);
            seen.push(*app.world().entity(camera).get::<Strategy3d>().unwrap());
        }

        assert!(matches!(seen[0], Strategy3d::Luminance(ramp) if ramp.characters == RAMP_SHADING));
        assert!(matches!(seen[1], Strategy3d::Luminance(_)));
        assert_eq!(seen[2], Strategy3d::Braille);
        assert_eq!(seen[3], Strategy3d::Halfblocks);
    }

    #[test]
    fn g_toggles_the_edge_overlay() {
        let (mut app, camera) = render_keys_app();

        type_key(&mut app, "g");
        assert!(app.world().entity(camera).get::<EdgeOverlay>().is_some());

        type_key(&mut app, "g");
        assert!(app.world().entity(camera).get::<EdgeOverlay>().is_none());
    }
}
