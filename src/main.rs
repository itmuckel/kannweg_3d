#![feature(cmp_min_max_by)]

extern crate rg3d;

use std::cmp::{max_by, min_by};
use std::time::Instant;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rg3d::engine::resource_manager::TextureImportOptions;
use rg3d::gui::message::MessageDirection;
use rg3d::renderer::QualitySettings;
use rg3d::resource::texture::{TextureMagnificationFilter, TextureMinificationFilter};
use rg3d::scene::light::{BaseLightBuilder, PointLightBuilder, SpotLightBuilder};
use rg3d::scene::Line;
use rg3d::{
    core::{color::Color, pool::Handle},
    engine::resource_manager::ResourceManager,
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{message::TextMessage, node::StubNode, text::TextBuilder, widget::WidgetBuilder},
    scene::{
        base::BaseBuilder, camera::CameraBuilder, node::Node, transform::TransformBuilder, Scene,
    },
    utils::translate_event,
};

use crate::level_generator::{FieldType, Level, RoomOptions};
use crate::player::Player;
use crate::sound::{add_air_vent_sound, load_footstep_sounds, play_footstep, start_ambient_sound};
use rg3d::futures::executor::block_on;
use rg3d::physics::na::{UnitQuaternion, Vector3};
use rg3d::sound::context::Context;
use std::any::Any;
use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

mod level_generator;
mod player;
mod sound;

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

struct GameScene {
    player: Player,
    scene: Scene,
    camera_handle: Handle<Node>,
    flash_light_handle: Handle<Node>,
}

fn create_point_light(radius: f32) -> Node {
    let point_light = PointLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()).with_scatter_enabled(false));

    point_light.with_radius(radius).build_node()
}

fn create_flash_light() -> Node {
    let spot_light = SpotLightBuilder::new(
        BaseLightBuilder::new(BaseBuilder::new())
            .with_color(Color::from_rgba(232, 226, 185, 255))
            .with_scatter_enabled(true),
    );

    spot_light
        .with_distance(6.0)
        .with_hotspot_cone_angle(60.0f32.to_radians())
        .with_falloff_angle_delta(12.0f32.to_radians())
        // .with_shadow_bias(0.05)
        .build_node()
}

