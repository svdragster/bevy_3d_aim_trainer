mod fps_controller;
mod fps_gun_plugin;
mod multiplayer;
mod game_states;
mod game_modes;
mod animations;

use bevy::audio::{SpatialScale, Volume};
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::time::Stopwatch;
use bevy::window::CursorGrabMode;
use bevy_rapier3d::prelude::*;
use clap::{Parser, Subcommand};
use fps_controller::fps_controller::*;
use rand::distr::Uniform;
use rand::prelude::*;
use std::f32::consts::TAU;
use std::net::IpAddr;
use crate::fps_gun_plugin::FpsGunPlugin;
use crate::game_states::game_states::{GameState, GameStatesPlugin};

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 0.0);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct FpsControllerSetup;

#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Target;

#[derive(Component)]
struct PointsDisplay;

#[derive(Default, Resource)]
struct Points {
    pub value: i32,
}

#[derive(Component)]
struct ShootTracker {
    stopwatch: Stopwatch,
    spray_count: usize,
}

#[derive(Component)]
struct BulletImpact {
    stopwatch: Stopwatch,
}

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Mode {
    Client {
        #[arg(long)]
        port: u16,
        #[arg(long)]
        ip: String,
    },
    Server,
}

#[derive(Resource)]
pub struct Global {
    pub mouse_captured: bool,
    pub is_server: bool,
}

fn main() {
    let cli = Cli::parse();
    let mut app = App::new();
    app.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 6000.0,
    })
    .insert_resource(ClearColor(Color::srgb(0.83, 0.96, 0.96)))
    .insert_resource(Points::default())
    .insert_resource(Global {
        mouse_captured: false,
        is_server: matches!(cli.mode, Mode::Server),
    })
    .add_plugins(DefaultPlugins)
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
    //.add_plugins(RapierDebugRenderPlugin::default())
    .add_systems(
        Startup,
        (setup,),
    )
    .add_systems(
        Update,
        (
            manage_cursor,
            //click_targets,
            //update_points_display,
            despawn_bullet_impacts,
        ),
    );

    // Multiplayer
    match cli.mode.clone() {
        Mode::Client { port, ip } => {
            let server_ip = ip.parse::<IpAddr>();
            if let Ok(server_ip) = server_ip {
                app.add_plugins(multiplayer::client::FpsClientPlugin { server_port: port, server_ip});
            } else {
                panic!("Invalid IP address: {}", ip);
            }
        }
        Mode::Server => {
            app.add_plugins(multiplayer::server::FpsServerPlugin);
        }
    }
    app.add_plugins(multiplayer::protocol::ProtocolPlugin {
        is_server: matches!(cli.mode, Mode::Server),
    });
    app.add_plugins(GameStatesPlugin);

    // Run the app
    app.run();
}


fn setup(
    mut commands: Commands,
    mut window: Query<&mut Window>,
) {
    let mut window = window.single_mut();

    let cli = Cli::parse();
    match cli.mode {
        Mode::Client { port: _port, ip: _ip } => {
            window.title = String::from("Multiplayer FPS Client");
        }
        Mode::Server => {
            window.title = String::from("Multiplayer FPS Server");
        }
    }

    commands.set_state(GameState::InGame {paused: false});

}

fn respawn(mut query: Query<(&mut Transform, &mut Velocity)>) {
    for (mut transform, mut velocity) in &mut query {
        if transform.translation.y > -50.0 {
            continue;
        }

        velocity.linvel = Vec3::ZERO;
        transform.translation = SPAWN_POINT;
    }
}

fn manage_cursor(
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window>,
    mut global: ResMut<Global>,
) {
    for mut window in &mut window_query {
        if btn.just_pressed(MouseButton::Left) {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
            global.mouse_captured = true;
        }
        if key.just_pressed(KeyCode::Escape) {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
            global.mouse_captured = false;
        }
    }
}



