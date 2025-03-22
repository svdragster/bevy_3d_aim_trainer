use std::f32::consts::*;

use crate::multiplayer::protocol::{ReplicatedMoveData, SoundEvent};
use crate::{Global, SPAWN_POINT};
use bevy::render::camera::Exposure;
use bevy::time::Stopwatch;
use bevy::{input::mouse::MouseMotion, math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::*;
use rand::distr::Uniform;
use rand::Rng;

/// Manages the FPS controllers. Executes in `PreUpdate`, after bevy's internal
/// input processing is finished.
///
/// If you need a system in `PreUpdate` to execute after FPS Controller's systems,
/// Do it like so:
///
/// ```
/// # use bevy::prelude::*;
///
/// struct MyPlugin;
/// impl Plugin for MyPlugin {
///     fn build(&self, app: &mut App) {
///         app.add_systems(
///             PreUpdate,
///             my_system.after(bevy_fps_controller::controller::fps_controller_render),
///         );
///     }
/// }
///
/// fn my_system() { }
/// ```
pub struct FpsControllerPlugin;

impl Plugin for FpsControllerPlugin {
    fn build(&self, app: &mut App) {
        use bevy::input::{gamepad, keyboard, mouse, touch};

        app.add_systems(
            PreUpdate,
            (
                //fps_controller_mouse_input, --> now handled by multiplayer client
                //fps_controller_look, --> now handled by multiplayer server
                //fps_controller_move, --> now handled by multiplayer server
                fps_controller_look,
                fps_controller_render,
            )
                .chain()
                .after(mouse::mouse_button_input_system)
                .after(keyboard::keyboard_input_system)
                .after(gamepad::gamepad_event_processing_system)
                .after(gamepad::gamepad_connection_system)
                .after(touch::touch_screen_input_system),
        );

        app.add_event::<EntityShotEvent>();
    }
}

pub const EYE_HEIGHT_OFFSET: f32 = 2.0;

#[derive(PartialEq)]
pub enum MoveMode {
    Noclip,
    Ground,
}

#[derive(Component)]
pub struct LogicalPlayer;

#[derive(Component)]
pub struct RenderPlayer {
    pub logical_entity: Entity,
}

#[derive(Component)]
pub struct CameraConfig {
    pub height_offset: f32,
}

#[derive(Component, Default)]
pub struct FpsControllerInput {
    pub fly: bool,
    pub sprint: bool,
    pub jump: bool,
    pub crouch: bool,
    pub shoot: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub movement: Vec3,
}

#[derive(Component, Default)]
pub struct FpsControllerLook {
    pub pitch: f32,
    pub yaw: f32,
}

#[derive(Component)]
pub struct FpsController {
    pub move_mode: MoveMode,
    pub radius: f32,
    pub eye_height_offset: f32,
    pub gravity: f32,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub forward_speed: f32,
    pub side_speed: f32,
    pub air_speed_cap: f32,
    pub air_acceleration: f32,
    pub max_air_speed: f32,
    pub acceleration: f32,
    pub friction: f32,
    /// If the dot product (alignment) of the normal of the surface and the upward vector,
    /// which is a value from [-1, 1], is greater than this value, ground movement is applied
    pub traction_normal_cutoff: f32,
    pub friction_speed_cutoff: f32,
    pub jump_speed: f32,
    pub fly_speed: f32,
    pub crouched_speed: f32,
    pub crouch_speed: f32,
    pub uncrouch_speed: f32,
    pub height: f32,
    pub upright_height: f32,
    pub crouch_height: f32,
    pub fast_fly_speed: f32,
    pub fly_friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub ground_tick: u8,
    pub stop_speed: f32,
    pub sensitivity: f32,
    pub enable_input: bool,
    pub step_offset: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_sprint: KeyCode,
    pub key_jump: KeyCode,
    pub key_fly: KeyCode,
    pub key_crouch: KeyCode,
    // Shooting
    pub shoot_stopwatch: Stopwatch,
    pub spray_count: usize,
    //
}

impl Default for FpsController {
    fn default() -> Self {
        Self {
            move_mode: MoveMode::Ground,
            radius: 0.5,
            eye_height_offset: EYE_HEIGHT_OFFSET,
            fly_speed: 10.0,
            fast_fly_speed: 30.0,
            gravity: 23.0,
            walk_speed: 9.0,
            run_speed: 14.0,
            forward_speed: 30.0,
            side_speed: 30.0,
            air_speed_cap: 2.0,
            air_acceleration: 20.0,
            max_air_speed: 15.0,
            crouched_speed: 5.0,
            crouch_speed: 6.0,
            uncrouch_speed: 8.0,
            height: 3.0,
            upright_height: 3.0,
            crouch_height: 1.5,
            acceleration: 10.0,
            friction: 10.0,
            traction_normal_cutoff: 0.7,
            friction_speed_cutoff: 0.1,
            fly_friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            ground_tick: 0,
            stop_speed: 1.0,
            jump_speed: 8.5,
            step_offset: 0.25,
            enable_input: true,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyQ,
            key_down: KeyCode::KeyE,
            key_sprint: KeyCode::ShiftLeft,
            key_jump: KeyCode::Space,
            key_fly: KeyCode::KeyF,
            key_crouch: KeyCode::ControlLeft,
            sensitivity: 0.001,
            shoot_stopwatch: Stopwatch::new(),
            spray_count: 0,
        }
    }
}

// ██╗      ██████╗  ██████╗ ██╗ ██████╗
// ██║     ██╔═══██╗██╔════╝ ██║██╔════╝
// ██║     ██║   ██║██║  ███╗██║██║
// ██║     ██║   ██║██║   ██║██║██║
// ███████╗╚██████╔╝╚██████╔╝██║╚██████╗
// ╚══════╝ ╚═════╝  ╚═════╝ ╚═╝ ╚═════╝

// Used as padding by camera pitching (up/down) to avoid spooky math problems
pub const ANGLE_EPSILON: f32 = 0.001953125;

// If the distance to the ground is less than this value, the player is considered grounded
pub const GROUNDED_DISTANCE: f32 = 0.125;

pub const SLIGHT_SCALE_DOWN: f32 = 0.9375;

pub fn spawn_logical_entity(commands: &mut Commands) -> Entity {
    let listener = SpatialListener::new(0.5);
    commands
        .spawn(build_logical_entity_bundle())
        .insert(CameraConfig {
            height_offset: -0.5,
        })
        .insert(listener)
        .id()
}

pub fn insert_logical_entity_bundle(commands: &mut Commands, entity: Entity) -> Entity {
    let listener = SpatialListener::new(0.5);
    commands
        .entity(entity)
        .insert(build_logical_entity_bundle())
        .insert(CameraConfig {
            height_offset: -0.5,
        })
        .insert(listener)
        .id()
}

fn build_logical_entity_bundle() -> (
    Collider,
    Friction,
    Restitution,
    ActiveEvents,
    Velocity,
    RigidBody,
    Sleeping,
    LockedAxes,
    AdditionalMassProperties,
    GravityScale,
    Ccd,
    Transform,
    LogicalPlayer,
    FpsControllerInput,
    FpsController,
) {
    let upright_height = 3.0;
    let eye_height = 2.0;
    (
        Collider::cylinder(upright_height / 2.0, 0.5),
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
            eye_height_offset: eye_height,
            upright_height,
            ..default()
        },
    )
}

