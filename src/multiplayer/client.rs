use std::f32::consts::{FRAC_PI_2, PI, TAU};
use crate::multiplayer::protocol::{InputData, Inputs, PlayerColor, PlayerId, ReplicatedMoveData, ReplicatedSoundEffect, SoundEvent};
use crate::multiplayer::shared::{shared_config, shared_input_behaviour, KEY, PROTOCOL_ID};
use bevy::prelude::*;
use lightyear::client::input::native::InputSystemSet;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use rand::Rng;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Add;
use std::time::Duration;
use bevy::audio::{SpatialScale, Volume};
use bevy::input::mouse::MouseMotion;
use bevy::time::Stopwatch;
use bevy_rapier3d::dynamics::Velocity;
use bevy_rapier3d::geometry::Collider;
use bevy_rapier3d::plugin::ReadRapierContext;
use crate::{fps_gun_plugin, BulletImpact, Global};
use crate::animations::animated_entity_plugin::{Animations, LoadedAnimations};
use crate::fps_controller::fps_controller;
use crate::fps_controller::fps_controller::{EntityShotEvent, FpsController, FpsControllerInput, ANGLE_EPSILON, EYE_HEIGHT_OFFSET};

pub struct FpsClientPlugin {
    pub server_port: u16,
    pub server_ip: IpAddr,
}

impl Plugin for FpsClientPlugin {
    fn build(&self, app: &mut App) {
        // You can add a link conditioner to simulate network conditions
        let link_conditioner = LinkConditionerConfig {
            incoming_latency: Duration::from_millis(100),
            incoming_jitter: Duration::from_millis(0),
            incoming_loss: 0.00,
        };
        // Here we use the `UdpSocket` transport layer, with the link conditioner
        let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);
        let io_config = IoConfig::from_transport(client::ClientTransport::UdpSocket(client_addr))
            //.with_conditioner(link_conditioner)
          ;

        let mut rng = rand::rng();
        let client_id = rng.random_range(0..=u64::MAX);

        let server_addr = SocketAddr::new(self.server_ip, self.server_port);
        let auth = Authentication::Manual {
            // server's IP address
            server_addr,
            // ID to uniquely identify the client
            client_id,
            // private key shared between the client and server
            private_key: KEY,
            // PROTOCOL_ID identifies the version of the protocol
            protocol_id: PROTOCOL_ID,
        };

        let net_config = NetConfig::Netcode {
            auth,
            config: NetcodeConfig { ..default() },
            io: io_config,
        };

        let client_config = client::ClientConfig {
            shared: shared_config(Mode::Separate),
            net: net_config,
            ..default()
        };
        let client_plugin = client::ClientPlugins::new(client_config);
        app.add_plugins(client_plugin);
        app.add_systems(Startup, init);
        app.add_systems(
            FixedPreUpdate,
            buffer_input.in_set(InputSystemSet::BufferInputs),
        );
        app.add_systems(FixedUpdate, (
            on_player_input,
            update_physics,
            post_update_physics,
        ).chain());
        app.add_systems(Update, (draw_gizmos, receive_entity_spawn));
        app.add_systems(Update, (on_entity_shot).run_if(on_event::<EntityShotEvent>));
        app.add_systems(Update, (on_sound_event).run_if(on_event::<SoundEvent>));
        app.add_systems(Update, (on_sound_from_server));
        app.add_systems(Update, (update_fps_animations));
        app.insert_resource(ClientData { client_id, client_entity: None });
    }
}

#[derive(Resource)]
pub struct ClientData {
    pub client_id: u64,
    pub client_entity: Option<Entity>,
}

#[derive(Component)]
struct LocalPlayer;

fn init(mut commands: Commands) {
    commands.connect_client();
}

