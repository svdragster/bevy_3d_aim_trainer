mod fps_gun_plugin;

use crate::fps_gun_plugin::FpsGunPlugin;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::render::view::RenderLayers;
use bevy::time::Stopwatch;
use bevy::window::CursorGrabMode;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::prelude::*;
use std::f32::consts::TAU;
use rand::distr::Uniform;

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
    spray: usize,
}

#[derive(Component)]
struct BulletImpact {
    stopwatch: Stopwatch,
}

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 6000.0,
        })
        .insert_resource(ClearColor(Color::srgb(0.83, 0.96, 0.96)))
        .insert_resource(Points::default()) // Add this line
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(FpsControllerPlugin)
        .add_plugins(FpsGunPlugin)
        .add_systems(
            Startup,
            (setup, fps_controller_setup.in_set(FpsControllerSetup)),
        )
        .add_systems(
            Update,
            (respawn, manage_cursor, click_targets, update_points_display, despawn_bullet_impacts),
        ) // Add update_points_display system
        .run();
}

fn fps_controller_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let height = 3.0;
    let logical_entity = commands
        .spawn((
            Collider::cylinder(height / 2.0, 0.5),
            // A capsule can be used but is NOT recommended
            // If you use it, you have to make sure each segment point is
            // equidistant from the translation of the player transform
            // Collider::capsule_y(height / 2.0, 0.5),
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            ActiveEvents::COLLISION_EVENTS,
            Velocity::zero(),
            RigidBody::Dynamic,
            Sleeping::disabled(),
            LockedAxes::ROTATION_LOCKED,
            AdditionalMassProperties::Mass(1.0),
            GravityScale(0.0),
            Ccd { enabled: true }, // Prevent clipping when going fast
            Transform::from_translation(SPAWN_POINT),
            LogicalPlayer,
            FpsControllerInput {
                pitch: -TAU / 12.0,
                yaw: TAU * 5.0 / 8.0,
                ..default()
            },
            FpsController {
                air_acceleration: 80.0,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: -0.5,
        })
        .insert(fps_gun_plugin::LastPosition {
            last_position: Vec3::ZERO,
        })
        .insert(ShootTracker {
            stopwatch: Stopwatch::new(),
            spray: 0,
        })
        .id();

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: TAU / 5.0,
            ..default()
        }),
        Exposure::SUNLIGHT,
        RenderPlayer { logical_entity },
    ));
}

fn setup(
    mut commands: Commands,
    mut window: Query<&mut Window>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut materials2d: ResMut<Assets<ColorMaterial>>,
) {
    let mut window = window.single_mut();
    window.title = String::from("Minimal FPS Controller Example");

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 14.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            ..default()
        },
    ));

    // Ground collider
    commands.spawn((
        Collider::cuboid(20.0, 0.1, 20.0),
        RigidBody::Fixed,
        Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
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
    ));

    // Wall
    commands.spawn((
        Collider::cuboid(5.0, 2.5, 0.5),
        RigidBody::Fixed,
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 5.0, 1.0))),
        MeshMaterial3d(ground_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
    ));

    spawn_random_target(&mut commands, &mut meshes, &mut materials);
    spawn_random_target(&mut commands, &mut meshes, &mut materials);
    spawn_random_target(&mut commands, &mut meshes, &mut materials);

    // Crosshair
    let color = Color::srgb(0.5, 0.7, 1.0);
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(2.0))),
        MeshMaterial2d(materials2d.add(color)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
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
    ));
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
    mut controller_query: Query<&mut FpsController>,
) {
    for mut window in &mut window_query {
        if btn.just_pressed(MouseButton::Left) {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
            for mut controller in &mut controller_query {
                controller.enable_input = true;
            }
        }
        if key.just_pressed(KeyCode::Escape) {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
            for mut controller in &mut controller_query {
                controller.enable_input = false;
            }
        }
    }
}

const SPRAY_DIRECTIONS: [Vec3; 12] = [
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(-0.01, 0.025, 0.0),
    Vec3::new(-0.02, 0.05, 0.0),
    Vec3::new(-0.03, 0.055, 0.0),
    Vec3::new(-0.032, 0.065, 0.0),
    Vec3::new(-0.034, 0.075, 0.0),
    Vec3::new(-0.038, 0.08, 0.0),
    Vec3::new(-0.042, 0.082, 0.0),
    Vec3::new(-0.046, 0.085, 0.0),
    Vec3::new(-0.042, 0.087, 0.0),
    Vec3::new(-0.039, 0.090, 0.0),
    Vec3::new(-0.038, 0.093, 0.0),
];

fn click_targets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    rapier_context: ReadDefaultRapierContext,
    player_query: Query<Entity, With<LogicalPlayer>>,
    camera: Query<&Transform, With<RenderPlayer>>,
    buttons: Res<ButtonInput<MouseButton>>,
    targets: Query<Entity, With<Target>>,
    mut points: ResMut<Points>,
    mut gun_animation_state: Query<&mut fps_gun_plugin::GunAnimationState>,
    mut shoot_stopwatch: Query<&mut ShootTracker>,
    time: Res<Time>,
) {
    let player_handle = player_query.single();
    let mut shoot_tracker = shoot_stopwatch
        .get_mut(player_handle)
        .expect("LogicalPlayer also needs a ShootStopwatch");
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
            shoot_tracker.stopwatch.reset();
            let rapier_context = rapier_context.single();
            let camera_transform = camera.single();
            let ray_pos = camera_transform.translation;
            let spray: Vec3;
            if shoot_tracker.spray >= SPRAY_DIRECTIONS.len() {
                let mut rng = rand::rng();
                let range = Uniform::new(-0.1f32, 0.1).unwrap();
                spray = Vec3::new(
                    rng.sample(range),
                    rng.sample(range),
                    0.0,
                );
            } else {
                spray = SPRAY_DIRECTIONS[shoot_tracker.spray];
            }
            shoot_tracker.spray += 1;
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
        }
    } else {
        shoot_tracker.spray = 0;
    }
}

fn spawn_random_target(
    mut commands: &mut Commands,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let rng = &mut rand::thread_rng();
    let x = rng.gen_range(-4.0..4.0);
    let y = rng.gen_range(2.0..5.0);
    let z = rng.gen_range(1.0..2.0);
    let size = rng.gen_range(0.3..0.8); // Random size between 0.3 and 0.8
    let color = Color::rgb(rng.gen(), rng.gen(), rng.gen()); // Random color

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