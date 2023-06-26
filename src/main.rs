mod events;
mod ray;

use std::time::Instant;

use events::WandCastEvent;
use ray::{raycast, Ray, RayHit};
use valence::{
    app::App,
    client::hand_swing::HandSwingEvent,
    entity::{entity::NameVisible, zombie::ZombieEntityBundle, *},
    inventory::HeldItem,
    network::{async_trait, BroadcastToLan, ConnectionMode},
    prelude::*,
};

#[allow(unused)]
use valence::log;

const SPAWN_Y: i32 = 64;

// Projectile Marker
#[derive(Component)]
struct Projectile {
    pub spawn_time: Instant,
    pub particle: Particle,
    pub location: Location,
    pub position: Vec3,
    pub direction: Vec3,
    pub speed: f32,
}

fn main() {
    let mut app = App::new();
    build_app(&mut app);
    app.run();
}

pub fn build_app(app: &mut App) {
    app.add_event::<WandCastEvent>();

    app.insert_resource(NetworkSettings {
        connection_mode: ConnectionMode::Offline,
        callbacks: MyCallbacks.into(),
        ..Default::default()
    })
    .add_plugins(DefaultPlugins)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            init_clients,
            despawn_disconnected_clients,
            on_client_click,
            on_wand_cast,
            update_projectile,
            projectile_collision_detect,
        ),
    )
    .run();
}

fn on_wand_cast(
    mut commands: Commands,
    mut clients: Query<(&Position, &Look, &Location)>,
    mut wand_cast_events: EventReader<WandCastEvent>,
) {
    for event in wand_cast_events.iter() {
        let Ok((pos, look, location)) = clients.get_mut(event.client) else {
            continue;
        };

        let direction: Vec3 = look.vec();

        let position = pos.0.clone();

        let position = Vec3::from((
            position.x as f32,
            position.y as f32 + 1.40,
            position.z as f32,
        ));

        let direction = Vec3::new(direction.x as f32, direction.y as f32, direction.z as f32);

        commands.spawn(Projectile {
            spawn_time: Instant::now(),
            particle: Particle::Dust {
                rgb: [1.0, 0.0, 0.0].into(),
                scale: 1.0,
            },
            location: location.clone(),
            position: position,
            direction: direction.normalize(),
            speed: 2.5,
        });
    }
}

fn update_projectile(
    mut instances: Query<&mut Instance>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile)>,
) {
    for (entity, mut projectile) in projectiles.iter_mut() {
        if projectile.spawn_time.elapsed().as_secs() > 5 {
            commands.entity(entity).insert(Despawned);
            continue;
        }

        let Ok(mut instance) = instances.get_mut(projectile.location.0) else {
            log::warn!("No instance on projectile, aborting..");
            continue;
        };

        let new_pos = projectile.position + (projectile.direction * projectile.speed);

        projectile.position = new_pos;

        instance.play_particle(
            &projectile.particle,
            true,
            [new_pos.x as f64, new_pos.y as f64, new_pos.z as f64],
            [0.0, 0.0, 0.0],
            25.0,
            10,
        );
    }
}

