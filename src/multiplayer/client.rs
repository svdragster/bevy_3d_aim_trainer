use std::f32::consts::{FRAC_PI_2, PI, TAU};
use crate::multiplayer::protocol::{InputData, Inputs, PlayerColor, PlayerId, ReplicatedTransform};
use crate::multiplayer::shared::{
    shared_config, shared_movement_behaviour, KEY, PROTOCOL_ID,
};
use bevy::prelude::*;
use lightyear::client::input::native::InputSystemSet;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use rand::Rng;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use bevy::input::mouse::MouseMotion;
use crate::fps_controller::fps_controller;
use crate::fps_controller::fps_controller::{FpsController, FpsControllerInput, ANGLE_EPSILON};

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
            player_movement,
            post_update_physics,
        ).chain());
        app.add_systems(Update, (draw_gizmos, receive_entity_spawn));
        app.insert_resource(ClientData { client_id, client_entity: None });
    }
}

#[derive(Resource)]
pub struct ClientData {
    pub client_id: u64,
    pub client_entity: Option<Entity>,
}

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
    query_fps_controller_input: Query<&FpsControllerInput>,
) {
    let tick = tick_manager.tick();
    if query_fps_controller_input.is_empty() {
        return;
    }
    let fps_controller_input = query_fps_controller_input.single();
    let mut input = Inputs::None;
    let mut input_data = InputData {
        fly: false,
        sprint: false,
        jump: false,
        crouch: false,
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

    input = Inputs::Input(input_data);
    input_manager.add_input(input, tick)
}

pub(crate) fn receive_entity_spawn(
    mut commands: Commands,
    mut reader: EventReader<EntitySpawnEvent>,
    query: Query<&PlayerId>,
    mut client_data: ResMut<ClientData>,
) {
    for event in reader.read() {
        let entity = event.entity();
        info!("Received entity spawn: {:?}", entity);
        if let Ok(player_id) = query.get(entity) {
            if player_id.0.to_bits() == client_data.client_id {
                info!("This is my entity!");
                let entity = fps_controller::insert_logical_entity_bundle(&mut commands, entity);
                commands.spawn(fps_controller::create_render_entity_bundle(entity));
                client_data.client_entity = Some(entity);
            } else {
                info!("This is not my entity!");
            }
        } else {
            info!("Entity does not have a PlayerId component");
        }
    }
}

fn player_movement(
    // Event that will contain the inputs for the correct tick
    mut input_reader: EventReader<lightyear::prelude::client::InputEvent<Inputs>>,
    mut query: Query<&mut FpsControllerInput>,
    client_data: Res<ClientData>,
) {
    for input in input_reader.read() {
        if let Some(input) = input.input() {
            if let Some(entity) = client_data.client_entity {
                shared_movement_behaviour(
                    &entity,
                    &input,
                    &mut query,
                );
            }
        }
    }
}

fn post_update_physics(
    transform_query: Query<&ReplicatedTransform>,
    mut query: Query<(
        Entity,
        &mut FpsController,
        &mut Transform,
    )>,
) {
    for (entity, controller, mut transform) in query.iter_mut() {
        if let Ok(replicated_transform) = transform_query.get(entity) {
            transform.translation = replicated_transform.0.translation;
            //controller.pitch = replicated_transform.0.rotation.to_euler(EulerRot::YXZ).1;
            //controller.yaw = replicated_transform.0.rotation.to_euler(EulerRot::YXZ).0;
            transform.scale = replicated_transform.0.scale;
        }
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    players: Query<(&ReplicatedTransform, &PlayerColor, &PlayerId)>,
    client_data: Res<ClientData>,
) {
    for (position, color, player_id) in &players {
        if client_data.client_id == player_id.0.to_bits() {
            continue;
        }
        gizmos.sphere(
            Isometry3d::new(
                position.0.translation + Vec3::new(0.0, 1.0, 0.0),
                Quat::default(),
            ),
            0.5,
            color.0,
        );
        gizmos.arrow(
            position.0.translation + Vec3::new(0.0, 1.0, 0.0),
            position.0.translation + Vec3::new(0.0, 1.0, 0.0) + position.0.forward().as_vec3(),
            color.0,
        );
    }
}
