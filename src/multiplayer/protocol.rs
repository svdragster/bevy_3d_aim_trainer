use bevy::prelude::*;
use lightyear::prelude::client::ComponentSyncMode;
use lightyear::prelude::*;

/// A component that will identify which player the box belongs to
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(ClientId);

/// A component that will store the position of the box. We could also directly use the `Transform` component.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerPosition(Vec3);

/// A component that will store the color of the box, so that each player can have a different color.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMessage(pub String);

#[derive(Channel)]
pub struct ChatChannel;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Messages
        //app.add_message::<ChatMessage>(ChannelDirection::Bidirectional);

        // Inputs
        app.add_plugins(InputPlugin::<Inputs>::default());

        // Components
        app.register_component::<PlayerId>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);

        app.register_component::<PlayerPosition>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation(ComponentSyncMode::Full);

        app.register_component::<PlayerColor>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);

        // Channels
        app.add_channel::<ChatChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        });

        // Client

    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InputData {
    pub fly: bool,
    pub sprint: bool,
    pub jump: bool,
    pub crouch: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub movement: Vec3,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Inputs {
    Input(InputData),
    Delete,
    Spawn,
    /// NOTE: we NEED to provide a None input so that the server can distinguish between lost input packets and 'None' inputs
    None,
}
