#![feature(cmp_min_max_by)]

extern crate rg3d;

use std::cmp::{max_by, min_by};
use std::time::Instant;

use rg3d::engine::resource_manager::TextureImportOptions;
use rg3d::gui::message::MessageDirection;
use rg3d::renderer::QualitySettings;
use rg3d::resource::texture::{TextureMagnificationFilter, TextureMinificationFilter};
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

use crate::level_generator::{Field, Level, RoomOptions};

mod level_generator;

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

const PLAYER_SPEED: f32 = 0.7f32;
const EXTRA_RUN_SPEED: f32 = 0.7f32;
const MOUSE_SPEED: f32 = 0.15f32;

struct GameScene {
    scene: Scene,
    // model_handle: Handle<Node>,
    camera_handle: Handle<Node>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    resource_manager.state().set_textures_import_options(
        TextureImportOptions::default()
            .with_minification_filter(TextureMinificationFilter::Nearest)
            .with_magnification_filter(TextureMagnificationFilter::Nearest),
    );

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vec3::new(0.0, 6.0, -12.0))
                .build(),
        ),
    )
    .build();

    let camera_handle = scene.graph.add_node(Node::Camera(camera));

    scene.graph[camera_handle]
        .local_transform_mut()
        .offset(Vec3::new(20.0 * 50.0, 10.0, 20.0 * 50.0));

    let model_resource = resource_manager
        .request_model("assets/items.fbx")
        .await
        .unwrap();
    let floor_resource = resource_manager
        .request_model("assets/floor.fbx")
        .await
        .unwrap();

    // create level
    let level = Level::create_dungeon(
        99,
        99,
        RoomOptions {
            max_rooms: 9,
            max_attempts: 125,
            min_size: 7,
            max_size: 30,
        },
        Field::Floor,
        Field::Corridor,
    );

    let mut tile_count = 0;

    for x in 0..level.map.len() {
        for y in 0..level.map[0].len() {
            if level.map[x][y] == Field::Empty {
                continue;
            }

            tile_count += 1;

            let floor_handle = floor_resource.instantiate_geometry(&mut scene);

            scene.graph[floor_handle]
                .local_transform_mut()
                .set_scale(Vec3::new(0.2, 0.2, 0.2))
                .offset(Vec3::new(20.0 * x as f32, 0.0, 20.0 * y as f32));
        }
    }
    println!("TILES: {}", tile_count);

    let model_handle = model_resource.instantiate_geometry(&mut scene);

    scene.graph[model_handle]
        .local_transform_mut()
        // Our model is too big, fix it by scale.
        .set_position(Vec3::new(20.0, 20.0, 20.0));

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

    if let Err(err) = engine
        .renderer
        .set_quality_settings(&QualitySettings::low())
    {
        panic!("{:?}", err);
    }

    engine.resource_manager.state().set_textures_path("assets");

    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    let GameScene {
        scene,
        camera_handle,
    } = rg3d::futures::executor::block_on(create_scene(engine.resource_manager.clone()));

    let scene_handle = engine.scenes.add(scene);

    engine
        .renderer
        .set_ambient_color(Color::opaque(200, 200, 200));

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