pub fn create_render_entity_bundle(
    logical_entity: Entity,
) -> (
    Camera3d,
    Camera,
    Projection,
    FpsControllerLook,
    Exposure,
    RenderPlayer,
) {
    (
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: TAU / 5.0,
            ..default()
        }),
        FpsControllerLook {
            pitch: -TAU / 12.0,
            yaw: TAU * 5.0 / 8.0,
        },
        Exposure::SUNLIGHT,
        RenderPlayer { logical_entity },
    )
}

pub fn fps_controller_look(
    mut mouse_events: EventReader<MouseMotion>,
    mut look_query: Query<&mut FpsControllerLook>,
    mut query: Query<(&mut FpsControllerInput, &FpsController)>,
    global: Res<Global>,
) {
    if !global.mouse_captured {
        return;
    }
    if look_query.is_empty() {
        return;
    }
    let mut look = look_query.single_mut();
    for (mut input, controller) in query.iter_mut() {
        let mut mouse_delta = Vec2::ZERO;
        for mouse_event in mouse_events.read() {
            mouse_delta += mouse_event.delta;
        }
        mouse_delta *= controller.sensitivity;

        look.pitch = (look.pitch - mouse_delta.y)
            .clamp(-FRAC_PI_2 + ANGLE_EPSILON, FRAC_PI_2 - ANGLE_EPSILON);
        look.yaw -= mouse_delta.x;
        if look.yaw.abs() > PI {
            look.yaw = look.yaw.rem_euclid(TAU);
        }

        input.pitch = look.pitch;
        input.yaw = look.yaw;
    }
}

