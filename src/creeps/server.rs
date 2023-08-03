use ambient_api::{
    animation::{PlayClipFromUrlNode, AnimationPlayer},
    asset, 
    components::core::{
        transform::{translation, local_to_world, rotation, local_to_parent},
        physics::{character_controller_height, character_controller_radius, dynamic, physics_controlled},
        app::name,
        ecs::{parent, children},
        prefab::prefab_from_url,
        animation::apply_animation_player
    },
    ecs::query,
    concepts::make_transformable,
    entity::{self, get_component, set_component, resources}, 
    physics::move_character, 
    prelude::{
        Quat, Entity, EntityId, Vec3, Vec2, Vec3Swizzles,
        vec3, delta_time, 
    }, main, 
};
use components::{team, creep_current_state, creep_next_state};

const INIT_POS: f32 = std::f32::consts::FRAC_PI_2;

const MARS_TEAM: u32 = 0;

const TIME_TO_NEXT_CREEP_SPAWNS: f32 = 5.;

const CREEP_IDLE_STATE: u16 = 0;
const CREEP_MOVE_STATE: u16 = 1;
const CREEP_PURSUIT_STATE: u16 = 2;
const CREEP_ATTACK_STATE: u16 = 3;

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

    entity::add_component(resources(), components::spawn_timer(), TIME_TO_NEXT_CREEP_SPAWNS);
    
    checks_if_creeps_should_change_their_states_system();

    spawns_creeps_regularly_system(idle_player);
    
    //Creeper movement and animation
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

                let target_pos = entity::get_component(model, components::target_pos()).unwrap();

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
                    //commented out the target_pos change as it's breaking the path finding.
                    /*entity::set_component(
                        model,
                        components::target_pos(),
                        current_pos.xy(),
                    );*/
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

fn checks_if_creeps_should_change_their_states_system() {

    // query of all creeps that have current and next state
    // If current_state != next state, changes current state to have the next state value
    // Removes components from current_state
    // Adds components of next_state

}

fn spawns_creeps_regularly_system(idle_player:AnimationPlayer) {
    //Spawns mars creeps regularly from the mars paths starting points
    query((translation(), components::is_path_point(), components::is_first_mars_point())).each_frame({
        move |list| {
            let time_to_next_creep_spawn = entity::get_component(resources(), components::spawn_timer()).unwrap();

            if time_to_next_creep_spawn <= 0. {
                for (mars_spawn_point_entity_id, (coordinates, _, _)) in list {
                    let next_path_point = entity::get_component(mars_spawn_point_entity_id, components::next_path_point()).unwrap();
                
                    create_ranged_creep(coordinates, idle_player, next_path_point, MARS_TEAM);
                }
                entity::set_component(resources(), components::spawn_timer(), TIME_TO_NEXT_CREEP_SPAWNS);
            }
            else {
                entity::set_component(resources(), components::spawn_timer(), time_to_next_creep_spawn - delta_time());
            }
        }
    });
}

fn create_ranged_creep(init_pos: Vec3, idle_player:AnimationPlayer, next_path_point:EntityId, which_team:u32) -> EntityId{
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
        .with(creep_current_state(), CREEP_IDLE_STATE)
        .with(creep_next_state(), CREEP_IDLE_STATE)
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
    entity::add_component(model, components::next_path_point(), next_path_point);
    
    let target = get_component(next_path_point, translation()).unwrap();

    entity::add_component(model, components::target_pos(), Vec2{x:target.x, y:target.y});

    entity::add_component(model, team(), which_team);

    model
}