async fn add_corners(level: &mut Level, scene: &mut Scene, resource_manager: &ResourceManager) {
    let wall_inner_corner_resource = resource_manager
        .request_model("assets/wall_inner_corner.fbx")
        .await
        .unwrap();

    let wall_outer_corner_resource = resource_manager
        .request_model("assets/wall_outer_corner.fbx")
        .await
        .unwrap();

    for x in 0..level.map.len() {
        for y in 0..level.map[0].len() {
            if level.map[x][y].typ == FieldType::Empty {
                // add outer corners
                let neighbours = level
                    .get_neighbours((x, y), 1)
                    .into_iter()
                    .filter(|&(x, y)| level.map[x][y].typ != FieldType::Empty)
                    .collect::<Vec<_>>();

                let mut add_corner = |rotation: f32| {
                    let corner_handle = wall_outer_corner_resource.instantiate_geometry(scene);
                    scene.graph[corner_handle]
                        .local_transform_mut()
                        .set_rotation(UnitQuaternion::from_axis_angle(
                            &Vector3::y_axis(),
                            rotation.to_radians(),
                        ))
                        .offset(Vector3::new(x as f32, 0.0, y as f32));
                };

                if neighbours.iter().any(|&(n_x, _)| n_x < x)
                    && neighbours.iter().any(|&(_, n_y)| n_y < y)
                {
                    add_corner(0.0);
                    level.map[x - 1][y].walls.right_up = true;
                    level.map[x][y - 1].walls.down_left = true;
                }

                if neighbours.iter().any(|&(n_x, _)| n_x < x)
                    && neighbours.iter().any(|&(_, n_y)| n_y > y)
                {
                    add_corner(90.0);
                    level.map[x - 1][y].walls.right_down = true;
                    level.map[x][y + 1].walls.up_left = true;
                }

                if neighbours.iter().any(|&(n_x, _)| n_x > x)
                    && neighbours.iter().any(|&(_, n_y)| n_y < y)
                {
                    add_corner(-90.0);
                    level.map[x + 1][y].walls.left_up = true;
                    level.map[x][y - 1].walls.down_right = true;
                }

                if neighbours.iter().any(|&(n_x, _)| n_x > x)
                    && neighbours.iter().any(|&(_, n_y)| n_y > y)
                {
                    add_corner(180.0);
                    level.map[x + 1][y].walls.left_down = true;
                    level.map[x][y + 1].walls.up_right = true;
                }

                continue;
            }

            // add inner corners
            let neighbours = level
                .get_neighbours((x, y), 1)
                .into_iter()
                .filter(|&(x, y)| level.map[x][y].typ == FieldType::Empty)
                .collect::<Vec<_>>();

            let mut add_corner = |rotation: f32| {
                let corner_handle = wall_inner_corner_resource.instantiate_geometry(scene);
                scene.graph[corner_handle]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        rotation.to_radians(),
                    ))
                    .offset(Vector3::new(x as f32, 0.0, y as f32));
            };

            if neighbours.iter().any(|&(n_x, _)| n_x < x)
                && neighbours.iter().any(|&(_, n_y)| n_y < y)
            {
                level.map[x][y].walls.up_left = true;
                level.map[x][y].walls.left_up = true;
                add_corner(0.0);
            }

            if neighbours.iter().any(|&(n_x, _)| n_x > x)
                && neighbours.iter().any(|&(_, n_y)| n_y < y)
            {
                level.map[x][y].walls.up_right = true;
                level.map[x][y].walls.right_up = true;
                add_corner(-90.0);
            }

            if neighbours.iter().any(|&(n_x, _)| n_x < x)
                && neighbours.iter().any(|&(_, n_y)| n_y > y)
            {
                level.map[x][y].walls.down_left = true;
                level.map[x][y].walls.left_down = true;
                add_corner(90.0);
            }

            if neighbours.iter().any(|&(n_x, _)| n_x > x)
                && neighbours.iter().any(|&(_, n_y)| n_y > y)
            {
                level.map[x][y].walls.down_right = true;
                level.map[x][y].walls.right_down = true;
                add_corner(180.0);
            }
        }
    }
}

async fn add_rest(level: &mut Level, scene: &mut Scene, resource_manager: &ResourceManager) {
    let wall_resource = resource_manager
        .request_model("assets/wall.fbx")
        .await
        .unwrap();

    let floor_resource = resource_manager
        .request_model("assets/floor.fbx")
        .await
        .unwrap();

    let corridor_resource = resource_manager
        .request_model("assets/corridor.fbx")
        .await
        .unwrap();

    let mut tile_count = 0;
    for x in 0..level.map.len() {
        for y in 0..level.map[0].len() {
            if level.map[x][y].typ == FieldType::Empty {
                continue;
            }

            tile_count += 1;

            // create floor
            let floor_handle = match level.map[x][y].typ {
                FieldType::Floor => floor_resource.instantiate_geometry(scene),
                FieldType::Corridor => corridor_resource.instantiate_geometry(scene),
                FieldType::Door => corridor_resource.instantiate_geometry(scene),
                _ => floor_resource.instantiate_geometry(scene), // should be something else...
            };

            scene.graph[floor_handle]
                .local_transform_mut()
                .offset(Vector3::new(x as f32, 0.0, y as f32));

            // add light to floors
            if level.map[x][y].typ == FieldType::Corridor && tile_count % 3 == 0 {
                let handle = scene.graph.add_node(create_point_light(1.0));
                scene.graph[handle]
                    .local_transform_mut()
                    .offset(Vector3::new(x as f32, 0.3, y as f32));
            }

            // fill in missing walls
            let add_wall = |scene: &mut Scene, rotation: f32, offset_x: f32, offset_y: f32| {
                let wall_handle = wall_resource.instantiate_geometry(scene);
                scene.graph[wall_handle]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        rotation.to_radians(),
                    ))
                    .offset(Vector3::new(x as f32 + offset_x, 0.0, y as f32 + offset_y));
            };

            let neighbours = level
                .get_neighbours((x, y), 1)
                .into_iter()
                .filter(|(x, y)| level.map[*x][*y].typ == FieldType::Empty)
                .collect::<Vec<_>>();

            for n in neighbours {
                let mut walls = &mut level.map[x][y].walls;
                if n.0 < x {
                    if !walls.left_up {
                        add_wall(scene, 90.0, 0.0, -0.5);
                        walls.left_up = true;
                    }
                    if !walls.left_down {
                        add_wall(scene, 90.0, 0.0, 0.0);
                        walls.left_down = true;
                    }
                }
                if n.0 > x {
                    if !walls.right_up {
                        add_wall(scene, -90.0, 0.0, 0.0);
                        walls.right_up = true;
                    }
                    if !walls.right_down {
                        add_wall(scene, -90.0, 0.0, 0.5);
                        walls.right_down = true;
                    }
                }
                if n.1 < y {
                    if !walls.up_left {
                        add_wall(scene, 0.0, 0.0, 0.0);
                        walls.up_left = true;
                    }
                    if !walls.up_right {
                        add_wall(scene, 0.0, 0.5, 0.0);
                        walls.up_right = true;
                    }
                }
                if n.1 > y {
                    if !walls.down_left {
                        add_wall(scene, 180.0, -0.5, 0.0);
                        walls.down_left = true;
                    }
                    if !walls.down_right {
                        add_wall(scene, 180.0, 0.0, 0.0);
                        walls.down_right = true;
                    }
                }
            }
        }
    }
}