pub fn fps_controller_move(
    // FPS Controller
    time: &Res<Time>,
    physics_context: &ReadRapierContext,
    query: &mut Query<(
        Entity,
        &mut FpsController,
        &mut FpsControllerInput,
        &mut Collider,
        &mut Transform,
        &mut Velocity,
    )>,
) {
    let dt = time.delta_secs();

    for (entity, mut controller, input, mut collider, mut transform, mut velocity) in
        query.iter_mut()
    {
        // Look direction
        controller.pitch = input.pitch;
        controller.yaw = input.yaw;

        // Movement
        if input.fly {
            controller.move_mode = match controller.move_mode {
                MoveMode::Noclip => MoveMode::Ground,
                MoveMode::Ground => MoveMode::Noclip,
            }
        }

        match controller.move_mode {
            MoveMode::Noclip => {
                if input.movement == Vec3::ZERO {
                    let friction = controller.fly_friction.clamp(0.0, 1.0);
                    velocity.linvel *= 1.0 - friction;
                    if velocity.linvel.length_squared() < f32::EPSILON {
                        velocity.linvel = Vec3::ZERO;
                    }
                } else {
                    let fly_speed = if input.sprint {
                        controller.fast_fly_speed
                    } else {
                        controller.fly_speed
                    };
                    let mut move_to_world =
                        Mat3::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
                    move_to_world.z_axis *= -1.0; // Forward is -Z
                    move_to_world.y_axis = Vec3::Y; // Vertical movement aligned with world up
                    velocity.linvel = move_to_world * input.movement * fly_speed;
                }
            }
            MoveMode::Ground => {
                // Shape cast downwards to find ground
                // Better than a ray cast as it handles when you are near the edge of a surface
                let filter = QueryFilter::default().exclude_rigid_body(entity);
                let ground_cast = physics_context.single().cast_shape(
                    transform.translation,
                    transform.rotation,
                    -Vec3::Y,
                    // Consider when the controller is right up against a wall
                    // We do not want the shape cast to detect it,
                    // so provide a slightly smaller collider in the XZ plane
                    &scaled_collider_laterally(&collider, SLIGHT_SCALE_DOWN),
                    ShapeCastOptions::with_max_time_of_impact(GROUNDED_DISTANCE),
                    filter,
                );

                let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
                let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
                move_to_world.z_axis *= -1.0; // Forward is -Z
                let mut wish_direction = move_to_world * (input.movement * speeds);
                let mut wish_speed = wish_direction.length();
                if wish_speed > f32::EPSILON {
                    // Avoid division by zero
                    wish_direction /= wish_speed; // Effectively normalize, avoid length computation twice
                }
                let max_speed = if input.crouch {
                    controller.crouched_speed
                } else if input.sprint {
                    controller.run_speed
                } else {
                    controller.walk_speed
                };
                wish_speed = f32::min(wish_speed, max_speed);

                if let Some((hit, hit_details)) = unwrap_hit_details(ground_cast) {
                    let has_traction =
                        Vec3::dot(hit_details.normal1, Vec3::Y) > controller.traction_normal_cutoff;

                    // Only apply friction after at least one tick, allows b-hopping without losing speed
                    if controller.ground_tick >= 1 && has_traction {
                        let lateral_speed = velocity.linvel.xz().length();
                        if lateral_speed > controller.friction_speed_cutoff {
                            let control = f32::max(lateral_speed, controller.stop_speed);
                            let drop = control * controller.friction * dt;
                            let new_speed = f32::max((lateral_speed - drop) / lateral_speed, 0.0);
                            velocity.linvel.x *= new_speed;
                            velocity.linvel.z *= new_speed;
                        } else {
                            velocity.linvel = Vec3::ZERO;
                        }
                        if controller.ground_tick == 1 {
                            velocity.linvel.y = -hit.time_of_impact;
                        }
                    }

                    let mut add = acceleration(
                        wish_direction,
                        wish_speed,
                        controller.acceleration,
                        velocity.linvel,
                        dt,
                    );
                    if !has_traction {
                        add.y -= controller.gravity * dt;
                    }
                    velocity.linvel += add;

                    if has_traction {
                        let linear_velocity = velocity.linvel;
                        velocity.linvel -=
                            Vec3::dot(linear_velocity, hit_details.normal1) * hit_details.normal1;

                        if input.jump {
                            velocity.linvel.y = controller.jump_speed;
                        }
                    }

                    // Increment ground tick but cap at max value
                    controller.ground_tick = controller.ground_tick.saturating_add(1);
                } else {
                    controller.ground_tick = 0;
                    wish_speed = f32::min(wish_speed, controller.air_speed_cap);

                    let mut add = acceleration(
                        wish_direction,
                        wish_speed,
                        controller.air_acceleration,
                        velocity.linvel,
                        dt,
                    );
                    add.y = -controller.gravity * dt;
                    velocity.linvel += add;

                    let air_speed = velocity.linvel.xz().length();
                    if air_speed > controller.max_air_speed {
                        let ratio = controller.max_air_speed / air_speed;
                        velocity.linvel.x *= ratio;
                        velocity.linvel.z *= ratio;
                    }
                }

                /* Crouching */

                let crouch_height = controller.crouch_height;
                let upright_height = controller.upright_height;

                let crouch_speed = if input.crouch {
                    -controller.crouch_speed
                } else {
                    controller.uncrouch_speed
                };
                controller.height += dt * crouch_speed;
                controller.height = controller.height.clamp(crouch_height, upright_height);

                if let Some(mut capsule) = collider.as_capsule_mut() {
                    let radius = capsule.radius();
                    let half = Vec3::Y * (controller.height * 0.5 - radius);
                    capsule.set_segment(-half, half);
                } else if let Some(mut cylinder) = collider.as_cylinder_mut() {
                    cylinder.set_half_height(controller.height * 0.5);
                } else {
                    panic!("Controller must use a cylinder or capsule collider")
                }

                // Step offset really only works best for cylinders
                // For capsules the player has to practically teleported to fully step up
                if collider.as_cylinder().is_some()
                    && controller.step_offset > f32::EPSILON
                    && controller.ground_tick >= 1
                {
                    // Try putting the player forward, but instead lifted upward by the step offset
                    // If we can find a surface below us, we can adjust our position to be on top of it
                    let future_position = transform.translation + velocity.linvel * dt;
                    let future_position_lifted = future_position + Vec3::Y * controller.step_offset;
                    let rapier_context = physics_context.single();
                    let cast = rapier_context.cast_shape(
                        future_position_lifted,
                        transform.rotation,
                        -Vec3::Y,
                        &collider,
                        ShapeCastOptions::with_max_time_of_impact(
                            controller.step_offset * SLIGHT_SCALE_DOWN,
                        ),
                        filter,
                    );
                    if let Some((hit, details)) = unwrap_hit_details(cast) {
                        let has_traction_on_ledge =
                            Vec3::dot(details.normal1, Vec3::Y) > controller.traction_normal_cutoff;
                        if has_traction_on_ledge {
                            transform.translation.y += controller.step_offset - hit.time_of_impact;
                        }
                    }
                }

                // Prevent falling off ledges
                if controller.ground_tick >= 1 && input.crouch && !input.jump {
                    let rapier_context = physics_context.single();
                    for _ in 0..2 {
                        // Find the component of our velocity that is overhanging and subtract it off
                        let overhang = overhang_component(
                            entity,
                            &collider,
                            transform.as_ref(),
                            &rapier_context,
                            velocity.linvel,
                            dt,
                        );
                        if let Some(overhang) = overhang {
                            velocity.linvel -= overhang;
                        }
                    }
                    // If we are still overhanging consider unsolvable and freeze
                    if overhang_component(
                        entity,
                        &collider,
                        transform.as_ref(),
                        &rapier_context,
                        velocity.linvel,
                        dt,
                    )
                    .is_some()
                    {
                        velocity.linvel = Vec3::ZERO;
                    }
                }
            }
        }
    }
}

