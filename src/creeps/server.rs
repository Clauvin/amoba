use ambient_api::{
    animation::{PlayClipFromUrlNode, AnimationPlayer},
    asset, 
    components::core::{
        transform::{translation, local_to_world, rotation, local_to_parent},
        physics::{character_controller_height, character_controller_radius, dynamic, physics_controlled, cube_collider},
        app::name,
        ecs::{parent, children},
        prefab::prefab_from_url,
        animation::apply_animation_player
    },
    ecs::query,
    concepts::make_transformable,
    global::delta_time,
    entity::{add_component, self, get_component, set_component}, 
    physics::move_character, 
    prelude::{
        Quat, Entity, EntityId, Vec3, Vec2, Vec3Swizzles,
        vec3, 
    }, main, 
};

const INIT_POS: f32 = std::f32::consts::FRAC_PI_2;

#[main]
pub fn main() {
    let ranged_idle = PlayClipFromUrlNode::new(
        asset::url("assets/anim/Zombie Idle.fbx/animations/mixamo.com.anim").unwrap(),
    );

    let ranged_walk = PlayClipFromUrlNode::new(
        asset::url("assets/anim/Zombie Walk.fbx/animations/mixamo.com.anim").unwrap(),
    );

    let idle_player = AnimationPlayer::new(&ranged_idle);
    let walk_player = AnimationPlayer::new(&ranged_walk);

    let list = query((translation(), components::is_path_point(), components::is_first_mars_point())).build().evaluate();

    for (mars_spawn_point_entity_id, (coordinates, _, _)) in list {
        let next_path_point = entity::get_component(mars_spawn_point_entity_id, components::next_path_point()).unwrap();

        let model_test = create_ranged_creep(coordinates, idle_player, next_path_point);
        let target_pos = entity::get_component(model_test, components::target_pos()).unwrap();
        println!("{} {}", target_pos.x, target_pos.y);
    }

    query(components::is_creep()).each_frame({
        move |list| {
            for model in list {

                let model = model.0;
                
                let anim_model = entity::get_component(model, components::anim_model()).unwrap();

                let anim_state = entity::get_component(anim_model, components::anim_state()).unwrap();

                if anim_state == vec![0.0, 0.0, 1.0] {
                    continue;
                }

                let current_pos = entity::get_component(model, translation()).unwrap();
                //println!("{} {}", current_pos.x, current_pos.y);

                let target_pos = entity::get_component(model, components::target_pos()).unwrap();
                println!("{} {}", target_pos.x, target_pos.y);

                let diff = target_pos - current_pos.xy();

                if diff.length() < 1.0 {

                    move_character(model, vec3(0., 0., -0.1), 0.01, delta_time());

                    if anim_state != vec![0.0, 0.0, 1.0] {
                        entity::set_component(
                            anim_model,
                            apply_animation_player(),
                            idle_player.0,
                        );
                        entity::set_component(
                            anim_model,
                            components::anim_state(),
                            vec![1.0, 0.0, 0.0],
                        );

                        let current_path_point = get_component(model, components::next_path_point()).unwrap();

                        let next_path_point = match get_component(current_path_point, components::next_path_point()) {
                            Some(next) => next,
                            None => current_path_point
                        };

                        set_component(model, components::next_path_point(), next_path_point);

                        let next_target = get_component(next_path_point, translation()).unwrap();

                        entity::set_component(model, components::target_pos(), Vec2{x:next_target.x, y:next_target.y});

                        continue;
                    }
                }


                //-----------------------

                let target_direction = diff;
                let initial_direction: Vec2 = Vec2::new(1.0, 0.0);
                let dot = initial_direction.dot(target_direction);
                let det = initial_direction.x * target_direction.y
                    - initial_direction.y * target_direction.x;
                let angle = det.atan2(dot);
                let rot: Quat = Quat::from_rotation_z(angle - INIT_POS);
                entity::set_component(model, rotation(), rot);

                let speed = 0.05;
                let displace = diff.normalize_or_zero() * speed;

                if anim_state != vec![0.0, 1.0, 0.0] {
                    entity::set_component(anim_model, apply_animation_player(), walk_player.0);
                    entity::set_component(
                        anim_model,
                        components::anim_state(),
                        vec![0.0, 1.0, 0.0],
                    );
                }
                let collision = move_character(
                    model,
                    vec3(displace.x, displace.y, -0.1),
                    0.01,
                    delta_time(),
                );

                if collision.side {
                    entity::set_component(
                        model,
                        components::target_pos(),
                        current_pos.xy(),
                    );
                    entity::set_component(anim_model, apply_animation_player(), idle_player.0);
                    entity::set_component(
                        anim_model,
                        components::anim_state(),
                        vec![1.0, 0.0, 0.0],
                    );
                }
            }
        }
    });
}

fn create_ranged_creep(init_pos: Vec3, idle_player:AnimationPlayer, next_path_point:EntityId) -> EntityId{
    let model = Entity::new()
        .with_merge(make_transformable())
        .with(translation(), vec3(init_pos.x, init_pos.y, init_pos.z))
        .with(character_controller_height(), 2.)
        .with(character_controller_radius(), 0.3)
        .with(dynamic(), true)
        .with_default(physics_controlled())
        .with_default(local_to_world())
        .with(rotation(), Quat::from_rotation_z(-INIT_POS))
        .with(name(), "Ranged Creep".to_string())
        .spawn();

    let anim_model = Entity::new()
        .with_merge(make_transformable())
        .with_default(dynamic())
        .with(parent(), model)
        .with(
            prefab_from_url(),
            asset::url("assets/model/copzombie_l_actisdato.fbx").unwrap(),
        )
        .with_default(local_to_parent())
        .with_default(local_to_world())
        .with(translation(), vec3(0.0, 0.0, 0.))
        .spawn();

    entity::add_component(model, components::is_creep(), model);    

    entity::add_component(anim_model, apply_animation_player(), idle_player.0);
    entity::add_component(anim_model, components::anim_state(), vec![1.0, 0.0]);

    entity::add_component(model, children(), vec![anim_model]);
    entity::add_component(model, components::anim_model(), anim_model);
    //entity::add_component(model, components::target_pos(), Vec2{x:init_pos.x, y:init_pos.y});
    entity::add_component(model, components::next_path_point(), next_path_point);
    
    let target = get_component(next_path_point, translation()).unwrap();

    entity::add_component(model, components::target_pos(), Vec2{x:target.x, y:target.y});

    model
}