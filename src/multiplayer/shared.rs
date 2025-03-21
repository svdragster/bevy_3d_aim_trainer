use crate::fps_controller::fps_controller::FpsControllerInput;
use crate::multiplayer::protocol::Inputs;
use bevy::prelude::*;
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

                fps_controller_input.yaw = input_data.yaw;
                fps_controller_input.pitch = input_data.pitch;
            }
        }
        _ => {}
    }
}

