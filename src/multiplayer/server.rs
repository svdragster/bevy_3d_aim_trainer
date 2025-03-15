use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use crate::multiplayer::shared::{shared_config, KEY, PROTOCOL_ID};

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
        let io_config = lightyear::connection::server::IoConfig::from_transport(server::ServerTransport::UdpSocket(server_addr))
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
        app.add_systems(Startup, init);
    }
}



fn init(mut commands: Commands) {
    commands.start_server();
}