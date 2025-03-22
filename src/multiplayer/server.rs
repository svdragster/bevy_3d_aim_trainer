use crate::fps_controller::fps_controller;
use crate::fps_controller::fps_controller::{EntityShotEvent, FpsController, FpsControllerInput};
use crate::multiplayer::protocol::{
    Inputs, PlayerColor, PlayerId, ReplicatedMoveData, ReplicatedSoundEffect, SoundEvent,
};
use crate::multiplayer::shared::{shared_config, shared_input_behaviour, KEY, PROTOCOL_ID};
use crate::BulletImpact;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::time::Stopwatch;
use bevy::utils::HashMap;
use bevy_rapier3d::dynamics::Velocity;
use bevy_rapier3d::geometry::Collider;
use bevy_rapier3d::plugin::ReadRapierContext;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

pub struct FpsServerPlugin;

impl Plugin for FpsServerPlugin {
    fn build(&self, app: &mut App) {
        let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 25565);
        // You need to provide the private key and protocol id when building the `NetcodeConfig`
        let netcode_config = NetcodeConfig::default()
            .with_protocol_id(PROTOCOL_ID)
            .with_key(KEY);
        // You can also add a link conditioner to simulate network conditions for packets received by the server
        let link_conditioner = LinkConditionerConfig {
            incoming_latency: Duration::from_millis(100),
            incoming_jitter: Duration::from_millis(0),
            incoming_loss: 0.00,
        };
        let io_config = lightyear::connection::server::IoConfig::from_transport(
            server::ServerTransport::UdpSocket(server_addr),
        )
        //.with_conditioner(link_conditioner)
          ;
        let net_config = NetConfig::Netcode {
            config: netcode_config,
            io: io_config,
        };
        let server_config = ServerConfig {
            shared: shared_config(Mode::Separate),
            // Here we only provide a single net config, but you can provide multiple!
            net: vec![net_config],
            ..default()
        };

        let server_plugin = server::ServerPlugins::new(server_config);
        app.add_plugins(server_plugin);
        app.add_systems(Startup, (start_server, setup_spectator));
        app.add_systems(Update, (handle_connections, draw_gizmos));
        app.add_systems(
            FixedUpdate,
            (on_input, update_physics, post_update_physics).chain(),
        );
        app.insert_resource(Global {
            client_id_to_entity_id: HashMap::default(),
        });
        app.add_systems(Update, (on_entity_shot).run_if(on_event::<EntityShotEvent>));
        app.add_systems(Update, (on_sound_event).run_if(on_event::<SoundEvent>));

        //app.add_event::<InputEvent<Inputs>>();
    }
}

fn start_server(mut commands: Commands) {
    commands.start_server();
}

fn setup_spectator(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 7.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            order: 0,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: TAU / 5.0,
            ..default()
        }),
        Exposure::SUNLIGHT,
    ));
}

#[derive(Resource)]
pub(crate) struct Global {
    pub client_id_to_entity_id: HashMap<ClientId, Entity>,
}

const COLORS: [Color; 8] = [
    Color::srgb(0.0, 0.0, 1.0),
    Color::srgb(1.0, 0.0, 0.0),
    Color::srgb(0.0, 1.0, 0.0),
    Color::srgb(1.0, 1.0, 0.0),
    Color::srgb(1.0, 0.0, 1.0),
    Color::srgb(0.0, 1.0, 1.0),
    Color::srgb(1.0, 0.5, 0.0),
    Color::srgb(0.5, 0.0, 1.0),
];

// Create a player entity whenever a client connects
pub(crate) fn handle_connections(
    // Here we listen for the `ConnectEvent` event
    mut connections: EventReader<ConnectEvent>,
    mut global: ResMut<Global>,
    mut commands: Commands,
) {
    let mut rng = rand::rng();
    for connection in connections.read() {
        // on the server, the `context()` method returns the `ClientId` of the client that connected
        let client_id = connection.client_id;

        let color = COLORS
            .get(rng.random_range(0..COLORS.len()))
            .unwrap_or(&Color::WHITE)
            .clone();

        let logical_entity = fps_controller::spawn_logical_entity(&mut commands);

        // We add the `Replicate` bundle to start replicating the entity to clients
        // By default, the entity will be replicated to all clients
        let replicated_entity = commands
            .entity(logical_entity)
            .insert((
                PlayerId(client_id.clone()),
                ReplicatedMoveData {
                    translation: Default::default(),
                    scale: Default::default(),
                    velocity: Default::default(),
                    yaw: 0.0,
                    pitch: 0.0,
                },
                PlayerColor(color),
                Replicate {
                    sync: SyncTarget {
                        prediction: NetworkTarget::Single(client_id.clone()),
                        ..default()
                    },
                    ..default()
                },
            ))
            .id();

        // Add a mapping from client id to entity id
        global
            .client_id_to_entity_id
            .insert(client_id, replicated_entity);
    }
}