fn click_targets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    rapier_context: ReadRapierContext,
    player_query: Query<Entity, With<LogicalPlayer>>,
    camera: Query<&Transform, With<RenderPlayer>>,
    buttons: Res<ButtonInput<MouseButton>>,
    targets: Query<Entity, With<Target>>,
    mut points: ResMut<Points>,
    mut gun_animation_state: Query<&mut fps_gun_plugin::GunAnimationState>,
    mut shoot_stopwatch: Query<&mut ShootTracker>,
    time: Res<Time>,
) {
    if player_query.is_empty() {
        return;
    }
    let player_handle = player_query.single();
    let shoot_tracker = shoot_stopwatch
      .get_mut(player_handle);
    if shoot_tracker.is_err() {
        return;
    }
    let mut shoot_tracker = shoot_tracker
        .expect("LogicalPlayer also needs a ShootTracker");

    shoot_tracker.stopwatch.tick(time.delta());

    if let Ok(mut gun_animation_state) = gun_animation_state.get_single_mut() {
        if buttons.pressed(MouseButton::Left) {
            gun_animation_state.shooting = true;
        } else {
            gun_animation_state.shooting = false;
        }
    }
    if buttons.pressed(MouseButton::Left) {
        if shoot_tracker.stopwatch.elapsed_secs() > 0.1 {
            let rapier_context = rapier_context.single();
            let camera_transform = camera.single();
            let ray_pos = camera_transform.translation;
            let mut spray: Vec3;

            // Spray while holding left mouse button
            if shoot_tracker.spray_count >= SPRAY_DIRECTIONS.len() {
                let mut rng = rand::rng();
                let range = Uniform::new(-0.065f32, 0.065).unwrap();
                spray = Vec3::new(rng.sample(range), rng.sample(range), 0.0);
            } else {
                spray = SPRAY_DIRECTIONS[shoot_tracker.spray_count];
            }

            // Spray while walking
            if let Ok(gun_animation_state) = gun_animation_state.get_single() {
                if gun_animation_state.walking {
                    let mut rng = rand::rng();
                    let range = Uniform::new(-0.1f32, 0.1).unwrap();
                    spray += Vec3::new(rng.sample(range), rng.sample(range), 0.0);
                }
            }

            // Increment the spray count
            shoot_tracker.spray_count += 1;

            let mut rng = rand::rng();
            let pitch_range = Uniform::new(-0.12f32, 0.12).unwrap();

            commands.spawn((
                Transform::from_translation(ray_pos),
                AudioPlayer::new(
                    asset_server.load("sounds/weapons-rifle-assault-rifle-fire-01.ogg"),
                ),
                PlaybackSettings::DESPAWN
                    .with_spatial(true)
                    .with_speed(1.1 + rng.sample(pitch_range))
                    .with_volume(Volume::new(0.3)),
            ));

            let ray_dir = camera_transform.forward().as_vec3() + camera_transform.rotation * spray;
            let max_toi: bevy_rapier3d::math::Real = 100.0;
            let solid = true;
            let filter = QueryFilter::new()
                .exclude_sensors()
                .exclude_rigid_body(player_handle);

            if let Some((entity, toi)) =
                rapier_context.cast_ray(ray_pos, ray_dir, max_toi, solid, filter)
            {
                let hit_point = ray_pos + ray_dir * Vec3::splat(toi.into());
                println!("Hit entity {:?} at {:?}", entity, hit_point);
                commands.spawn((
                    BulletImpact {
                        stopwatch: Stopwatch::new(),
                    },
                    Transform::from_translation(hit_point),
                    Mesh3d(meshes.add(Sphere::new(0.1))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.0, 0.0),
                        ..Default::default()
                    })),
                ));

                commands.spawn((
                    Transform::from_translation(hit_point),
                    AudioPlayer::new(
                        asset_server.load("sounds/weapons-shield-metal-impact-ring-02.ogg"),
                    ),
                    PlaybackSettings::DESPAWN
                        .with_spatial(true)
                        .with_spatial_scale(SpatialScale::new(0.2))
                        .with_volume(Volume::new(0.35))
                        .with_speed(1.0 + rng.sample(pitch_range)),
                ));

                // Handle the hit.
                if let Ok(target_entity) = targets.get(entity) {
                    println!("Hit target entity {:?}", target_entity);
                    // Remove the target
                    commands.entity(entity).despawn_recursive();
                    // Spawn a new target
                    spawn_random_target(&mut commands, &mut meshes, &mut materials);
                    // Increment points
                    points.value += 1;
                } else {
                    points.value -= 1;
                }
            } else {
                points.value -= 1;
            }

            shoot_tracker.stopwatch.reset();
        }
    } else {
        shoot_tracker.spray_count = 0;
    }
}

fn spawn_random_target(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::rng();
    let range_x = Uniform::new(-4.0f32, 4.0).unwrap();
    let range_y = Uniform::new(2.0f32, 5.0).unwrap();
    let range_z = Uniform::new(1.0f32, 2.0).unwrap();
    let range_size = Uniform::new(0.3f32, 0.8).unwrap();
    let range_color = Uniform::new(0.1f32, 1.0).unwrap();
    let x = rng.sample(range_x);
    let y = rng.sample(range_y);
    let z = rng.sample(range_z);
    let size = rng.sample(range_size);
    let color = Color::srgb(
        rng.sample(range_color),
        rng.sample(range_color),
        rng.sample(range_color),
    );

    let target_material = materials.add(StandardMaterial {
        base_color: color,
        ..Default::default()
    });

    commands.spawn((
        Collider::ball(size),
        RigidBody::Fixed,
        Transform::from_translation(Vec3::new(x, y, z)),
        Target,
        Mesh3d(meshes.add(Sphere::new(size))),
        MeshMaterial3d(target_material),
    ));
}

fn update_points_display(points: Res<Points>, mut query: Query<&mut Text, With<PointsDisplay>>) {
    for mut text in &mut query {
        text.0 = format!("Points: {}", points.value);
    }
}

fn despawn_bullet_impacts(
    mut commands: Commands,
    mut bullet_impacts: Query<(Entity, &mut BulletImpact)>,
    time: Res<Time>,
) {
    for (entity, mut impact) in &mut bullet_impacts.iter_mut() {
        impact.stopwatch.tick(time.delta());
        if impact.stopwatch.elapsed_secs() > 0.1 {
            commands.entity(entity).despawn_recursive();
        }
    }
}