fn projectile_collision_detect(
    mut commands: Commands,
    mut instances: Query<&mut Instance>,
    mut projectiles: Query<(Entity, &Projectile)>,
    entities: Query<(Entity, &Hitbox)>,
) {
    for (entity, projectile) in projectiles.iter_mut() {
        let Ok(mut instance) = instances.get_mut(projectile.location.0) else {
            log::warn!("No instance on projectile, aborting..");
            continue;
        };

        let pos = DVec3::new(
            projectile.position.x as f64,
            projectile.position.y as f64,
            projectile.position.z as f64,
        );

        let dir = DVec3::new(
            projectile.direction.x as f64,
            projectile.direction.y as f64,
            projectile.direction.z as f64,
        );

        let ray = Ray::new(pos, dir, projectile.speed as f64);

        let hits = raycast(ray, &instance, &entities);
        for hit in hits {
            if let RayHit::Entity {
                entity: _,
                position,
            } = hit
            {
                // spawn a particle at the hit position
                instance.play_particle(
                    &Particle::Explosion,
                    true,
                    position,
                    [0.0, 0.0, 0.0],
                    0.05,
                    1,
                );

                // despawn projectile
                commands.entity(entity).insert(Despawned);
                break;
            } else if let RayHit::Block { state, pos, offset } = hit {
                if (state != BlockState::AIR) && (state != BlockState::CAVE_AIR) {
                    // calculate where on the block we've hit
                    let hit_pos = DVec3::new(pos.x as f64, pos.y as f64, pos.z as f64) + offset;

                    // spawn a particle at the hit position
                    instance.play_particle(
                        &Particle::Explosion,
                        true,
                        hit_pos,
                        [0.0, 0.0, 0.0],
                        0.05,
                        1,
                    );

                    // despawn projectile
                    commands.entity(entity).insert(Despawned);
                    break;
                }
            }
        }
    }
}

fn on_client_click(
    mut clients: Query<&HeldItem>,
    mut _instances: Query<&mut Instance>,
    mut hand_swing_events: EventReader<HandSwingEvent>,
    mut wand_cast_events: EventWriter<WandCastEvent>,
) {
    let _instance = _instances.single();

    for event in hand_swing_events.iter() {
        let Ok(held) = clients.get_mut(event.client) else {
            continue;
        };

        let bar_slot = held.slot() - 36;

        if bar_slot == 0 {
            wand_cast_events.send(WandCastEvent {
                client: event.client,
                slot: held.slot(), // this shouldnt be slot, this should be the selected spell, when it gets implemented
            });
        }
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    mut dimensions: ResMut<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    dimensions.insert(
        ident!("hogwarts"),
        DimensionType {
            ambient_light: 1.0,
            has_skylight: false,
            has_ceiling: false,
            natural: false,
            ..Default::default()
        },
    );

    let mut instance = Instance::new(ident!("hogwarts"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    // add a wall
    for y in 0..5 {
        for x in -5..5 {
            instance.set_block([x, SPAWN_Y + y, 5], BlockState::STONE_BRICKS);
        }
    }

    // add a torch to the wall
    instance.set_block(
        [0, SPAWN_Y + 2, 4],
        BlockState::WALL_TORCH.set(PropName::Facing, PropValue::North),
    );

    let instance_id = commands.spawn(instance).id();

    // spawn a zombie
    commands.spawn(ZombieEntityBundle {
        location: Location(instance_id),
        position: Position(DVec3::new(4.0, SPAWN_Y as f64 + 1.0, 1.0)),
        look: Look::new(180.0, 0.0),
        head_yaw: HeadYaw(135.0),
        entity_name_visible: NameVisible(true),
        ..Default::default()
    });

    commands.spawn(ZombieEntityBundle {
        location: Location(instance_id),
        position: Position(DVec3::new(-4.0, SPAWN_Y as f64 + 1.0, 1.0)),
        look: Look::new(180.0, 0.0),
        head_yaw: HeadYaw(225.0),
        entity_name_visible: NameVisible(true),
        ..Default::default()
    });
}

fn init_clients(
    mut clients: Query<
        (
            &mut Location,
            &mut Position,
            &mut HasRespawnScreen,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut has_respawn_screen, mut game_mode) in &mut clients {
        loc.0 = instances.iter().next().unwrap();
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        has_respawn_screen.0 = true;
        *game_mode = GameMode::Adventure;
    }
}

struct MyCallbacks;

#[async_trait]
impl NetworkCallbacks for MyCallbacks {
    async fn broadcast_to_lan(&self, _shared: &SharedNetworkState) -> BroadcastToLan {
        BroadcastToLan::Enabled("Connect!".into())
    }
}
