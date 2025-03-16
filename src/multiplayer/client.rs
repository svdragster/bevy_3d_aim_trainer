use crate::multiplayer::protocol::{InputData, Inputs, PlayerTransform};
use crate::multiplayer::server::Global;
use crate::multiplayer::shared::{
    draw_boxes, shared_config, shared_movement_behaviour, KEY, PROTOCOL_ID,
};
use bevy::prelude::*;
use lightyear::client::input::native::InputSystemSet;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use rand::Rng;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

pub struct FpsClientPlugin {
    pub port: u16,
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
        let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), self.port);
        let io_config = IoConfig::from_transport(client::ClientTransport::UdpSocket(client_addr))
            .with_conditioner(link_conditioner);

        let mut rng = rand::rng();
        let client_id = rng.random_range(0..=u64::MAX);

        let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 25565);
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
        app.add_systems(FixedUpdate, player_movement);
        app.add_systems(Update, (draw_boxes, receive_entity_spawn));
        //app.insert_resource(InputManager::<Inputs>::default());
    }
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
) {
    let tick = tick_manager.tick();
    let mut input = Inputs::None;
    let mut input_data = InputData {
        fly: false,
        sprint: false,
        jump: false,
        crouch: false,
        pitch: 0.0,
        yaw: 0.0,
        movement: Vec3::ZERO,
    };
    let mut has_input = false;
    if keypress.pressed(KeyCode::KeyW) {
        input_data.movement.z += 1.0;
        has_input = true;
    }
    if keypress.pressed(KeyCode::KeyS) {
        input_data.movement.z -= 1.0;
        has_input = true;
    }
    if keypress.pressed(KeyCode::KeyA) {
        input_data.movement.x -= 1.0;
        has_input = true;
    }
    if keypress.pressed(KeyCode::KeyD) {
        input_data.movement.x += 1.0;
        has_input = true;
    }
    if has_input {
        if input_data.movement.x != 0.0 || input_data.movement.z != 0.0 {
            input_data.movement = input_data.movement.normalize();
        }
        input = Inputs::Input(input_data);
    }
    input_manager.add_input(input, tick)
}

pub(crate) fn receive_entity_spawn(mut reader: EventReader<EntitySpawnEvent>) {
    for event in reader.read() {
        info!("Received entity spawn: {:?}", event.entity());
    }
}

fn player_movement(
    mut transform_query: Query<&mut PlayerTransform>,
    // Event that will contain the inputs for the correct tick
    mut input_reader: EventReader<lightyear::prelude::client::InputEvent<Inputs>>,
) {
    for input in input_reader.read() {
        if let Some(input) = input.input() {
            //shared_movement_behaviour(transform_query.single_mut(), input);
        }
    }
}