pub(crate) fn buffer_input(
    // You will need to specify the exact tick at which the input was emitted. You can use
    // the `TickManager` to retrieve the current tick
    tick_manager: Res<TickManager>,
    // You will use the `InputManager` to send an input
    mut input_manager: ResMut<InputManager<Inputs>>,
    keypress: Res<ButtonInput<KeyCode>>,
    mut mouse_events: EventReader<MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    query_fps_controller_input: Query<&FpsControllerInput>,
    mut global: ResMut<Global>,
) {
    let tick = tick_manager.tick();
    if query_fps_controller_input.is_empty() {
        return;
    }
    if !global.mouse_captured {
        return;
    }
    let fps_controller_input = query_fps_controller_input.single();
    let mut input = Inputs::None;
    let mut input_data = InputData {
        fly: false,
        sprint: false,
        jump: false,
        crouch: false,
        shoot: false,
        movement: Vec3::ZERO,
        pitch: fps_controller_input.pitch,
        yaw: fps_controller_input.yaw,
    };
    if keypress.pressed(KeyCode::KeyW) {
        input_data.movement.z += 1.0;
    }

    if keypress.pressed(KeyCode::KeyS) {
        input_data.movement.z -= 1.0;
    }
    if keypress.pressed(KeyCode::KeyA) {
        input_data.movement.x -= 1.0;
    }
    if keypress.pressed(KeyCode::KeyD) {
        input_data.movement.x += 1.0;
    }
    if keypress.pressed(KeyCode::Space) {
        input_data.jump = true;
    }
    if keypress.pressed(KeyCode::ControlLeft) {
        input_data.crouch = true;
    }

    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.read() {
        mouse_delta += mouse_event.delta;
    }
    mouse_delta *= 0.001;

    input_data.pitch = (input_data.pitch - mouse_delta.y)
      .clamp(-FRAC_PI_2 + ANGLE_EPSILON, FRAC_PI_2 - ANGLE_EPSILON);
    input_data.yaw -= mouse_delta.x;
    if input_data.yaw.abs() > PI {
        input_data.yaw = input_data.yaw.rem_euclid(TAU);
    }

    if input_data.movement.x != 0.0 || input_data.movement.z != 0.0 {
        input_data.movement = input_data.movement.normalize();
    }

    if buttons.pressed(MouseButton::Left) {
        input_data.shoot = true;
    }

    input = Inputs::Input(input_data);
    input_manager.add_input(input, tick)
}

pub(crate) fn receive_entity_spawn(
    mut commands: Commands,
    mut reader: EventReader<EntitySpawnEvent>,
    query: Query<&PlayerId>,
    mut client_data: ResMut<ClientData>,
    asset_server: Res<AssetServer>,
    loaded_animations: Res<LoadedAnimations>,
) {
    for event in reader.read() {
        let entity = event.entity();
        info!("Received entity spawn: {:?}", entity);
        if let Ok(player_id) = query.get(entity) {
            if player_id.0.to_bits() == client_data.client_id {
                info!("This is my entity!");
                let entity = fps_controller::insert_logical_entity_bundle(&mut commands, entity);
                commands.spawn(fps_controller::create_render_entity_bundle(entity));
                commands.entity(entity).insert(LocalPlayer);
                client_data.client_entity = Some(entity);
            } else {
                info!("This is not my entity!");
                crate::game_states::ingame::ingame::spawn_soldier(
                    &mut commands,
                    &asset_server,
                    "models/players/soldier_animated.glb".to_string(),
                    format!("Soldier {}", player_id.0.to_bits()),
                    Vec3::splat(0.0),
                    &loaded_animations,
                    entity,
                );
            }
        }
    }
}

fn on_player_input(
    // Event that will contain the inputs for the correct tick
    mut input_reader: EventReader<lightyear::prelude::client::InputEvent<Inputs>>,
    mut query: Query<&mut FpsControllerInput>,
    client_data: Res<ClientData>,
) {
    for input in input_reader.read() {
        if let Some(input) = input.input() {
            if let Some(entity) = client_data.client_entity {
                shared_input_behaviour(
                    &entity,
                    &input,
                    &mut query,
                );
            }
        }
    }
}

