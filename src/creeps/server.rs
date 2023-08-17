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
    entity::{self, get_component, set_component, resources, add_component}, 
    physics::move_character, 
    prelude::{
        Quat, Entity, EntityId, Vec3, Vec2, Vec3Swizzles,
        vec3, delta_time, 
    }, main, 
};
use components::{team, is_creep, creep_current_state, creep_next_state, pursuit_target};

const INIT_POS: f32 = std::f32::consts::FRAC_PI_2;

const MARS_TEAM: u32 = 0;
const JUPYTER_TEAM: u32 = 1;

const TIME_TO_NEXT_CREEP_SPAWNS: f32 = 5.;

const CREEP_IDLE_STATE: u16 = 0;
const CREEP_MOVE_STATE: u16 = 1;
const CREEP_PURSUIT_STATE: u16 = 2;
const CREEP_ATTACK_STATE: u16 = 3;

const CREEP_MAXIMUM_PURSUIT_CHECK_DISTANCE: f32 = 5.;

macro_rules! idle_animation_state { () => { vec![1.0, 0.0, 0.0] }; }
macro_rules! walk_animation_state { () => { vec![0.0, 1.0, 0.0] }; }
macro_rules! attack_animation_state { () => { vec![0.0, 0.0, 1.0] }; }

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

    creep_idle_state_system();
    
    creep_pursuit_state_system();
    
    creep_move_state_system(idle_player, walk_player);
    
    creep_attack_state_system();

    spawns_creeps_regularly_system(idle_player);
    
    //Creeper movement and animation
    
}

fn checks_if_creeps_should_change_their_states_system() {

    query((is_creep(), creep_current_state(), creep_next_state())).each_frame({
        move |list| {
            for (creep, (_, current_state, next_state)) in list {
                if current_state != next_state {
                    
                    //Refactor this code to use a match instead
                    //Leaving current_state
                    if current_state == CREEP_IDLE_STATE {

                    }
                    else if current_state == CREEP_MOVE_STATE {
                        entity::remove_component(creep, components::target_pos());
                    }
                    else if current_state == CREEP_PURSUIT_STATE {
                        entity::remove_component(creep, pursuit_target());
                    }
                    else if current_state == CREEP_ATTACK_STATE {

                    }

                    //Entering next_state
                    if next_state == CREEP_IDLE_STATE {

                    }
                    else if next_state == CREEP_MOVE_STATE {
                        let next_path_point = entity::get_component(creep, components::next_path_point()).unwrap();
    
                        let target = get_component(next_path_point, translation()).unwrap();

                        entity::add_component(creep, components::target_pos(), Vec2{x:target.x, y:target.y});
                    }
                    else if next_state == CREEP_PURSUIT_STATE {

                    }
                    else if next_state == CREEP_ATTACK_STATE {

                    }

                    entity::set_component(creep, creep_current_state(), next_state);
                    //println!("Changed state from {:?} to {:?}", current_state, next_state);
                }
            }


        }
    });



}

