//! GPU frame path: uploads each engine frame into a texture shown on a
//! CRT-shaded quad in front of a 3d camera, converted to cells by the 3d
//! pipeline.

use bevy_app::{App, Plugin, Startup, Update};
use bevy_asset::{Asset, Assets, Handle, RenderAssetUsages, embedded_asset};
use bevy_camera::Camera3d;
use bevy_core_pipeline::tonemapping::Tonemapping;
use bevy_ecs::prelude::{Commands, Entity, MessageReader, Query, Res, ResMut, Resource};
use bevy_image::Image;
use bevy_math::primitives::Rectangle;
use bevy_mesh::{Mesh, Mesh3d};
use bevy_pbr::{Material, MaterialPlugin, MeshMaterial3d, PbrPlugin};
use bevy_reflect::TypePath;
use bevy_render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy_shader::ShaderRef;
use bevy_transform::components::Transform;
use doomgeneric::game::{DOOMGENERIC_RESX, DOOMGENERIC_RESY};
use plurimus::core::TerminalCamera;
use plurimus::core::raster::PixelGrid;
use plurimus::render3d::{
    EdgeOverlay, LuminanceRamp, Plugin3d, RAMP_SHADING, Render3dPlugins, Strategy3d,
};
use plurimus::term::{KeyCode, KeyKind, KeyMessage, TermPlugin};

use crate::doom_thread::{DoomHandle, SharedFrame};

const QUAD_HEIGHT: f32 = 2.0;
const CAMERA_DISTANCE: f32 = 2.42;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct CrtMaterial {
    #[texture(0)]
    #[sampler(1)]
    frame: Handle<Image>,
}

impl Material for CrtMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://plurimus_doom/crt.wgsl".into()
    }
}

#[derive(Resource)]
struct FrameTexture {
    image: Handle<Image>,
    latest: SharedFrame,
}

pub struct GpuPlugin;

impl Plugin for GpuPlugin {
    fn build(&self, app: &mut App) {
        // The render-mode hotkeys read terminal input directly.
        if !app.is_plugin_added::<TermPlugin>() {
            app.add_plugins(TermPlugin);
        }
        // Render3dPlugins stops at the render stack, so the material
        // system is ours to add - MaterialPlugin alone has no mesh
        // pipeline behind it.
        app.add_plugins((Render3dPlugins, PbrPlugin::default(), Plugin3d));
        embedded_asset!(app, "crt.wgsl");
        app.add_plugins(MaterialPlugin::<CrtMaterial>::default());
        app.add_systems(Startup, spawn_screen);
        app.add_systems(Update, (upload_frame, toggle_strategy));
    }
}

fn spawn_screen(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CrtMaterial>>,
) {
    let image = images.add(blank_frame_image());
    commands.insert_resource(FrameTexture {
        image: image.clone(),
        latest: SharedFrame::default(),
    });
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(quad_width(), QUAD_HEIGHT))),
        MeshMaterial3d(materials.add(CrtMaterial { frame: image })),
    ));
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        TerminalCamera::default(),
        Strategy3d::default(),
        Transform::from_xyz(0.0, 0.0, CAMERA_DISTANCE),
    ));
}

