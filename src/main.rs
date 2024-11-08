use bevy::color::palettes::basic::RED;
use bevy::ecs::world;
use bevy::input::common_conditions::*;
use bevy::input::mouse::MouseMotion;
use bevy::render::camera::ScalingMode;
use bevy::sprite::Anchor;
use bevy::window::CursorGrabMode;
use bevy::{
    prelude::*,
    window::{WindowResized, WindowResolution},
};
use bevy_slippy_tiles::{
    Coordinates, DownloadSlippyTilesEvent, Radius, SlippyTileCoordinates,
    SlippyTileDownloadedEvent, SlippyTilesPlugin, SlippyTilesSettings, TileSize, ZoomLevel,
};

use bevy::input::mouse::MouseWheel;

fn main() {
    App::new()
        // Our slippy tiles settings and plugin
        .insert_resource(SlippyTilesSettings::new(
            "https://tile.openstreetmap.org", // Tile server endpoint.
            "tiles/",                         // assets/ folder storing the slippy tile downloads.
        ))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Map Example".into(),
                resolution: WindowResolution::new(1280.0, 720.0),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SlippyTilesPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                start_moving.run_if(input_just_pressed(MouseButton::Left)),
                update_camera_move.run_if(input_pressed(MouseButton::Left)),
                end_moving.run_if(input_just_released(MouseButton::Left)),
            )
                .chain(),
        )
        .add_systems(Update, update_camera_zoom.run_if(run_if_scroll))
        .add_systems(Update, display_tiles)
        .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct TextBox;

#[derive(Resource)]
struct WorldState {
    position: Vec2,
    camera_position: Vec3,
    world: Vec2,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut download_slippy_tile_events: EventWriter<DownloadSlippyTilesEvent>,
) {
    let zoom_level = ZoomLevel::L1;
    let tile_size_x = 256;
    let tile_size_y = 256;
    let size_x = 2_i32.pow(zoom_level.to_u8() as u32);
    let size_y = 2_i32.pow(zoom_level.to_u8() as u32);
    let world_x = (tile_size_x * size_x) as f32;
    let world_y = (tile_size_y * size_y) as f32;
    print!(
        "Setup. world_x={}, world_y={}, size_y={}, size_x={}",
        world_x, world_y, size_x, size_y
    );

    // Components.
    commands.spawn((
        MainCamera,
        Camera2dBundle {
            projection: OrthographicProjection {
                far: 1000.,
                near: -1000.,
                scaling_mode: ScalingMode::Fixed {
                    width: world_x,
                    height: world_y,
                },
                ..default()
            },
            ..default()
        },
    ));

    let slippy_tile_event = DownloadSlippyTilesEvent {
        tile_size: TileSize::Normal, // Size of tiles - Normal = 256px, Large = 512px (not all tile servers).
        zoom_level: zoom_level, // Map zoom level (L0 = entire world, L19 = closest zoom level).
        coordinates: Coordinates::from_latitude_longitude(0.0, 0.0),
        radius: Radius(1), // Request one layer of surrounding tiles (2 = two layers of surrounding tiles - 25 total, 3 = three layers of surrounding tiles - 49 total, etc).
        use_cache: true, // Don't make request if already requested previously, or if file already exists in tiles directory.
    };
    download_slippy_tile_events.send(slippy_tile_event);

    commands.spawn((TextBox, Text2dBundle::default()));

    // Resources.
    commands.insert_resource(WorldState {
        position: Vec2::default(),
        camera_position: Vec3::default(),
        world: Vec2::new(world_x, world_y),
    });
}

fn start_moving(
    cameras: Query<&Transform, With<MainCamera>>,
    mut windows: Query<&mut Window>,
    mut state: ResMut<WorldState>,
) {
    println!("Mouse pressed");
    let camera = cameras.single();
    let mut window = windows.single_mut();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor_position().map(|pos| {
        state.position = pos;
        state.camera_position = camera.translation;
    });
}

fn end_moving(
    cameras: Query<&Transform, With<MainCamera>>,
    mut windows: Query<&mut Window>,
    mut state: ResMut<WorldState>,
) {
    println!("Mouse released");
    let camera = cameras.single();
    let mut window = windows.single_mut();
    window.cursor.grab_mode = CursorGrabMode::None;
    window.cursor_position().map(|pos| {
        state.position = pos;
        state.camera_position = camera.translation;
    });

    let lat = camera.translation.y / 10.;
    let lon = camera.translation.x / 10.;
    println!("Coordinates lat={}; lon={}", lat, lon);
    info!(
        "Requesting slippy tile for latitude/longitude: {:?}",
        (lat, lon)
    );
}

