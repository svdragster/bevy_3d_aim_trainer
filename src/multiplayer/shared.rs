use lightyear::prelude::*;
use std::time::Duration;
use bevy::prelude::*;
use crate::multiplayer::protocol::{Inputs, PlayerColor, PlayerTransform};

pub const KEY: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0,
];
pub const PROTOCOL_ID: u64 = 1;

pub fn shared_config(mode: Mode) -> SharedConfig {
    SharedConfig {
        // how often does the server send replication updates to the client?
        // A duration of 0 means that we send replication updates every frame
        server_replication_send_interval: Duration::from_millis(0),
        tick: TickConfig {
            tick_duration: Duration::from_secs_f64(1.0 / 64.0),
        },
        // Here we make the `Mode` an argument so that we can run `lightyear` either in `Separate` mode (distinct client and server apps)
        // or in `HostServer` mode (the server also acts as a client).
        mode,
    }
}

pub(crate) fn shared_movement_behaviour(mut transform: Mut<PlayerTransform>, input: &Inputs) {
    const MOVE_SPEED: f32 = 0.1;
    match input {
        Inputs::Input(input_data) => {
            transform.0.translation += input_data.movement * MOVE_SPEED;
        }
        _ => {}
    }
}

pub(crate) fn draw_boxes(
    mut gizmos: Gizmos,
    players: Query<(&PlayerTransform, &PlayerColor)>,
) {
    for (position, color) in &players {
        gizmos.sphere(
            Isometry3d::new(
                position.0.translation + Vec3::new(0.0, 1.0, 0.0),
                Quat::default(),
            ),
            0.5,
            color.0,
        );
    }
}