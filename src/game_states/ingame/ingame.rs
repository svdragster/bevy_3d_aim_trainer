use crate::animations::animated_entity_plugin::*;
use crate::animations::{animated_entity_plugin, look_plugin};
use crate::fps_controller::fps_controller::*;
use crate::fps_gun_plugin::FpsGunPlugin;
use crate::game_states::game_states::InGameState;
use crate::multiplayer::protocol::{ReplicatedAnimationData, ReplicatedMoveData};
use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use bevy_rapier3d::dynamics::RigidBody;
use bevy_rapier3d::geometry::Collider;
use std::f32::consts::PI;
use std::time::Duration;
use clap::command;
use crate::animations::look_plugin::{LookPlugin, VerticalLook, VerticalLookAnchor};

pub struct IngamePlugin {
    pub is_client: bool,
}

impl Plugin for IngamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
          OnEnter(InGameState),
          (setup_world,),
        );
        app.add_systems(Update, (update_soldier_translation, update_soldier_animation));
        app.add_plugins(FpsControllerPlugin)
          .add_plugins(FpsGunPlugin {
              is_client: self.is_client,
          })
          .add_plugins(AnimatedEntityPlugin)
          .add_plugins(LookPlugin);
    }
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut materials2d: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 14.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        StateScoped(InGameState),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            ..default()
        },
        StateScoped(InGameState),
    ));

    // Ground collider
    commands.spawn((
        Collider::cuboid(20.0, 0.1, 20.0),
        RigidBody::Fixed,
        Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
        StateScoped(InGameState),
    ));
    // Ground mesh
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        ..Default::default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 0.1, 40.0))),
        MeshMaterial3d(ground_material.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
        StateScoped(InGameState),
    ));

    // Wall
    commands.spawn((
        Collider::cuboid(5.0, 2.5, 0.5),
        RigidBody::Fixed,
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        StateScoped(InGameState),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 5.0, 1.0))),
        MeshMaterial3d(ground_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        StateScoped(InGameState),
    ));

    //spawn_random_target(&mut commands, &mut meshes, &mut materials);
    //spawn_random_target(&mut commands, &mut meshes, &mut materials);
    //spawn_random_target(&mut commands, &mut meshes, &mut materials);

    // Crosshair
    let color = Color::srgb(0.5, 0.7, 1.0);
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(2.0))),
        MeshMaterial2d(materials2d.add(color)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        StateScoped(InGameState),
    ));

    /*commands.spawn((
        // Here we are able to call the `From` method instead of creating a new `TextSection`.
        // This will use the default font (a minimal subset of FiraMono) and apply the default styling.
        Text::new("From an &str into a Text with the default font!"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
        PointsDisplay,
    ));*/
}

#[derive(Component)]
struct MySoldier {
    pub vertical_rotation: f32,
    pub horizontal_rotation: f32,
    pub parent: Entity,
}

pub fn spawn_soldier(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    scene_path: String,
    name: String,
    translation: Vec3,
    loaded_animations: &Res<LoadedAnimations>,
    parent: Entity,
) -> Option<Entity> {
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(scene_path.to_string()));
    if let Some(soldier_animations) = loaded_animations.animations.get(scene_path.as_str()) {
        let soldier = commands
          .spawn((
              Name::from(name),
              SceneRoot(scene),
              soldier_animations.clone(),
              Transform {
                  translation,
                  ..default()
              },
              look_plugin::VerticalLook {
                  node_name: "mixamorig:Spine",
                  anchor: None,
              },
              MySoldier {
                  vertical_rotation: 0.0,
                  horizontal_rotation: 0.0,
                  parent,
              },
              NoFrustumCulling,
          ))
          .observe(animated_entity_plugin::initialize_animation)
          .observe(look_plugin::setup_vertical_look)
          .id();

        println!("spawned soldier: {:?}", soldier);

        Some(soldier)
    } else {
        None
    }
}

fn update_soldier_translation(
    mut soldier_query: Query<(&mut Transform, &MySoldier, &mut VerticalLook), With<MySoldier>>,
    move_data_query: Query<&ReplicatedMoveData, Without<MySoldier>>,
    mut vertical_look_anchor_query: Query<&mut VerticalLookAnchor>,
) {
    for (mut soldier_transform, soldier, mut vertical_look) in soldier_query.iter_mut() {
        let parent_move_data = move_data_query.get(soldier.parent);
        if let Ok(parent_move_data) = parent_move_data {
            soldier_transform.translation = parent_move_data.translation.clone();
            soldier_transform.translation.y -= 1.6; // TODO: magic number
            soldier_transform.rotation = Quat::from_euler(EulerRot::YXZ, parent_move_data.yaw + PI, 0.0, 0.0);
            if let Some(anchor_entity) = vertical_look.anchor {
                if let Ok(mut vertical_look_anchor) = vertical_look_anchor_query.get_mut(anchor_entity) {
                    vertical_look_anchor.vertical_rotation = Quat::from_euler(EulerRot::YXZ, 0.0, -parent_move_data.pitch, 0.0).x;
                }
            }
        }
    }
}

fn update_soldier_animation(
    mut commands: Commands,
    mut soldier_query: Query<(Entity, &MySoldier, &mut Animations, &AnimatedEntity), With<MySoldier>>,
    animation_data_query: Query<&ReplicatedAnimationData, Without<MySoldier>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (soldier_entity, soldier, mut animations, animated_entity) in soldier_query.iter_mut() {
        let parent_animation_data = animation_data_query.get(soldier.parent);
        if let Ok(parent_animation_data) = parent_animation_data {
            if let Ok((mut animation_player, mut transitions)) = animation_players.get_mut(animated_entity.animation_player_entity) {
                if parent_animation_data.animation_index != animations.current_animation_index {
                    animations.current_animation_index = parent_animation_data.animation_index;
                    transitions
                      .play(
                          &mut animation_player,
                          animations.animations[parent_animation_data.animation_index],
                          Duration::from_millis(400),
                      )
                      .repeat();

                    // TODO hacky workaround
                    let mut speed = 1.0;
                    if parent_animation_data.animation_index == SoldierAnimations::Walking as usize {
                        speed = 1.8;
                    } else if parent_animation_data.animation_index == SoldierAnimations::WalkingBack as usize {
                        speed = 1.4;
                    }
                    for (_, active_animation) in animation_player.playing_animations_mut() {
                        active_animation.set_speed(speed);
                    }
                }
            }
        }
    }
}