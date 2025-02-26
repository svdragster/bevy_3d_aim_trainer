use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::window::CursorGrabMode;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;
use std::f32::consts::TAU;
use bevy::render::view::RenderLayers;

use bevy_rapier3d::prelude::*;
use rand::Rng;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 0.0);

#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Target;


fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 6000.0,
        })
        .insert_resource(ClearColor(Color::srgb(0.83, 0.96, 0.96)))
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (respawn, manage_cursor, click_targets))
        .run();
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
        .id();

    commands.spawn((
        Camera3d::default(),
        Camera { order: 0, ..default() },
        Projection::Perspective(PerspectiveProjection {
            fov: TAU / 5.0,
            ..default()
        }),
        Exposure::SUNLIGHT,
        RenderPlayer { logical_entity },
    ));
    commands.spawn((Camera2d, Camera { order: 1, ..default() }));

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
        MeshMaterial3d(ground_material),
        Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
    ));

    spawn_random_target(&mut commands, &mut meshes, &mut materials);
    spawn_random_target(&mut commands, &mut meshes, &mut materials);
    spawn_random_target(&mut commands, &mut meshes, &mut materials);

    // Crosshair
    let color = Color::srgb(0.5, 0.7, 1.0);
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(2.0))),
        MeshMaterial2d(materials2d.add(color)),
        Transform::from_xyz(
            0.0,
            0.0,
            0.0,
        ),
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



fn click_targets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    rapier_context: ReadDefaultRapierContext,
    player_query: Query<Entity, With<LogicalPlayer>>,
    camera: Query<&Transform, With<RenderPlayer>>,
    buttons: Res<ButtonInput<MouseButton>>,
    targets: Query<Entity, With<Target>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let rapier_context = rapier_context.single();
        let player_handle = player_query.single();
        let camera_transform = camera.single();
        let ray_pos = camera_transform.translation;
        let ray_dir = camera_transform.forward().as_vec3();
        let max_toi: bevy_rapier3d::math::Real = 100.0;
        let solid = true;
        let filter = QueryFilter::new()
          .exclude_sensors()
          .exclude_rigid_body(player_handle);

        if let Some((entity, toi)) = rapier_context.cast_ray(ray_pos, ray_dir, max_toi, solid, filter) {
            // Handle the hit.
            if let Ok(target_entity) = targets.get(entity) {
                println!("Hit target entity {:?}", target_entity);
                // Remove the target
                commands.entity(entity).despawn_recursive();
                // Spawn a new target
                spawn_random_target(&mut commands, &mut meshes, &mut materials);
            }
        }
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

    let target_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.4, 0.4),
        ..Default::default()
    });

    commands.spawn((
        Collider::ball(0.5),
        RigidBody::Fixed,
        Transform::from_translation(Vec3::new(x, y, z)),
        Target,
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(target_material),
    ));
}