async fn create_scene(resource_manager: ResourceManager, ctx: Arc<Mutex<Context>>) -> GameScene {
    let mut scene = Scene::new();

    resource_manager.state().set_textures_import_options(
        TextureImportOptions::default()
            .with_minification_filter(TextureMinificationFilter::Nearest)
            .with_magnification_filter(TextureMagnificationFilter::Nearest),
    );

    // create level
    let mut level = Level::create_dungeon(
        23,
        39,
        RoomOptions {
            max_rooms: 10,
            max_attempts: 125,
            min_size: 4,
            max_size: 10,
        },
        FieldType::Floor,
    );

    add_corners(&mut level, &mut scene, &resource_manager).await;
    add_rest(&mut level, &mut scene, &resource_manager).await;

    let pl = create_point_light(4.0);

    let air_vent = resource_manager
        .request_model("assets/air_vent.fbx")
        .await
        .unwrap();

    let oxygen_tank = resource_manager
        .request_model("assets/oxygen.fbx")
        .await
        .unwrap();

    let mut rng = thread_rng();

    for room in &mut level.rooms {
        // add lights
        room.sort();
        let pos = room[room.len() / 2];

        let point_light = scene.graph.add_node(pl.raw_copy());

        scene.graph[point_light]
            .local_transform_mut()
            .set_position(Vector3::new(pos.0 as f32, 2.0, pos.1 as f32));

        // add vents
        let (min_x, min_y) = room[0];
        let (max_x, max_y) = room[room.len() - 1];
        let edges = room
            .clone()
            .into_iter()
            .filter(|&(x, y)| x == min_x || x == max_x || y == min_y || y == max_y)
            .collect::<Vec<_>>();

        'attempt: loop {
            let pos = edges.choose(&mut rng).unwrap();

            let walls = &level.map[pos.0][pos.1].walls;

            let sound_offset: (f32, f32);

            let rot: f32;
            if walls.up_left {
                rot = 0.0;
                sound_offset = (0.0, -0.5);
            } else if walls.right_up {
                rot = 270.0;
                sound_offset = (0.5, 0.0);
            } else if walls.down_left {
                rot = 180.0;
                sound_offset = (0.0, 0.5);
            } else if walls.left_up {
                rot = 90.0;
                sound_offset = (-0.5, 0.0);
            } else {
                // no wall!
                continue 'attempt;
            }

            let handle = air_vent.instantiate_geometry(&mut scene);
            scene.graph[handle]
                .local_transform_mut()
                .offset(Vector3::new(pos.0 as f32, 0.0, pos.1 as f32))
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    rot.to_radians(),
                ));

            add_air_vent_sound(
                ctx.clone(),
                &resource_manager,
                pos.0 as f32 + sound_offset.0,
                pos.1 as f32 + sound_offset.1,
            )
            .await;

            break 'attempt;
        }

        let oxygen_tank_pos = room.choose(&mut rng).unwrap();

        let handle = oxygen_tank.instantiate_geometry(&mut scene);
        scene.graph[handle]
            .local_transform_mut()
            .offset(Vector3::new(
                oxygen_tank_pos.0 as f32,
                0.0,
                oxygen_tank_pos.1 as f32,
            ))
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                [0.0f32, 90.0, 180.0, 270.0]
                    .choose(&mut rng)
                    .unwrap()
                    .to_radians(),
            ));
    }

    let camera = CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(7.0, 0.5, 7.0))
                .build(),
        ),
    )
    .build();

    let camera_handle = scene.graph.add_node(Node::Camera(camera));

    let camera_pos = scene.graph[camera_handle].global_position();

    let flash_light_handle = scene.graph.add_node(create_flash_light());

    scene.graph[flash_light_handle]
        .local_transform_mut()
        .set_rotation(UnitQuaternion::from_axis_angle(
            &Vector3::x_axis(),
            -90.0f32.to_radians(),
        ))
        .set_position(camera_pos + Vector3::new(-0.3, -0.2, 0.0));

    scene.graph.link_nodes(flash_light_handle, camera_handle);

    start_ambient_sound(ctx.clone(), resource_manager.clone()).await;

    GameScene {
        player: Player::default(),
        scene,
        camera_handle,
        flash_light_handle,
    }
}

