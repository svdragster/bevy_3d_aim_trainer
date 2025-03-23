use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use std::time::Duration;

pub struct AnimatedEntityPlugin;

impl Plugin for AnimatedEntityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LoadedAnimations {
            animations: std::collections::HashMap::new(),
        });
        app.add_systems(Startup, (load_animations,).in_set(FpsAnimationsSetup));
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct FpsAnimationsSetup;


#[derive(Resource)]
pub struct LoadedAnimations {
    pub animations: std::collections::HashMap<String, Animations>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SoldierAnimations {
    Idle = 0,
    Walking = 1,
    WalkingBack = 2,
    WalkingLeft = 3,
    WalkingRight = 4,
}

impl Default for SoldierAnimations {
    fn default() -> Self {
        SoldierAnimations::Idle
    }
}

impl SoldierAnimations {
    fn all() -> Vec<SoldierAnimations> {
        vec![
            SoldierAnimations::Idle,
            SoldierAnimations::Walking,
            SoldierAnimations::WalkingBack,
            SoldierAnimations::WalkingLeft,
            SoldierAnimations::WalkingRight,
        ]
    }

    fn all_indices() -> Vec<usize> {
        Self::all().into_iter().map(|anim| anim as usize).collect()
    }
}

fn load_animations(
    asset_server: Res<AssetServer>,
    graphs: ResMut<Assets<AnimationGraph>>,
    mut animations_resource: ResMut<LoadedAnimations>,
) {
    let soldier_path = "models/players/soldier_animated.glb";
    let soldier_animations = setup_humanoid_animations(
        &asset_server,
        soldier_path,
        SoldierAnimations::all_indices(),
        graphs,
        SoldierAnimations::Idle as usize,
    );
    animations_resource
        .animations
        .insert(soldier_path.to_string(), soldier_animations);
}

///////////////////////////////
///////////////////////////////
///////////////////////////////

#[derive(Component)]
pub struct AnimatedEntity {
    pub animation_player_entity: Entity,
}

#[derive(Component, Clone)]
pub struct Animations {
    pub default_animation_index: usize,
    pub animations: Vec<AnimationNodeIndex>,
    pub graph: Handle<AnimationGraph>,
    pub current_animation_index: usize,
}

///////////////////////////////
///////////////////////////////
///////////////////////////////

pub fn setup_humanoid_animations(
    asset_server: &Res<AssetServer>,
    asset_path: &str,
    animation_indices: impl Into<Vec<usize>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    default_animation_index: usize,
) -> Animations {
    let (graph, node_indices) =
        _build_animation_graph(&asset_server, asset_path, animation_indices);
    let graph_handle = graphs.add(graph);
    Animations {
        default_animation_index,
        animations: node_indices,
        graph: graph_handle,
        current_animation_index: default_animation_index,
    }
}

fn _build_animation_graph(
    asset_server: &Res<AssetServer>,
    asset_path: &str,
    animation_indices: impl Into<Vec<usize>>,
) -> (AnimationGraph, Vec<AnimationNodeIndex>) {
    AnimationGraph::from_clips(
        animation_indices
            .into()
            .iter()
            .map(|&index| {
                asset_server
                    .load(GltfAssetLabel::Animation(index).from_asset(asset_path.to_string()))
            })
            .collect::<Vec<_>>(),
    )
}

///////////////////////////////
///////////////////////////////
///////////////////////////////

pub fn initialize_animation(
    trigger: Trigger<SceneInstanceReady>,
    children: Query<&Children>,
    animations: Query<&Animations>,
    mut commands: Commands,
    mut players: Query<&mut AnimationPlayer>,
) {
    let trigger_entity = trigger.entity();
    let entity = children.get(trigger_entity).unwrap()[0];
    let player_entity = children.get(entity).unwrap()[0];
    let animations = animations.get(trigger_entity).unwrap();

    let mut player = players.get_mut(player_entity).unwrap();
    let mut transitions = AnimationTransitions::new();
    transitions
        .play(
            &mut player,
            animations.animations[animations.default_animation_index],
            Duration::ZERO,
        )
        .repeat();

    commands
        .entity(player_entity)
        .insert((AnimationGraphHandle(animations.graph.clone()), transitions));

    commands.entity(trigger_entity).insert(AnimatedEntity {
        animation_player_entity: player_entity,
    });
}
