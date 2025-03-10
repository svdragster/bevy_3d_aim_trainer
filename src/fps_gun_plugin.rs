use crate::FpsControllerSetup;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::render::view::RenderLayers;
use bevy::scene::SceneInstanceReady;
use bevy_fps_controller::controller::{CameraConfig, FpsController, LogicalPlayer, RenderPlayer};
use bevy_rapier3d::geometry::Collider;
use std::f32::consts::{PI, TAU};
use std::time::Duration;
use bevy::animation::RepeatAnimation;

pub struct FpsGunPlugin;

impl Plugin for FpsGunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup.after(FpsControllerSetup),));
        app.add_systems(Update, (move_listener, on_fps_gun_animation));
    }
}

#[derive(Component, Clone, Debug, Eq, PartialEq, Hash)]
pub struct GunAnimationState {
    pub walking: bool,
    pub shooting: bool,
    pub reloading: bool,
    pub previous_state: Option<Box<GunAnimationState>>,
}
/// Used by the view model camera and the player's arm.
/// The light source belongs to both layers.
const VIEW_MODEL_RENDER_LAYER: usize = 1;

#[derive(Component)]
pub struct ViewModelRenderPlayer;

#[derive(Component, Clone)]
pub struct FpsGunAnimationsData {
    pub default_animation_index: usize,
    pub animations: Vec<AnimationNodeIndex>,
    pub graph: Handle<AnimationGraph>,
    pub current_animation_index: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GunAnimations {
    Idle = 0,
    Shooting = 1,
    Walking = 2,
    //Reloading = 3,
}

impl Default for GunAnimations {
    fn default() -> Self {
        GunAnimations::Idle
    }
}

impl GunAnimations {
    fn all() -> Vec<GunAnimations> {
        vec![
            GunAnimations::Idle,
            GunAnimations::Shooting,
            GunAnimations::Walking,
        ]
    }

    fn all_indices() -> Vec<usize> {
        Self::all().into_iter().map(|anim| anim as usize).collect()
    }

    fn get_speed(&self) -> f32 {
        match self {
            GunAnimations::Idle => 1.0,
            GunAnimations::Walking => 1.0,
            GunAnimations::Shooting => 2.5,
        }
    }
}

#[derive(Component, Clone)]
pub struct LastPosition {
    pub last_position: Vec3,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    commands.spawn((
        ViewModelRenderPlayer,
        Camera3d::default(),
        Camera {
            // Bump the order to render on top of the world model.
            order: 1,
            ..default()
        },
        Projection::from(PerspectiveProjection {
            fov: 80.0_f32.to_radians(),
            ..default()
        }),
        // Only render objects belonging to the view model.
        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
    ));

    // Spawn the gun
    spawn_gun(
        &mut commands,
        &asset_server,
        &mut graphs,
        "ak47_animated.glb",
    );

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        // The light source illuminates both the world model and the view model.
        RenderLayers::from_layers(&[VIEW_MODEL_RENDER_LAYER]),
    ));
}

fn spawn_gun(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    graphs: &mut ResMut<Assets<AnimationGraph>>,
    asset_path: &str,
) {
    let (graph, node_indices) = AnimationGraph::from_clips(
        GunAnimations::all_indices()
            .iter()
            .map(|&index| {
                asset_server
                    .load(GltfAssetLabel::Animation(index).from_asset(asset_path.to_string()))
            })
            .collect::<Vec<_>>(),
    );

    let graph_handle = graphs.add(graph);
    let animations = FpsGunAnimationsData {
        default_animation_index: GunAnimations::Idle as usize,
        animations: node_indices,
        graph: graph_handle,
        current_animation_index: GunAnimations::Idle as usize,
    };

    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset_path.to_string()));
    commands
        .spawn((
            SceneRoot(scene),
            Transform {
                translation: Vec3::new(1.0, -1.0, -1.5),
                scale: Vec3::splat(0.15),
                rotation: Quat::from_euler(EulerRot::XYX, 0.0, -PI, 0.0),
            },
            RenderLayers::from_layers(&[VIEW_MODEL_RENDER_LAYER]),
            animations,
        ))
        .observe(on_gun_loaded);
}