fn post_update_physics(
    transform_query: Query<&ReplicatedMoveData>,
    mut query: Query<(
        Entity,
        &mut FpsController,
        &mut Transform,
    )>,
) {
    for (entity, controller, mut transform) in query.iter_mut() {
        if let Ok(replicated_transform) = transform_query.get(entity) {
            transform.translation = replicated_transform.translation;
            transform.scale = replicated_transform.scale;
        }
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    players: Query<(&ReplicatedMoveData, &PlayerColor, &PlayerId)>,
    client_data: Res<ClientData>,
) {
    for (position, color, player_id) in &players {
        if client_data.client_id == player_id.0.to_bits() {
            continue;
        }
        gizmos.sphere(
            Isometry3d::new(
                position.translation + Vec3::new(0.0, EYE_HEIGHT_OFFSET, 0.0),
                Quat::default(),
            ),
            0.5,
            color.0,
        );
        let rotation = Quat::from_euler(EulerRot::YXZ, position.yaw, position.pitch, 0.0);
        gizmos.arrow(
            position.translation + Vec3::new(0.0, EYE_HEIGHT_OFFSET, 0.0),
            position.translation + Vec3::new(0.0, EYE_HEIGHT_OFFSET, 0.0) + rotation * Vec3::NEG_Z,
            color.0,
        );
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
    //fps_controller::fps_controller_move(&time, &physics_context, &mut query);
    fps_controller::fps_controller_shoot(&time, &physics_context, &mut query, &query_move_data, &mut entity_shot_event, &mut sound_event);
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

fn on_sound_from_server(
    mut commands: Commands,
    mut sound_event: EventReader<SoundEvent>,
    asset_server: Res<AssetServer>,
    mut query: Query<&ReplicatedSoundEffect, Added<ReplicatedSoundEffect>>,
) {
    for event in query.iter() {
        let emitter = event.emitter;
        let asset = event.asset.clone();
        let position = event.position;
        let volume = event.volume;
        let speed = event.speed;
        let spatial = event.spatial;
        let spatial_scale = event.spatial_scale;

        play_sound_effect(&mut commands, &asset_server, asset, position, volume, speed, spatial, spatial_scale);
    }
}

fn on_sound_event(
    mut commands: Commands,
    mut sound_event: EventReader<SoundEvent>,
    asset_server: Res<AssetServer>,
) {
    for event in sound_event.read() {
        let emitter = event.emitter;
        let asset = event.asset.clone();
        let position = event.position;
        let volume = event.volume;
        let speed = event.speed;
        let spatial = event.spatial;
        let spatial_scale = event.spatial_scale;

        // TODO: only sounds from server for now
        //play_sound_effect(&mut commands, &asset_server, asset, position, volume, speed, spatial);
    }
}

fn play_sound_effect(commands: &mut Commands, asset_server: &Res<AssetServer>, asset: String, position: Vec3, volume: f32, speed: f32, spatial: bool, spatial_scale: Option<f32>) {
    let settings = PlaybackSettings::DESPAWN
      .with_spatial(spatial)
      .with_speed(speed)
      .with_volume(Volume::new(volume));
    if let Some(scale) = spatial_scale {
        settings.with_spatial_scale(SpatialScale::new(scale));
    }
    commands.spawn((
        Transform::from_translation(position),
        AudioPlayer::new(
            asset_server.load(asset),
        ),
        settings,
    ));
}

fn update_fps_animations(
    mut gun_animation_state: Query<&mut fps_gun_plugin::GunAnimationState>,
    mut query: Query<(
        Entity,
        &FpsController,
        &FpsControllerInput,
        &ReplicatedMoveData,
    )>,
    client_data: Res<ClientData>,
) {
    if gun_animation_state.is_empty() {
        return;
    }
    let mut gun_animation_state = gun_animation_state.single_mut();
    for (entity, controller, input, move_data) in query.iter() {
        if client_data.client_entity == Some(entity) {
            if move_data.velocity.y.abs() <= 0.01 {
                if move_data.velocity.length_squared() > 0.01 {
                    gun_animation_state.walking = true;
                } else {
                    gun_animation_state.walking = false;
                }
            }
            if input.shoot {
                gun_animation_state.shooting = true;
            } else {
                gun_animation_state.shooting = false;
            }
        }
    }

}