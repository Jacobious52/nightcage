use bevy::{
    input::{keyboard::KeyboardInput, mouse::MouseButtonInput},
    prelude::*,
};
use bevy_ecs_tilemap::helpers::square_grid::neighbors::Neighbors;
use bevy_ecs_tilemap::prelude::*;
use nightcage::camera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TilemapPlugin)
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            (
                camera::movement,
                update_cursor_pos,
                highlight_tile_labels,
                apply_tile_textures,
                cycle_tile_texture_index,
                place_highlighted_tile,
                rotate_highlighted_tile,
                illuminate_tiles,
            ),
        )
        .init_resource::<CursorPos>()
        .init_resource::<NextTileTextureIndex>()
        .insert_resource(ClearColor(Color::hex("1F1E19").unwrap()))
        .run();
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // add a border around the tilemap
    let border_handle = asset_server.load("border.png");
    commands.spawn(SpriteBundle {
        texture: border_handle,
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..Default::default()
    });

    let texture_handle: Handle<Image> = asset_server.load("tiles.png");
    let map_size = TilemapSize { x: 7, y: 7 };
    let mut tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 128.0, y: 128.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}

#[derive(Resource)]
pub struct CursorPos(Vec2);
impl Default for CursorPos {
    fn default() -> Self {
        // Initialize the cursor pos at some far away place. It will get updated
        // correctly when the cursor moves.
        Self(Vec2::new(-1000.0, -1000.0))
    }
}

#[derive(Component)]
struct HighlightedLabel;

#[derive(Component)]
struct IlluminatedLabel;

#[derive(Component)]
struct TileType {
    texture_index: u32,
}

// We need to keep the cursor position updated based on any `CursorMoved` events.
pub fn update_cursor_pos(
    mut gizmos: Gizmos,
    camera_q: Query<(&GlobalTransform, &Camera)>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor_pos: ResMut<CursorPos>,
) {
    gizmos.rect_2d(Vec2::ZERO, 0.0, Vec2::splat(7.0 * 128.0), Color::ORANGE_RED);

    for cursor_moved in cursor_moved_events.read() {
        // To get the mouse's world position, we have to transform its window position by
        // any transforms on the camera. This is done by projecting the cursor position into
        // camera space (world space).
        for (cam_t, cam) in camera_q.iter() {
            if let Some(pos) = cam.viewport_to_world_2d(cam_t, cursor_moved.position) {
                *cursor_pos = CursorPos(pos);
            }
        }
    }
}

fn highlight_tile_labels(
    mut commands: Commands,
    cursor_pos: Res<CursorPos>,
    tilemap_q: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapType,
        &TileStorage,
        &Transform,
    )>,
    highlighted_tiles_q: Query<Entity, With<HighlightedLabel>>,
    illuminated_tiles_q: Query<Entity, With<IlluminatedLabel>>,
) {
    for highlighted_tile_entity in highlighted_tiles_q.iter() {
        commands
            .entity(highlighted_tile_entity)
            .remove::<HighlightedLabel>();
    }
    for illuminated_tile_entity in illuminated_tiles_q.iter() {
        commands
            .entity(illuminated_tile_entity)
            .remove::<IlluminatedLabel>();
    }

    for (map_size, grid_size, map_type, tile_storage, map_transform) in tilemap_q.iter() {
        // Grab the cursor position from the `Res<CursorPos>`
        let cursor_pos: Vec2 = cursor_pos.0;
        // We need to make sure that the cursor's world position is correct relative to the map
        // due to any map transformation.
        let cursor_in_map_pos: Vec2 = {
            // Extend the cursor_pos vec3 by 0.0 and 1.0
            let cursor_pos = Vec4::from((cursor_pos, 0.0, 1.0));
            let cursor_in_map_pos = map_transform.compute_matrix().inverse() * cursor_pos;
            cursor_in_map_pos.xy()
        };
        // Once we have a world position we can transform it into a possible tile position.
        if let Some(tile_pos) =
            TilePos::from_world_pos(&cursor_in_map_pos, map_size, grid_size, map_type)
        {
            // Highlight the relevant tile's label
            if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                commands.entity(tile_entity).insert(HighlightedLabel);
            }

            // Highlight the relevant tile's neighbors
            let neighbor_positions =
                Neighbors::get_square_neighboring_positions(&tile_pos, &map_size, false);
            let neighbor_entities = neighbor_positions.entities(&tile_storage);
            for neighbor_entity in neighbor_entities.iter() {
                commands.entity(*neighbor_entity).insert(IlluminatedLabel);
            }
        }
    }
}