#[derive(Event)]
pub struct EntityShotEvent {
    pub shooter: Entity,
    pub entity: Entity,
    pub hit_point: Vec3,
}

pub const SPRAY_DIRECTIONS: [Vec3; 12] = [
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

pub const RANDOM_SPRAY_DIRECTIONS: [Vec3; 6] = [
    Vec3::new(-0.12, 0.12, 0.0),
    Vec3::new(0.05, -0.07, 0.0),
    Vec3::new(0.12, -0.13, 0.0),
    Vec3::new(-0.10, 0.11, 0.0),
    Vec3::new(0.09, 0.08, 0.0),
    Vec3::new(-0.04, -0.11, 0.0),
];

pub fn fps_controller_shoot(
    time: &Res<Time>,
    rapier_context: &ReadRapierContext,
    query: &mut Query<(
        Entity,
        &mut FpsController,
        &mut FpsControllerInput,
        &mut Collider,
        &mut Transform,
        &mut Velocity,
    )>,
    query_move_data: &Query<&ReplicatedMoveData>,
    entity_shot_event: &mut EventWriter<EntityShotEvent>,
    sound_event: &mut EventWriter<SoundEvent>,
) {
    let delta = time.delta();
    for (entity, mut controller, input, _, transform, _) in query.iter_mut() {
        controller.shoot_stopwatch.tick(delta);
        let replicated_move_data = query_move_data.get(entity);
        if replicated_move_data.is_err() {
            continue;
        }
        let replicated_move_data = replicated_move_data.unwrap();
        if input.shoot {
            if controller.shoot_stopwatch.elapsed_secs() > 0.1 {
                let rapier_context = rapier_context.single();

                let camera_offset = Vec3::Y * controller.eye_height_offset;
                let mut eye_transform = transform.clone();
                eye_transform.translation += camera_offset;
                eye_transform.rotation =
                    Quat::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
                let ray_pos = eye_transform.translation;
                let mut spray: Vec3;

                // Spray while holding left mouse button
                if controller.spray_count >= SPRAY_DIRECTIONS.len() {
                    spray = RANDOM_SPRAY_DIRECTIONS[controller.spray_count % RANDOM_SPRAY_DIRECTIONS.len()];
                } else {
                    spray = SPRAY_DIRECTIONS[controller.spray_count];
                }

                // Spray while walking
                if replicated_move_data.velocity.length_squared() > 0.1 * 0.1 {
                    spray += RANDOM_SPRAY_DIRECTIONS[controller.spray_count % RANDOM_SPRAY_DIRECTIONS.len()];
                }

                // Increment the spray count
                controller.spray_count += 1;

                let mut rng = rand::rng();
                let pitch_range = Uniform::new(-0.12f32, 0.12).unwrap();
                sound_event.send(SoundEvent {
                    emitter: Some(entity),
                    asset: "sounds/weapons-rifle-assault-rifle-fire-01.ogg".to_string(),
                    position: ray_pos.clone(),
                    volume: 0.3,
                    speed: 1.1 + rng.sample(pitch_range),
                    spatial: true,
                    spatial_scale: None,
                });

                let ray_dir = eye_transform.forward().as_vec3() + eye_transform.rotation * spray;
                let max_toi: bevy_rapier3d::math::Real = 100.0;
                let solid = true;
                let filter = QueryFilter::new()
                    .exclude_sensors()
                    .exclude_rigid_body(entity);


                if let Some((entity, toi)) =
                    rapier_context.cast_ray(ray_pos, ray_dir, max_toi, solid, filter)
                {
                    let hit_point = ray_pos + ray_dir * Vec3::splat(toi.into());
                    entity_shot_event.send(EntityShotEvent {
                        shooter: entity,
                        entity,
                        hit_point,
                    });

                    sound_event.send(SoundEvent {
                        emitter: Some(entity),
                        asset: "sounds/weapons-shield-metal-impact-ring-02.ogg".to_string(),
                        position: hit_point.clone(),
                        volume: 0.35,
                        speed: 1.0 + rng.sample(pitch_range),
                        spatial: true,
                        spatial_scale: Some(0.2),
                    });
                }

                controller.shoot_stopwatch.reset();
            }
        } else {
            controller.spray_count = 0;
        }
    }
}

fn unwrap_hit_details(
    ground_cast: Option<(Entity, ShapeCastHit)>,
) -> Option<(ShapeCastHit, ShapeCastHitDetails)> {
    if let Some((_, hit)) = ground_cast {
        if let Some(details) = hit.details {
            return Some((hit, details));
        }
    }
    None
}

/// Returns the offset that puts a point at the center of the player transform to the bottom of the collider.
/// Needed for when we want to originate something at the foot of the player.
fn collider_y_offset(collider: &Collider) -> Vec3 {
    Vec3::Y
        * if let Some(cylinder) = collider.as_cylinder() {
            cylinder.half_height()
        } else if let Some(capsule) = collider.as_capsule() {
            capsule.half_height() + capsule.radius()
        } else {
            panic!("Controller must use a cylinder or capsule collider")
        }
}

/// Return a collider that is scaled laterally (XZ plane) but not vertically (Y axis).
fn scaled_collider_laterally(collider: &Collider, scale: f32) -> Collider {
    if let Some(cylinder) = collider.as_cylinder() {
        let new_cylinder = Collider::cylinder(cylinder.half_height(), cylinder.radius() * scale);
        new_cylinder
    } else if let Some(capsule) = collider.as_capsule() {
        let new_capsule = Collider::capsule(
            capsule.segment().a(),
            capsule.segment().b(),
            capsule.radius() * scale,
        );
        new_capsule
    } else {
        panic!("Controller must use a cylinder or capsule collider")
    }
}

fn overhang_component(
    entity: Entity,
    collider: &Collider,
    transform: &Transform,
    physics_context: &RapierContext,
    velocity: Vec3,
    dt: f32,
) -> Option<Vec3> {
    // Cast a segment (zero radius capsule) from our next position back towards us (sweeping a rectangle)
    // If there is a ledge in front of us we will hit the edge of it
    // We can use the normal of the hit to subtract off the component that is overhanging
    let cast_capsule = Collider::capsule(Vec3::Y * 0.25, -Vec3::Y * 0.25, 0.01);
    let filter = QueryFilter::default().exclude_rigid_body(entity);
    let collider_offset = collider_y_offset(collider);
    let future_position = transform.translation - collider_offset + velocity * dt;
    let cast = physics_context.cast_shape(
        future_position,
        transform.rotation,
        -velocity,
        &cast_capsule,
        ShapeCastOptions::with_max_time_of_impact(0.5),
        filter,
    );
    if let Some((_, hit_details)) = unwrap_hit_details(cast) {
        let cast = physics_context.cast_ray(
            future_position + Vec3::Y * 0.125,
            -Vec3::Y,
            0.375.into(),
            false,
            filter,
        );
        // Make sure that this is actually a ledge, e.g. there is no ground in front of us
        if cast.is_none() {
            let normal = -hit_details.normal1;
            let alignment = Vec3::dot(velocity, normal);
            return Some(alignment * normal);
        }
    }
    None
}

fn acceleration(
    wish_direction: Vec3,
    wish_speed: f32,
    acceleration: f32,
    velocity: Vec3,
    dt: f32,
) -> Vec3 {
    let velocity_projection = Vec3::dot(velocity, wish_direction);
    let add_speed = wish_speed - velocity_projection;
    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let acceleration_speed = f32::min(acceleration * wish_speed * dt, add_speed);
    wish_direction * acceleration_speed
}

// ██████╗ ███████╗███╗   ██╗██████╗ ███████╗██████╗
// ██╔══██╗██╔════╝████╗  ██║██╔══██╗██╔════╝██╔══██╗
// ██████╔╝█████╗  ██╔██╗ ██║██║  ██║█████╗  ██████╔╝
// ██╔══██╗██╔══╝  ██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗
// ██║  ██║███████╗██║ ╚████║██████╔╝███████╗██║  ██║
// ╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝

pub fn fps_controller_render(
    mut render_query: Query<
        (&mut Transform, &FpsControllerLook, &RenderPlayer),
        With<RenderPlayer>,
    >,
    logical_query: Query<
        (&Transform, &Collider, &FpsController, &CameraConfig),
        (With<LogicalPlayer>, Without<RenderPlayer>),
    >,
) {
    for (mut render_transform, look, render_player) in render_query.iter_mut() {
        if let Ok((logical_transform, collider, controller, camera_config)) =
            logical_query.get(render_player.logical_entity)
        {
            let camera_offset = Vec3::Y * controller.eye_height_offset;
            render_transform.translation =
                logical_transform.translation + camera_offset;
            render_transform.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
        }
    }
}