fn on_gun_loaded(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children_query: Query<&Children>,
    animations: Query<&FpsGunAnimationsData>,
    mut players: Query<&mut AnimationPlayer>,
    children: Query<&Children>,
) {
    let entity = trigger.entity();
    let scene_instance_entity = children.get(trigger.entity()).unwrap()[0];
    let entity_to_animate = children.get(scene_instance_entity).unwrap()[0];
    let animations = animations.get(trigger.entity()).unwrap();
    for child in children_query.iter_descendants(entity) {
        commands.entity(child).log_components();
        commands.entity(child).insert((
            // Ensure the gun is only rendered by the view model camera.
            RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            // The gun is free-floating, so shadows would look weird.
            NotShadowCaster,
        ));
    }

    let mut animation_player = players.get_mut(entity_to_animate).unwrap();
    let mut transitions = AnimationTransitions::new();

    transitions
        .play(
            &mut animation_player,
            animations.animations[animations.default_animation_index],
            Duration::ZERO,
        )
        .repeat();

    animation_player.adjust_speeds(1.0);

    commands.entity(entity_to_animate).insert((
        AnimationGraphHandle(animations.graph.clone()),
        transitions,
        GunAnimationState {
            walking: false,
            shooting: false,
            reloading: false,
            previous_state: Some(
                GunAnimationState {
                    walking: false,
                    shooting: false,
                    reloading: false,
                    previous_state: None,
                }
                .into(),
            ),
        },
    ));
}

fn move_listener(
    mut player_query: Query<(Entity, &Transform, &mut LastPosition), With<LogicalPlayer>>,
    mut gun_animation_state: Query<&mut GunAnimationState>,
) {
    let (player_entity, transform, mut last_position) = player_query.get_single_mut().unwrap();
    let current_position = transform.translation;
    let delta = current_position - last_position.last_position;
    last_position.last_position = current_position;
    let mut animation = GunAnimations::Idle;
    if let Ok(mut gun_animation_state) = gun_animation_state.get_single_mut() {
        println!("{}", delta.length());
        if delta.length_squared() > 0.02 * 0.02 {
            gun_animation_state.walking = true;
        } else {
            gun_animation_state.walking = false;
        }
    }
}

fn on_fps_gun_animation(
    mut animation_query: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut GunAnimationState,
    )>,
    mut animations: Query<&mut FpsGunAnimationsData>,
) {
    if let Ok((mut animation_player, mut transitions, mut state)) = animation_query.get_single_mut()
    {
        let previous_state = state.previous_state.clone();
        let mut animations = animations.get_single_mut().unwrap();
        let mut duration = 0;
        if let Some(previous_state) = previous_state {
            let mut new_animation: Option<GunAnimations> = None;
            if state.shooting {
                if !previous_state.shooting {
                    new_animation = Some(GunAnimations::Shooting);
                    duration = 100;
                }
            } else if state.walking {
                if !previous_state.walking
                    || animations.current_animation_index == GunAnimations::Shooting as usize
                {
                    new_animation = Some(GunAnimations::Walking);
                    duration = 200;
                }
            } else if !state.walking && !state.shooting {
                new_animation = Some(GunAnimations::Idle);
                duration = 200;
            }
            if let Some(new_animation) = new_animation {
                if animations.current_animation_index != new_animation as usize {
                    // Idle animation
                    transitions
                        .play(
                            &mut animation_player,
                            animations.animations[new_animation as usize],
                            Duration::from_millis(duration),
                        )
                        .repeat();
                    for (index, active_animation) in animation_player.playing_animations_mut() {
                        active_animation.set_speed(new_animation.get_speed());
                    }
                    animations.current_animation_index = new_animation as usize;
                }
            }
        }
        state.previous_state = Some(Box::new(state.clone()));
    }
}