// place current hilighted tiles when clicked
fn place_highlighted_tile(
    mut commands: Commands,
    next_tile_texture_index: Res<NextTileTextureIndex>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    highlighted_tiles_q: Query<Entity, With<HighlightedLabel>>,
) {
    for mouse_button_input in mouse_button_input_events.read() {
        if mouse_button_input.button == MouseButton::Left && mouse_button_input.state.is_pressed() {
            for highlighted_tile_entity in highlighted_tiles_q.iter() {
                commands.entity(highlighted_tile_entity).insert(TileType {
                    texture_index: next_tile_texture_index.0,
                });
            }
        }
    }
}

// rotate current hilighted tiles when right mouse clicked
fn rotate_highlighted_tile(
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    highlighted_tiles_q: Query<Entity, With<HighlightedLabel>>,
    mut tile_flips: Query<&mut TileFlip>,
    mut flips: Local<u32>,
) {
    for mouse_button_input in mouse_button_input_events.read() {
        if mouse_button_input.button == MouseButton::Right && mouse_button_input.state.is_pressed()
        {
            for highlighted_tile_entity in highlighted_tiles_q.iter() {
                // get the tile type if it exists on the tile entity
                if let Ok(mut flip) =
                    tile_flips.get_component_mut::<TileFlip>(highlighted_tile_entity)
                {
                    // rotate the tile
                    *flips = (*flips + 1) % 4;
                    match *flips {
                        0 => {
                            flip.x = false;
                            flip.y = false;
                            flip.d = false;
                        }
                        1 => {
                            flip.x = true;
                            flip.y = false;
                            flip.d = true;
                        }
                        2 => {
                            flip.x = true;
                            flip.y = true;
                            flip.d = false;
                        }
                        _ => {
                            flip.x = false;
                            flip.y = true;
                            flip.d = true;
                        }
                    }
                }
            }
        }
    }
}

fn illuminate_tiles(
    mut commands: Commands,
    illuminated_tiles_q: Query<Entity, With<IlluminatedLabel>>,
    non_illuminated_tiles_q: Query<Entity, Without<IlluminatedLabel>>,
) {
    for illuminated_tile_entity in illuminated_tiles_q.iter() {
        commands
            .entity(illuminated_tile_entity)
            .insert(TileColor(Color::ORANGE_RED));
    }

    for non_illuminated_tile_entity in non_illuminated_tiles_q.iter() {
        commands
            .entity(non_illuminated_tile_entity)
            .insert(TileColor(Color::WHITE));
    }
}

fn apply_tile_textures(
    mut commands: Commands,
    next_tile_texture_index: Res<NextTileTextureIndex>,
    highlighted_tiles_q: Query<Entity, With<HighlightedLabel>>,
    non_highlighted_tiles_q: Query<Entity, Without<HighlightedLabel>>,
    tile_types: Query<&TileType>,
) {
    for highlighted_tile_entity in highlighted_tiles_q.iter() {
        commands
            .entity(highlighted_tile_entity)
            .insert(TileTextureIndex(next_tile_texture_index.0));
    }

    for non_highlighted_tile_entity in non_highlighted_tiles_q.iter() {
        if let Ok(tile_type) = tile_types.get_component::<TileType>(non_highlighted_tile_entity) {
            commands
                .entity(non_highlighted_tile_entity)
                .insert(TileTextureIndex(tile_type.texture_index));
        } else {
            commands
                .entity(non_highlighted_tile_entity)
                .insert(TileTextureIndex(0));
        }
    }
}

#[derive(Resource)]
struct NextTileTextureIndex(u32);
impl Default for NextTileTextureIndex {
    fn default() -> Self {
        Self(1)
    }
}

impl NextTileTextureIndex {
    fn next(&mut self) {
        self.0 = (self.0 % 4) + 1;
    }
}

// cycle next tile texture index on pressing space
fn cycle_tile_texture_index(
    mut next_tile_texture_index: ResMut<NextTileTextureIndex>,
    mut keyboard_input_events: EventReader<KeyboardInput>,
) {
    for keyboard_input in keyboard_input_events.read() {
        if keyboard_input.state.is_pressed() && keyboard_input.key_code == Some(KeyCode::Space) {
            next_tile_texture_index.next();
        }
    }
}
