use bevy::app::Animation;
use bevy::math::Quat;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;

pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (apply_vertical_look_override
                .after(Animation)
                .before(TransformSystem::TransformPropagate),),
        );
    }
}

#[derive(Component)]
pub struct VerticalLook {
    pub node_name: &'static str,
    pub anchor: Option<Entity>,
}

#[derive(Component)]
pub struct VerticalLookAnchor {
    pub vertical_rotation: f32,
}

pub fn setup_vertical_look(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    mut vertical_look_query: Query<&mut VerticalLook>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
) {
    let parent = trigger.entity();
    let mut vertical_look = vertical_look_query
        .get_mut(parent)
        .expect("Entity must have 'VerticalLook' component if you want to use setup_vertical_look");
    let anchor_entity_name = vertical_look.node_name;
    for child in children_query.iter_descendants(parent) {
        if let Ok(name) = name_query.get(child) {
            if *name == anchor_entity_name.into() {
                if let Ok(parent_name) = name_query.get(parent) {
                    println!("Spine found, parent: {:?}", parent_name);
                } else {
                    println!("Spine found, parent: {:?}", parent);
                }

                let child_entity = commands
                    .entity(child)
                    .insert(VerticalLookAnchor {
                        vertical_rotation: 0.0,
                    })
                    .id();
                vertical_look.anchor = Some(child_entity);
            }
        }
    }
}

fn apply_vertical_look_override(
    mut vertical_look_anchor_query: Query<(&VerticalLookAnchor, &mut Transform)>,
) {
    for (anchor, mut transform) in &mut vertical_look_anchor_query.iter_mut() {
        transform.rotation = Quat::from_rotation_x(anchor.vertical_rotation);
    }
}