struct InputController {
    move_left: bool,
    move_right: bool,
    move_forward: bool,
    move_backward: bool,
    run: bool,
    jump: bool,
    crouch: bool,
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("kannweg_3d")
        .with_resizable(true)
        .with_maximized(true);

    let mut engine = GameEngine::new(window_builder, &event_loop).unwrap();

    let quality_settings = QualitySettings {
        point_shadow_map_size: 1024,
        point_shadows_distance: 10.0,
        point_shadows_enabled: true,
        point_soft_shadows: true,

        spot_shadow_map_size: 512,
        spot_shadows_distance: 10.0,
        spot_shadows_enabled: true,
        spot_soft_shadows: true,

        use_ssao: false,
        ssao_radius: 0.5,

        light_scatter_enabled: true,
    };
    if let Err(err) = engine.renderer.set_quality_settings(&quality_settings) {
        panic!("{:?}", err);
    }

    engine.resource_manager.state().set_textures_path("assets");
    engine.get_window().set_cursor_visible(false);

    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    // engine
    //     .sound_context
    //     .lock()
    //     .unwrap()
    //     .set_renderer(Renderer::HrtfRenderer(HrtfRenderer::new(
    //         HrirSphere::from_file("assets/IRC_1005_C.bin", context::SAMPLE_RATE).unwrap(),
    //     )));

    let GameScene {
        mut player,
        scene,
        camera_handle,
        flash_light_handle,
    } = block_on(create_scene(
        engine.resource_manager.clone(),
        engine.sound_context.clone(),
    ));

    let scene_handle = engine.scenes.add(scene);

    let foot_step = block_on(load_footstep_sounds(&mut engine.resource_manager));

    engine.renderer.set_ambient_color(Color::opaque(20, 20, 20));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    let mut camera_x = 0.0f32.to_radians();
    let mut camera_y = 0.0f32.to_radians();

