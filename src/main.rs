#![feature(cmp_min_max_by)]

extern crate rg3d;

use std::cmp::{max_by, min_by};
use std::time::Instant;

use rg3d::engine::resource_manager::TextureImportOptions;
use rg3d::gui::message::MessageDirection;
use rg3d::renderer::QualitySettings;
use rg3d::resource::texture::{TextureMagnificationFilter, TextureMinificationFilter};
use rg3d::scene::light::{BaseLightBuilder, PointLightBuilder};
use rg3d::{
    core::{
        color::Color,
        math::{quat::Quat, vec3::Vec3},
        pool::Handle,
    },
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
use rg3d::scene::Line;
use crate::sound::start_ambient_sound;

mod level_generator;
mod sound;

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

const PLAYER_SPEED: f32 = 0.03;
const EXTRA_RUN_SPEED: f32 = 0.04;
const MOUSE_SPEED: f32 = 0.15;

struct GameScene {
    scene: Scene,
    camera_handle: Handle<Node>,
}

fn create_point_light(radius: f32) -> Node {
    let point_light = PointLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()));

    point_light.with_radius(radius).build_node()
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
                        .set_rotation(Quat::from_axis_angle(Vec3::UP, rotation.to_radians()))
                        .offset(Vec3::new(x as f32, 0.0, y as f32));
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
                    .set_rotation(Quat::from_axis_angle(Vec3::UP, rotation.to_radians()))
                    .offset(Vec3::new(x as f32, 0.0, y as f32));
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
                .offset(Vec3::new(x as f32, 0.0, y as f32));

            // add light to floors
            if level.map[x][y].typ == FieldType::Corridor && tile_count % 3 == 0 {
                let handle = scene.graph.add_node(create_point_light(1.0));
                scene.graph[handle]
                    .local_transform_mut()
                    .offset(Vec3::new(x as f32, 0.3, y as f32));
            }

            // fill in missing walls
            let add_wall = |scene: &mut Scene, rotation: f32, offset_x: f32, offset_y: f32| {
                let wall_handle = wall_resource.instantiate_geometry(scene);
                scene.graph[wall_handle]
                    .local_transform_mut()
                    .set_rotation(Quat::from_axis_angle(Vec3::UP, rotation.to_radians()))
                    .offset(Vec3::new(x as f32 + offset_x, 0.0, y as f32 + offset_y));
            };

            let neighbours = level
                .get_neighbours((x, y), 1)
                .into_iter()
                .filter(|(x, y)| level.map[*x][*y].typ == FieldType::Empty)
                .collect::<Vec<_>>();

            for n in neighbours {
                let walls = &level.map[x][y].walls;
                if n.0 < x {
                    if !walls.left_up {
                        add_wall(scene, 90.0, 0.0, -0.5);
                    }
                    if !walls.left_down {
                        add_wall(scene, 90.0, 0.0, 0.0);
                    }
                }
                if n.0 > x {
                    if !walls.right_up {
                        add_wall(scene, -90.0, 0.0, 0.0);
                    }
                    if !walls.right_down {
                        add_wall(scene, -90.0, 0.0, 0.5);
                    }
                }
                if n.1 < y {
                    if !walls.up_left {
                        add_wall(scene, 0.0, 0.0, 0.0);
                    }
                    if !walls.up_right {
                        add_wall(scene, 0.0, 0.5, 0.0);
                    }
                }
                if n.1 > y {
                    if !walls.down_left {
                        add_wall(scene, 180.0, -0.5, 0.0);
                    }
                    if !walls.down_right {
                        add_wall(scene, 180.0, 0.0, 0.0);
                    }
                }
            }
        }
    }
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
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

    // add lights to rooms
    for room in level.rooms.iter_mut() {
        room.sort();
        let pos = room[room.len() / 2];
        let point_light = scene.graph.add_node(pl.raw_copy());

        scene.graph[point_light]
            .local_transform_mut()
            .set_position(Vec3::new(pos.0 as f32, 1.0, pos.1 as f32));
    }

    let camera = CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vec3::new(7.0, 0.5, 7.0))
                .build(),
        ),
    )
    .build();

    let camera_handle = scene.graph.add_node(Node::Camera(camera));

    GameScene {
        scene,
        camera_handle,
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
        point_shadow_map_size: 512,
        point_shadows_distance: 10.0,
        point_shadows_enabled: true,
        point_soft_shadows: true,

        spot_shadow_map_size: 512,
        spot_shadows_distance: 10.0,
        spot_shadows_enabled: true,
        spot_soft_shadows: true,

        use_ssao: false,
        ssao_radius: 0.5,

        light_scatter_enabled: false,
    };
    if let Err(err) = engine.renderer.set_quality_settings(&quality_settings) {
        panic!("{:?}", err);
    }

    engine.resource_manager.state().set_textures_path("assets");
    engine.get_window().set_cursor_visible(false);

    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    let GameScene {
        scene,
        camera_handle,
    } = rg3d::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    let scene_handle = engine.scenes.add(scene);

    start_ambient_sound();

    engine.renderer.set_ambient_color(Color::opaque(30, 30, 30));

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
                            Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), -camera_x.to_radians())
                                * Quat::from_axis_angle(
                                    Vec3::new(1.0, 0.0, 0.0),
                                    camera_y.to_radians(),
                                ),
                        );

                    let side = scene.graph[camera_handle].side_vector();
                    let mut back_front = scene.graph[camera_handle].look_vector();
                    back_front.y = 0.0;
                    back_front = back_front.normalized().unwrap_or(Vec3::ZERO);

                    let mut offset = Vec3::ZERO;

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

                    let speed = if input_controller.run {
                        PLAYER_SPEED + EXTRA_RUN_SPEED
                    } else {
                        PLAYER_SPEED
                    };

                    offset.x *= speed;
                    offset.z *= speed;

                    if input_controller.jump {
                        offset.y += speed;
                    }
                    if input_controller.crouch {
                        offset.y -= speed;
                    }

                    let pos = scene.graph[camera_handle].local_transform().position();
                    scene.graph[camera_handle]
                        .local_transform_mut()
                        .set_position(pos + offset);

                    let fps = engine.renderer.get_statistics().frames_per_second;
                    let text = format!("FPS: {}", fps);

                    engine.user_interface.send_message(TextMessage::text(
                        debug_text,
                        MessageDirection::ToWidget,
                        text,
                    ));

                    // for debugging
                    scene.drawing_context.clear_lines();
                    scene.drawing_context.add_line(Line {
                        begin: Vec3::ZERO,
                        end: Vec3::X.scale(20.0),
                        color: Color::RED,
                    });
                    scene.drawing_context.add_line(Line {
                        begin: Vec3::ZERO,
                        end: Vec3::Y.scale(20.0),
                        color: Color::BLUE,
                    });
                    scene.drawing_context.add_line(Line {
                        begin: Vec3::ZERO,
                        end: Vec3::Z.scale(20.0),
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
                    camera_x += (dx as f32) * MOUSE_SPEED;
                    camera_y += (dy as f32) * MOUSE_SPEED;

                    camera_y = min_by(camera_y, 89.0, |a, b| a.partial_cmp(b).unwrap());
                    camera_y = max_by(camera_y, -89.0, |a, b| a.partial_cmp(b).unwrap());
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
