use bevy::audio::SpatialScale;
use bevy::prelude::*;
use lightyear::prelude::client::ComponentSyncMode;
use lightyear::prelude::*;

/// A component that will identify which player the entity belongs to
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub ClientId);

/// A component that will store the transform of the player
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReplicatedMoveData {
    pub translation: Vec3,
    pub scale: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

/// A component that will store the color of the entity, so that each player can have a different color.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub Color);

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ReplicatedSoundEffect {
    pub emitter: Option<Entity>,
    pub asset: String,
    pub position: Vec3,
    pub volume: f32,
    pub speed: f32,
    pub spatial: bool,
    pub spatial_scale: Option<f32>,
}

#[derive(Event)]
pub struct SoundEvent {
    pub emitter: Option<Entity>,
    pub asset: String,
    pub position: Vec3,
    pub volume: f32,
    pub speed: f32,
    pub spatial: bool,
    pub spatial_scale: Option<f32>,
}

//#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
//pub struct BulletImpact(pub Color);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMessage(pub String);

#[derive(Channel)]
pub struct ChatChannel;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Events
        app.add_event::<SoundEvent>();
        app.add_systems(PreUpdate, remove_sound_effects);

        // Messages
        //app.add_message::<ChatMessage>(ChannelDirection::Bidirectional);

        // Inputs
        app.add_plugins(InputPlugin::<Inputs>::default());

        // Components
        app.register_component::<PlayerId>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);

        app.register_component::<ReplicatedMoveData>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation(ComponentSyncMode::Full)
            .add_interpolation_fn(|start, other, t: f32| {
                let mut interpolated = start.clone();
                interpolated.translation = interpolated.translation.lerp(other.translation, t);
                interpolated.scale = interpolated.scale.lerp(other.scale, t);
                interpolated.velocity = start.velocity.lerp(other.velocity, t);
                interpolated
            });

        app.register_component::<PlayerColor>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);

        app.register_component::<ReplicatedSoundEffect>(ChannelDirection::ServerToClient)
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
    pub shoot: bool,
    pub movement: Vec3,
    pub pitch: f32,
    pub yaw: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Inputs {
    Input(InputData),
    Delete,
    Spawn,
    /// NOTE: we NEED to provide a None input so that the server can distinguish between lost input packets and 'None' inputs
    None,
}

fn remove_sound_effects(
    mut commands: Commands,
    mut query: Query<Entity, With<ReplicatedSoundEffect>>,
) {
    for (entity) in query.iter_mut() {
        commands.entity(entity).despawn();
    }
}