    let mut input_controller = InputController {
        move_left: false,
        move_right: false,
        move_forward: false,
        move_backward: false,
        run: false,
        jump: false,
        crouch: false,
    };

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                while dt >= fixed_timestep {
                    dt -= fixed_timestep;
                    elapsed_time += fixed_timestep;

                    // ************************
                    // Put your game logic here.
                    // ************************

                    // Use stored scene handle to borrow a mutable reference of scene in
                    // engine.
                    let scene = &mut engine.scenes[scene_handle];

                    scene.graph[camera_handle]
                        .local_transform_mut()
                        .set_rotation(
                            UnitQuaternion::from_axis_angle(
                                &Vector3::y_axis(),
                                -camera_x.to_radians(),
                            ) * &UnitQuaternion::from_axis_angle(
                                &Vector3::x_axis(),
                                camera_y.to_radians(),
                            ),
                        );

                    let side = scene.graph[camera_handle].side_vector();
                    let mut back_front = scene.graph[camera_handle].look_vector();
                    back_front.y = 0.0;
                    back_front = back_front.try_normalize(0.0).unwrap_or(Vector3::default());

                    let mut offset = Vector3::default();

                    if input_controller.move_right {
                        offset -= side;
                    }
                    if input_controller.move_left {
                        offset += side;
                    }
                    if input_controller.move_forward {
                        offset += back_front;
                    }
                    if input_controller.move_backward {
                        offset -= back_front;
                    }

                    if input_controller.move_forward
                        || input_controller.move_backward
                        || input_controller.move_left
                        || input_controller.move_right
                    {
                        player.walk();
                    } else {
                        player.stand()
                    }

                    if input_controller.run {
                        player.run();
                    }

                    if player.should_play_step_sound() {
                        let mut ctx = engine.sound_context.lock().unwrap();
                        play_footstep(&mut ctx, foot_step.clone(), &player.walk_state)
                    }

                    let speed = if input_controller.run {
                        Player::SPEED + Player::EXTRA_RUN_SPEED
                    } else {
                        Player::SPEED
                    };

                    offset.x *= speed;
                    offset.z *= speed;

                    if input_controller.jump {
                        offset.y += speed;
                    }
                    if input_controller.crouch {
                        offset.y -= speed;
                    }

                    let camera = &mut scene.graph[camera_handle];

                    camera.local_transform_mut().offset(offset);

                    // update listener
                    {
                        let mut ctx = engine.sound_context.lock().unwrap();
                        let listener = ctx.listener_mut();
                        listener.set_position(camera.global_position());
                        listener.set_orientation_rh(camera.look_vector(), camera.up_vector());
                    }

                    let fps = engine.renderer.get_statistics().frames_per_second;
                    let text = format!(
                        "FPS: {} \nDraw Calls: {}",
                        fps,
                        engine.renderer.get_statistics().geometry.draw_calls
                    );

                    engine.user_interface.send_message(TextMessage::text(
                        debug_text,
                        MessageDirection::ToWidget,
                        text,
                    ));

                    // for debugging
                    scene.drawing_context.clear_lines();
                    scene.drawing_context.add_line(Line {
                        begin: Vector3::default(),
                        end: Vector3::x_axis().scale(20.0),
                        color: Color::RED,
                    });
                    scene.drawing_context.add_line(Line {
                        begin: Vector3::default(),
                        end: Vector3::y_axis().scale(20.0),
                        color: Color::BLUE,
                    });
                    scene.drawing_context.add_line(Line {
                        begin: Vector3::default(),
                        end: Vector3::z_axis().scale(20.0),
                        color: Color::GREEN,
                    });

                    engine.update(fixed_timestep);
                }

                // It is very important to "pump" messages from UI. Even if don't need to
                // respond to such message, you should call this method, otherwise UI
                // might behave very weird.
                while let Some(_ui_event) = engine.user_interface.poll_message() {
                    // ************************
                    // Put your data model synchronization code here. It should
                    // take message and update data in your game according to
                    // changes in UI.
                    // ************************
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                engine.get_window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                engine.render(fixed_timestep).unwrap();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        engine.renderer.set_frame_size(size.into());
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                        if let Some(key_code) = input.virtual_keycode {
                            match key_code {
                                VirtualKeyCode::A => {
                                    input_controller.move_left =
                                        input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::D => {
                                    input_controller.move_right =
                                        input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::W => {
                                    input_controller.move_forward =
                                        input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::S => {
                                    input_controller.move_backward =
                                        input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::LShift => {
                                    input_controller.run = input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::Space => {
                                    input_controller.jump = input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::C => {
                                    input_controller.crouch = input.state == ElementState::Pressed
                                }
                                VirtualKeyCode::F => {
                                    if input.state == ElementState::Released {
                                        let scene = &mut engine.scenes[scene_handle];
                                        let flash_light =
                                            scene.graph[flash_light_handle].borrow_mut();
                                        let visibility = flash_light.visibility();
                                        flash_light.set_visibility(!visibility);
                                    }
                                }
                                VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::MouseMotion { delta } = event {
                    let (dx, dy) = delta;
                    camera_x += (dx as f32) * Player::MOUSE_SPEED;
                    camera_y += (dy as f32) * Player::MOUSE_SPEED;

                    camera_y = min_by(camera_y, 89.0, |a, b| a.partial_cmp(b).unwrap());
                    camera_y = max_by(camera_y, -89.0, |a, b| a.partial_cmp(b).unwrap());
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
