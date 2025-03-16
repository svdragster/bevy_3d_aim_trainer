use crate::multiplayer::shared::{draw_boxes, shared_config, shared_movement_behaviour, KEY, PROTOCOL_ID};
use bevy::prelude::*;
use bevy::utils::HashMap;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use rand::Rng;
use crate::multiplayer::protocol::{Inputs, PlayerColor, PlayerId, PlayerTransform};

pub struct FpsServerPlugin;

impl Plugin for FpsServerPlugin {
    fn build(&self, app: &mut App) {
        let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 25565);
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
        .with_conditioner(link_conditioner);
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
        app.add_systems(Startup, start_server);
        app.add_systems(Update, (handle_connections, draw_boxes));
        app.add_systems(FixedUpdate, movement);
        app.insert_resource(Global {
            client_id_to_entity_id: HashMap::default(),
        });

        //app.add_event::<InputEvent<Inputs>>();
    }
}

fn start_server(mut commands: Commands) {
    commands.start_server();
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

        // We add the `Replicate` bundle to start replicating the entity to clients
        // By default, the entity will be replicated to all clients
        let entity = commands.spawn((
            PlayerId(client_id),
            PlayerTransform(Transform::default()),
            PlayerColor(color),
            Replicate::default(),
        ));

        // Add a mapping from client id to entity id
        global.client_id_to_entity_id.insert(client_id, entity.id());
    }
}

fn movement(
    mut transform_query: Query<&mut PlayerTransform>,
    // Event that will contain the inputs for the correct tick
    mut input_reader: EventReader<InputEvent<Inputs>>,
    // Retrieve the entity associated with a given client
    global: Res<Global>,
) {
    for input in input_reader.read() {
        let client_id = input.from();
        println!("Received input from client: {:?}", client_id);
        if let Some(input) = input.input() {
            if let Some(player_entity) = global.client_id_to_entity_id.get(&client_id) {
                if let Ok(transform) = transform_query.get_mut(*player_entity) {
                    shared_movement_behaviour(transform, input);
                }
            }
        }
    }
}
