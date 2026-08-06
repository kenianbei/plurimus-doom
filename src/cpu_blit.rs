//! CPU frame path: extracts the latest engine frame and blits it as
//! halfblock cells into the default terminal camera.

use bevy_app::{App, Plugin, Startup};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource};
use plurimus::core::raster::{HalfblockGrid, PixelGrid, blit_halfblocks, letterbox};
use plurimus::core::ratatui_core::style::Color;
use plurimus::core::{
    Background, CameraBuffer, DefaultCamera, MainWorld, RasterizeSystems, SourceCamera,
    TerminalCamera, TerminalRenderApp, TerminalRenderAppExt, TerminalRenderSystems,
    camera_buffer_mut,
};

use crate::doom_thread::{DoomHandle, SharedFrame};

#[derive(Resource, Default)]
struct ExtractedDoomFrame(SharedFrame);

pub struct CpuBlitPlugin;

impl Plugin for CpuBlitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_world_camera);
        app.sub_app_mut(TerminalRenderApp)
            .init_resource::<ExtractedDoomFrame>();
        app.add_extract_systems(extract_frame);
        app.add_terminal_systems(
            TerminalRenderSystems::Rasterize,
            rasterize_doom.in_set(RasterizeSystems::World),
        );
    }
}

fn spawn_world_camera(mut commands: Commands) {
    commands.spawn(TerminalCamera {
        background: Background::Clear(Color::Black),
        ..TerminalCamera::default()
    });
}

fn extract_frame(main_world: Res<MainWorld>, mut extracted: ResMut<ExtractedDoomFrame>) {
    let Some(handle) = main_world.get_resource::<DoomHandle>() else {
        return;
    };
    handle.frame.fetch_if_newer(&mut extracted.0);
}

fn rasterize_doom(
    extracted: Res<ExtractedDoomFrame>,
    default_camera: Res<DefaultCamera>,
    mut cameras: Query<(&SourceCamera, &mut CameraBuffer)>,
    mut grid: Local<HalfblockGrid>,
) {
    let Some(camera) = default_camera.0 else {
        return;
    };
    let Some(mut buffer) = camera_buffer_mut(&mut cameras, camera) else {
        return;
    };
    let frame = &extracted.0;
    let source = PixelGrid::xrgb32(&frame.pixels, frame.width, frame.height);
    grid.reset(buffer.0.area);
    let fit = letterbox((frame.width, frame.height), grid.subcell_area());
    blit_halfblocks(&mut grid, &source, fit);
    grid.resolve_into(&mut buffer.0);
}