fn on_input(
    // Event that will contain the inputs for the correct tick
    mut input_reader: EventReader<InputEvent<Inputs>>,
    // Retrieve the entity associated with a given client
    global: Res<Global>,
    mut query: Query<&mut FpsControllerInput>,
) {
    for input in input_reader.read() {
        let client_id = input.from();
        if let Some(input) = input.input() {
            if let Some(player_entity) = global.client_id_to_entity_id.get(&client_id) {
                shared_input_behaviour(player_entity, input, &mut query);
            }
        }
    }
}

fn update_physics(
    time: Res<Time>,
    physics_context: ReadRapierContext,
    mut query: Query<(
        Entity,
        &mut FpsController,
        &mut FpsControllerInput,
        &mut Collider,
        &mut Transform,
        &mut Velocity,
    )>,
    query_move_data: Query<&ReplicatedMoveData>,
    mut entity_shot_event: EventWriter<EntityShotEvent>,
    mut sound_event: EventWriter<SoundEvent>,
) {
    fps_controller::fps_controller_move(&time, &physics_context, &mut query);
    fps_controller::fps_controller_shoot(
        &time,
        &physics_context,
        &mut query,
        &query_move_data,
        &mut entity_shot_event,
        &mut sound_event,
    );
}

fn post_update_physics(
    mut query_move_data: Query<&mut ReplicatedMoveData>,
    query: Query<(Entity, &FpsController, &Transform, &Velocity)>,
) {
    for (entity, controller, transform, velocity) in query.iter() {
        if let Ok(mut replicated_move_data) = query_move_data.get_mut(entity) {
            replicated_move_data.translation = (*transform).translation.clone();
            replicated_move_data.scale = (*transform).scale.clone();
            replicated_move_data.velocity = (*velocity).linvel.clone();
            replicated_move_data.yaw = controller.yaw;
            replicated_move_data.pitch = controller.pitch;
        }
    }
}

fn draw_gizmos(mut gizmos: Gizmos, players: Query<(&ReplicatedMoveData, &PlayerColor, &PlayerId)>) {
    for (position, color, player_id) in &players {
        gizmos.sphere(
            Isometry3d::new(
                position.translation + Vec3::new(0.0, 1.5, 0.0),
                Quat::default(),
            ),
            0.5,
            color.0,
        );
        let rotation = Quat::from_euler(EulerRot::YXZ, position.yaw, position.pitch, 0.0);
        gizmos.arrow(
            position.translation + Vec3::new(0.0, 1.5, 0.0),
            position.translation + Vec3::new(0.0, 1.5, 0.0) + rotation * Vec3::NEG_Z,
            color.0,
        );
    }
}

fn on_entity_shot(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut entity_shot_event: EventReader<EntityShotEvent>,
) {
    for event in entity_shot_event.read() {
        let entity = event.entity;
        let hit_point = event.hit_point;
        commands.spawn((
            BulletImpact {
                stopwatch: Stopwatch::new(),
            },
            Transform::from_translation(hit_point),
            Mesh3d(meshes.add(Sphere::new(0.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                ..Default::default()
            })),
        ));
    }
}

fn on_sound_event(mut commands: Commands, mut sound_event: EventReader<SoundEvent>) {
    for event in sound_event.read() {
        let emitter = event.emitter;
        let asset = event.asset.clone();
        let position = event.position;
        let volume = event.volume;
        let speed = event.speed;
        let spatial = event.spatial;
        let spatial_scale = event.spatial_scale;

        commands.spawn((
            ReplicatedSoundEffect {
                emitter,
                asset,
                position,
                volume,
                speed,
                spatial,
                spatial_scale,
            },
            Replicate::default(),
        ));
    }
}