fn creep_idle_state_system(){
    let all_heroes_query = query((components::hero_model(), components::role(), components::hero_model())).build();


    query(components::is_creep()).each_frame({
        move |list| {

            for (creep_model, _) in list.iter() {
                //TECHNOLOGICAL DEBT: There's for sure a better alternative to solve this than evaluation of this for every creep, and the rest of the other calculations too
                let all_heroes = all_heroes_query.evaluate();

                let creep_team = entity::get_component(*creep_model, team()).unwrap();

                let creep_position = entity::get_component(*creep_model, translation()).unwrap();

                let mut closest_hero: Option<EntityId> = None;
                let mut distance_of_closest_hero: Option<f32> = None;

                for (hero_id, (_, hero_role, hero_model)) in all_heroes {
                    if creep_team%2 != hero_role%2 {
                        let current_hero_position = entity::get_component(hero_model, translation()).unwrap();

                        let distance_of_current_hero = (creep_position.xy() - current_hero_position.xy()).length();

                        match closest_hero {
                            None => {
                                if distance_of_current_hero <= CREEP_MAXIMUM_PURSUIT_CHECK_DISTANCE {
                                    closest_hero = Some(hero_model);
                                    distance_of_closest_hero = Some(distance_of_current_hero);
                                }
                            }
                            Some(_) => {
                                if distance_of_current_hero < distance_of_closest_hero.unwrap() {
                                    closest_hero = Some(hero_id);
                                    distance_of_closest_hero = Some(distance_of_current_hero);
                                }
                            }
                        }
                    }
                }
                
                if closest_hero != None {
                    //TECHNOLOGICAL DEBT: Is there a better way to do this?
                    entity::add_component(*creep_model, pursuit_target(), closest_hero.unwrap());
                    entity::set_component(*creep_model, creep_next_state(), CREEP_PURSUIT_STATE);
                    continue;
                }

                //else Do we have an enemy creep close enough of the creep?
                let mut closest_enemy_creep: Option<EntityId> = None;
                let mut distance_of_closest_enemy_creep: Option<f32> = None;

                for (creep_model_2, _) in list.iter(){
                    let team_of_first_creep = entity::get_component(*creep_model, team());
                    let team_of_second_creep = entity::get_component(*creep_model_2, team());

                    //TECHNOLOGICAL DEBT: Refactor some var names here
                    //and optimize the code a bit
                    if team_of_first_creep != team_of_second_creep {
                        match closest_enemy_creep {
                            None => {
                                let other_creep_position = entity::get_component(*creep_model_2, translation()).unwrap();

                                let current_creep_position = entity::get_component(*creep_model, translation()).unwrap();

                                let creep_dist = (current_creep_position.xy() - other_creep_position.xy()).length();

                                if creep_dist <= CREEP_MAXIMUM_PURSUIT_CHECK_DISTANCE {
                                    closest_enemy_creep = Some(*creep_model_2);
                                }

                            }
                            Some(_) => {
                                let closest_creep_position = entity::get_component(closest_enemy_creep.unwrap(), translation()).unwrap();

                                let other_creep_position = entity::get_component(*creep_model_2, translation()).unwrap();

                                let current_creep_position = entity::get_component(*creep_model, translation()).unwrap();

                                let closest_creep_distance = (current_creep_position.xy() - closest_creep_position.xy()).length();

                                let new_creep_distance = (current_creep_position.xy() - other_creep_position.xy()).length();

                                if new_creep_distance < closest_creep_distance {
                                    closest_enemy_creep = Some(*creep_model_2);
                                }
                                
                            }
                        }
                    }
                                        
                    //If creep_model_2 is in a different team than creep_model
                    //Check distance, if distance <= maximum, pursue.

                }

                if closest_enemy_creep != None {
                    //TECHNOLOGICAL DEBT: Is there a better way to do this?
                    entity::add_component(*creep_model, pursuit_target(), closest_enemy_creep.unwrap());
                    entity::set_component(*creep_model, creep_next_state(), CREEP_PURSUIT_STATE);
                    continue;
                }

                //If yes, pursue creep.
                //else Do we have an enemy base close enough of the creep?
                //If yes, pursue base.
                
                let current_state = entity::get_component(*creep_model, creep_current_state()).unwrap();
                let next_state = entity::get_component(*creep_model, creep_next_state()).unwrap();


                if current_state == next_state && current_state == CREEP_IDLE_STATE {
                    entity::set_component(*creep_model, creep_next_state(), CREEP_MOVE_STATE);
                }

            }
        }
    });
}

fn creep_pursuit_state_system(){
    query((components::is_creep(), pursuit_target())).each_frame({
        move |list| {
            for (creep_model, (_, target_entity)) in list {
                println!("Should be pursuing {:?}", target_entity);
            }
        }
    });




}

fn creep_move_state_system(idle_player: AnimationPlayer, walk_player: AnimationPlayer){
    query((components::is_creep(), components::target_pos())).each_frame({
        move |list| {
            for (model, (_, _)) in list {
                
                let anim_model = entity::get_component(model, components::anim_model()).unwrap();

                let anim_state = entity::get_component(anim_model, components::anim_state()).unwrap();

                if anim_state == attack_animation_state!() {
                    continue;
                }

                let current_pos = entity::get_component(model, translation()).unwrap();

                let target_pos = entity::get_component(model, components::target_pos()).unwrap();

                let diff = target_pos - current_pos.xy();

                if diff.length() < 1.0 {

                    move_character(model, vec3(0., 0., -0.1), 0.01, delta_time());

                    if anim_state != attack_animation_state!() {
                        entity::set_component(
                            anim_model,
                            apply_animation_player(),
                            idle_player.0,
                        );
                        entity::set_component(
                            anim_model,
                            components::anim_state(),
                            idle_animation_state!(),
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

                if anim_state != walk_animation_state!() {
                    entity::set_component(anim_model, apply_animation_player(), walk_player.0);
                    entity::set_component(
                        anim_model,
                        components::anim_state(),
                        walk_animation_state!(),
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
                        idle_animation_state!(),
                    );
                }
            }
        }
    });
}

fn creep_attack_state_system(){

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
        .with(creep_next_state(), CREEP_MOVE_STATE)
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

    entity::add_component(model, components::is_creep(), ());    

    entity::add_component(anim_model, apply_animation_player(), idle_player.0);
    entity::add_component(anim_model, components::anim_state(), idle_animation_state!());

    entity::add_component(model, children(), vec![anim_model]);
    entity::add_component(model, components::anim_model(), anim_model);
    entity::add_component(model, components::next_path_point(), next_path_point);

    entity::add_component(model, team(), which_team);

    model
}