fn update_camera_move(
    mut cameras: Query<(&mut Transform, &GlobalTransform, &mut Camera), With<MainCamera>>,
    mut texts: Query<(&mut Transform, &mut Text), (With<TextBox>, Without<MainCamera>)>,
    state: Res<WorldState>,
    windows: Query<&Window>,
    mut gizmos: Gizmos,
    mut evr_motion: EventReader<MouseMotion>,
) {
    let window = windows.single();
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    println!("Begin update");
    let mut camera = cameras.single_mut();
    let mut text = texts.single_mut();

    // Calculate a world position based on the cursor's position.
    camera
        .2
        .viewport_to_world_2d(camera.1, cursor_position)
        .map(|point| {
            let radius = 25.0;
            gizmos.circle_2d(point, radius, RED);
            text.0.translation.x = point.x;
            text.0.translation.y = point.y + radius;
            text.1.sections = vec![TextSection {
                value: point.to_string(),
                style: TextStyle {
                    font_size: 32.0,
                    color: Color::Srgba(Srgba { ..RED }),
                    ..default()
                },
            }];

            camera
                .2
                .viewport_to_world_2d(camera.1, state.position)
                .map(|start_point| {
                    camera.0.translation.x = state.camera_position.x + start_point.x - point.x;
                    camera.0.translation.y = state.camera_position.y + start_point.y - point.y;
                });
        });

    //for ev in evr_motion.read() {
    //    println!("Mouse moved: X: {} px, Y: {} px", ev.delta.x, ev.delta.y);
    //    camera.0.translation.x -= ev.delta.x;
    //    camera.0.translation.y += ev.delta.y;
    //}

    println!("Exit update");
}

fn run_if_scroll(evr_scroll: EventReader<MouseWheel>) -> bool {
    !evr_scroll.is_empty()
}

fn update_camera_zoom(
    mut cameras: Query<(&mut OrthographicProjection, &mut Camera), With<MainCamera>>,
    mut evr_scroll: EventReader<MouseWheel>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    println!("Begin update");
    for ev in evr_scroll.read() {
        for mut camera in &mut cameras {
            match ev.unit {
                MouseScrollUnit::Line => {
                    println!(
                        "Scroll (line units): vertical: {}, horizontal: {}",
                        ev.y, ev.x
                    );
                }
                MouseScrollUnit::Pixel => {
                    println!(
                        "Scroll (pixel units): vertical: {}, horizontal: {}",
                        ev.y, ev.x
                    );
                }
            };
            camera.0.scale += 0.1 * ev.y;
        }
    }
    println!("Exit update");
}

fn display_tiles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    state: Res<WorldState>,
    mut slippy_tile_downloaded_events: EventReader<SlippyTileDownloadedEvent>,
) {
    for slippy_tile_downloaded_event in slippy_tile_downloaded_events.read() {
        println!("Display tiles");
        info!("Slippy tile fetched: {:?}", slippy_tile_downloaded_event);
        let zoom_level = slippy_tile_downloaded_event.zoom_level;
        // Convert our slippy tile position to pixels on the screen relative to the center tile.
        let SlippyTileCoordinates {
            x: center_x,
            y: center_y,
        } = Coordinates::from_latitude_longitude((0.0).into(), (0.0).into())
            .get_slippy_tile_coordinates(zoom_level);
        let SlippyTileCoordinates {
            x: current_x,
            y: current_y,
        } = slippy_tile_downloaded_event
            .coordinates
            .get_slippy_tile_coordinates(zoom_level);

        let tile_pixels = slippy_tile_downloaded_event.tile_size.to_pixels() as f32;
        let transform_x = (center_x as f32 - current_x as f32) * tile_pixels - state.world.x / 2.0
            + tile_pixels / 2.0;
        let transform_y = (center_y as f32 - current_y as f32) * tile_pixels - state.world.y / 2.0
            + tile_pixels / 2.0;
        print!(
            "pixels={}, current_x={}, current_y={}, center_x={}, center_y={}, x={}, y={}",
            tile_pixels, current_x, current_y, center_x, center_y, transform_x, transform_y
        );

        // Add our slippy tile to the screen.
        commands.spawn(SpriteBundle {
            texture: asset_server.load(slippy_tile_downloaded_event.path.clone()),
            transform: Transform::from_xyz(transform_x, transform_y, 0.0),
            sprite: Sprite {
                custom_size: Some(Vec2::new(tile_pixels, tile_pixels)),
                ..default()
            },
            ..Default::default()
        });
    }
}