fn toggle_strategy(
    mut keys: MessageReader<KeyMessage>,
    mut cameras: Query<(Entity, &mut Strategy3d, Option<&EdgeOverlay>)>,
    mut commands: Commands,
) {
    for message in keys.read() {
        if message.kind != KeyKind::Press {
            continue;
        }
        match message.code {
            KeyCode::Char('t') => {
                for (_, mut strategy, _) in &mut cameras {
                    *strategy = next_strategy(*strategy);
                }
            }
            KeyCode::Char('g') => toggle_edges(&cameras, &mut commands),
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
    cameras: &Query<(Entity, &mut Strategy3d, Option<&EdgeOverlay>)>,
    commands: &mut Commands,
) {
    for (camera, _, overlay) in cameras.iter() {
        if overlay.is_some() {
            commands.entity(camera).remove::<EdgeOverlay>();
        } else {
            commands.entity(camera).insert(EdgeOverlay::default());
        }
    }
}

fn quad_width() -> f32 {
    let aspect = DOOMGENERIC_RESX as f32 / DOOMGENERIC_RESY as f32;
    QUAD_HEIGHT * aspect
}

fn blank_frame_image() -> Image {
    Image::new_fill(
        Extent3d {
            width: DOOMGENERIC_RESX as u32,
            height: DOOMGENERIC_RESY as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, u8::MAX],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn upload_frame(
    handle: Res<DoomHandle>,
    mut texture: ResMut<FrameTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    let FrameTexture { image, latest } = &mut *texture;
    if !handle.frame.fetch_if_newer(latest) {
        return;
    }
    if latest.pixels.len() != DOOMGENERIC_RESX * DOOMGENERIC_RESY {
        return;
    }
    if let Some(mut image) = images.get_mut(&*image) {
        write_rgba(latest, &mut image);
    }
}

fn write_rgba(frame: &SharedFrame, image: &mut Image) {
    let Some(data) = image.data.as_mut() else {
        return;
    };
    PixelGrid::xrgb32(&frame.pixels, frame.width, frame.height).write_rgba8(data);
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use plurimus::core::{CorePlugin, TerminalSize};
    use plurimus::term::KeyModifiers;

    use super::*;

    fn render_keys_app() -> (App, bevy_ecs::prelude::Entity) {
        let mut app = App::new();
        app.add_plugins((CorePlugin, TermPlugin));
        app.insert_resource(TerminalSize::new(10, 4));
        app.add_systems(Update, toggle_strategy);
        let camera = app.world_mut().spawn(Strategy3d::default()).id();
        (app, camera)
    }

    fn press(app: &mut App, character: char) {
        app.world_mut().write_message(KeyMessage::new(
            KeyCode::Char(character),
            KeyModifiers::default(),
            KeyKind::Press,
        ));
        app.update();
    }

    #[test]
    fn t_cycles_the_strategy() {
        let (mut app, camera) = render_keys_app();

        let mut seen = Vec::new();
        for _ in 0..4 {
            press(&mut app, 't');
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

        press(&mut app, 'g');
        assert!(app.world().entity(camera).get::<EdgeOverlay>().is_some());

        press(&mut app, 'g');
        assert!(app.world().entity(camera).get::<EdgeOverlay>().is_none());
    }

    #[test]
    fn xrgb_pixels_upload_as_rgba_bytes() {
        let mut image = blank_frame_image();
        let frame = SharedFrame {
            pixels: vec![0x00ff_8040],
            width: 1,
            height: 1,
            version: 1,
        };

        write_rgba(&frame, &mut image);

        let data = image.data.as_deref().unwrap();
        assert_eq!(&data[..4], &[0xff, 0x80, 0x40, 0xff]);
    }
}

#[cfg(test)]
mod gpu_smoke {
    //! Needs a wgpu adapter; run with
    //! `cargo test --features gpu -- --ignored`.

    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;
    use std::sync::mpsc;

    use bevy_app::{App, PluginsState};
    use plurimus::core::{
        CorePlugin, FrameBuffer, TerminalCamera, TerminalRenderApp, TerminalSize,
    };

    use super::GpuPlugin;
    use crate::doom_thread::{DoomHandle, FrameHandle, SharedFrame};

    fn composed_frame(app: &App) -> String {
        let buffer = &app
            .sub_app(TerminalRenderApp)
            .world()
            .resource::<FrameBuffer>()
            .0;
        let area = buffer.area;
        let mut frame = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                frame.push_str(buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()));
            }
            frame.push('\n');
        }
        frame
    }

    const RED: u32 = 0x00ff_0000;

    #[test]
    #[ignore = "requires a wgpu adapter"]
    fn the_engine_frame_reaches_the_terminal_cells() {
        let (keys, _key_receiver) = mpsc::channel();
        let mut app = App::new();
        app.add_plugins((CorePlugin, GpuPlugin));
        app.insert_resource(TerminalSize::new(40, 12));
        app.insert_resource(DoomHandle {
            frame: FrameHandle::new(SharedFrame {
                pixels: vec![RED; 16],
                width: 4,
                height: 4,
                version: 1,
            }),
            keys,
            turn: Arc::new(AtomicI32::new(0)),
        });
        app.world_mut().spawn(TerminalCamera::default());
        while app.plugins_state() == PluginsState::Adding {
            bevy_tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();

        for _ in 0..30 {
            app.update();
        }

        let frame = composed_frame(&app);
        assert!(
            frame.chars().any(|cell| cell != ' '),
            "the CRT quad never reached the cells - the render stack is \
             missing a plugin:\n{frame}"
        );
    }
}
