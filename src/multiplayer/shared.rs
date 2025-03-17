use crate::fps_controller::fps_controller;
use crate::fps_controller::fps_controller::{FpsController, FpsControllerInput};
use crate::multiplayer::protocol::{Inputs, PlayerColor, ReplicatedTransform};
use bevy::prelude::*;
use bevy_rapier3d::dynamics::Velocity;
use bevy_rapier3d::geometry::Collider;
use bevy_rapier3d::plugin::ReadRapierContext;
use lightyear::prelude::*;
use std::time::Duration;

pub const KEY: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
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

pub(crate) fn shared_movement_behaviour(
    entity_to_update: &Entity,
    input: &Inputs,
    mut query: &mut Query<&mut FpsControllerInput>,
) {
    match input {
        Inputs::Input(input_data) => {
            if let Ok(mut fps_controller_input) = query.get_mut(*entity_to_update) {
                fps_controller_input.movement = input_data.movement.clone();
                fps_controller_input.jump = input_data.jump;
                fps_controller_input.sprint = input_data.sprint;
                fps_controller_input.crouch = input_data.crouch;
                fps_controller_input.fly = input_data.fly;

                fps_controller_input.pitch += input_data.pitch;
                fps_controller_input.yaw += input_data.yaw;

                println!("pitch: {}, yaw: {}", input_data.pitch, input_data.yaw);
            }
        }
        _ => {}
    }
}

pub(crate) fn draw_boxes(mut gizmos: Gizmos, players: Query<(&ReplicatedTransform, &PlayerColor)>) {
    for (position, color) in &players {
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
            position.0.translation + Vec3::new(0.0, 1.0, 0.0) + position.0.rotation * Vec3::Z,
            color.0,
        );
    }
}
