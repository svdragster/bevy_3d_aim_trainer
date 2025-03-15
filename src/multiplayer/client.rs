use crate::multiplayer::shared::{shared_config, KEY, PROTOCOL_ID};
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use rand::Rng;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

pub struct FpsClientPlugin;

impl Plugin for FpsClientPlugin {
    fn build(&self, app: &mut App) {
        // You can add a link conditioner to simulate network conditions
        let link_conditioner = LinkConditionerConfig {
            incoming_latency: Duration::from_millis(100),
            incoming_jitter: Duration::from_millis(0),
            incoming_loss: 0.00,
        };
        // Here we use the `UdpSocket` transport layer, with the link conditioner
        let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 25565);
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
    }
}

fn init(mut commands: Commands) {
    commands.connect_client();
}