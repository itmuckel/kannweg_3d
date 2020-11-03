use rand::{thread_rng, Rng};
use rg3d::engine::resource_manager::{ResourceManager, SharedSoundBuffer};
use rg3d::sound::context::Context;
use rg3d::sound::source::generic::GenericSourceBuilder;
use rg3d::sound::source::spatial::SpatialSourceBuilder;
use rg3d::sound::source::Status;

use crate::player::WalkState;
use crate::player::WalkState::Running;
use rg3d::physics::na::Vector3;
use std::sync::{Arc, Mutex};

pub async fn start_ambient_sound(ctx: Arc<Mutex<Context>>, resource_manager: ResourceManager) {
    let humming_buffer = resource_manager
        .request_sound_buffer("assets/humming.ogg", true)
        .await
        .unwrap();

    // Create flat source (without spatial effects) using that buffer.
    let source = GenericSourceBuilder::new(humming_buffer.into())
        .with_status(Status::Playing)
        .with_looping(true)
        .with_gain(0.1)
        .build_source()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _ = ctx.lock().unwrap().add_source(source);
}

pub async fn add_air_vent_sound(
    ctx: Arc<Mutex<Context>>,
    resource_manager: &ResourceManager,
    pos_x: f32,
    pos_y: f32,
) {
    let air_vent = resource_manager
        .request_sound_buffer("assets/air_vent.ogg", false)
        .await
        .unwrap();

    let _ = ctx.lock().unwrap().add_source(
        SpatialSourceBuilder::new(
            GenericSourceBuilder::new(air_vent.into())
                .with_looping(true)
                .with_status(Status::Playing)
                .with_gain(0.5)
                .build()
                .unwrap(),
        )
        .with_position(Vector3::new(pos_x, 0.5, pos_y))
        .with_radius(0.2)
        .with_max_distance(10.0)
        .with_rolloff_factor(2.5)
        .build_source(),
    );
}

pub async fn load_footstep_sounds(resource_manager: &mut ResourceManager) -> SharedSoundBuffer {
    resource_manager
        .request_sound_buffer("assets/footstep.ogg", false)
        .await
        .unwrap()
}

pub fn play_footstep(ctx: &mut Context, foot_step: SharedSoundBuffer, walk_state: &WalkState) {
    let gain = if *walk_state == Running { 0.15 } else { 0.07 };
    ctx.add_source(
        GenericSourceBuilder::new(foot_step.into())
            .with_play_once(true)
            .with_gain(gain)
            .with_pitch(thread_rng().gen_range(0.85, 1.0))
            .with_status(Status::Playing)
            .build_source()
            .unwrap(),
    );
}
