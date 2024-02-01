#[allow(unused_imports)]
use std::{
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    rc::Rc,
    cell::RefCell,
};

//use parking_lot::Mutex;
use three_d::*;
use wasm_thread as thread;
use bus::{Bus, BusReader};
use num_format::{Locale, ToFormattedString};

use crate::log; // macro import
use crate::utils::*;
use crate::scene::*;


#[derive(PartialEq)]
enum TdCameraControl { Orbit, Fly }


/// Re-implementation of three_d::OrbitControl to add right mouse button control
pub struct OrbitControl2 {
    control: CameraControl,
}
impl OrbitControl2 {
    /// Creates a new orbit control with the given target and minimum and maximum distance to the target.
    pub fn new(target: Vec3, min_distance: f32, max_distance: f32) -> Self {
        Self {
            control: CameraControl {
                left_drag_horizontal: CameraAction::OrbitLeft { target, speed: 0.1 },
                left_drag_vertical: CameraAction::OrbitUp { target, speed: 0.1 },
                scroll_vertical: CameraAction::Zoom {
                    min: min_distance,
                    max: max_distance,
                    speed: 0.001,
                    target,
                },
                right_drag_horizontal: CameraAction::Left { speed: 0.01 },
                right_drag_vertical: CameraAction::Up { speed: 0.01 },
                ..Default::default()
            },
        }
    }

    /// Handles the events. Must be called each frame.
    pub fn handle_events(&mut self, camera: &mut Camera, events: &mut [Event]) -> bool {

        // need to re-calculate the change so as to translate the target for orbit
        let mut change = Vec3::zero();
        for event in events.iter() {
            match event {
                Event::MouseMotion {
                    delta,
                    button,
                    handled,
                    ..
                } => {
                    if let Some(b) = button {
                        if let MouseButton::Right = b {
                            if let CameraAction::Left { speed } = &self.control.right_drag_horizontal {
                                change += -camera.right_direction() * delta.0 * (*speed);
                            }
                            if let CameraAction::Up { speed } = &self.control.right_drag_vertical {
                                let right = camera.right_direction();
                                let up = right.cross(camera.view_direction());
                                change += up * delta.1 * (*speed);
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        if let CameraAction::Zoom { speed, target, .. } = &mut self.control.scroll_vertical {
            let x = target.distance(*camera.position());
            *speed = 0.001 * x + 0.001;
            *target += change;
        }
        if let CameraAction::OrbitLeft { speed, target } = &mut self.control.left_drag_horizontal {
            let x = target.distance(*camera.position());
            *speed = 0.01 * x + 0.001;
            *target += change;
        }
        if let CameraAction::OrbitUp { speed, target } = &mut self.control.left_drag_vertical {
            let x = target.distance(*camera.position());
            *speed = 0.01 * x + 0.001;
            *target += change;
        }

        self.control.handle_events(camera, events)
    }
}


#[allow(unused_mut)]
fn launch_sorter_thread(
    scene: Arc<Scene>,
    mut rx_buffer: BusReader<Vec<u8>>,
    mut rx_vp: BusReader<Mat4>,
    mut bus_depth: Bus<Vec<u32>>,
    cpu_cores: usize,
    mut bus_time: Bus<f64>,
) -> thread::JoinHandle<()> {
    // launch another thread for view-dependent splat sorting
    let thread_handle = thread::spawn({
        let mut scene = scene.clone();

        move || loop {
            // receive splat binary buffer from async JS worker callback
            #[cfg(feature = "async_splat_stream")]
            if let Ok(buffer) = rx_buffer.try_recv() {
                /*
                FIXME: scene buffer needs to be duplicated here
                since Arc<Scene> does not have an interior mutability without a mutex
                (and mutex is not allowed in wasm main thread)
                */
                let mut s = Scene::new();
                s.buffer = buffer;
                s.splat_count = s.buffer.len() / 32; // 32bytes per splat
                //s.generate_texture(); // texture is created instead in render loop in main thread
                scene = Arc::new(s);
            }

            // receive view proj matrix from main thread
            if let Ok(view_proj) = rx_vp.try_recv() {
                let view_proj_slice = &[
                    view_proj[0][0], view_proj[0][1], view_proj[0][2], view_proj[0][3],
                    view_proj[1][0], view_proj[1][1], view_proj[1][2], view_proj[1][3],
                    view_proj[2][0], view_proj[2][1], view_proj[2][2], view_proj[2][3],
                    view_proj[3][0], view_proj[3][1], view_proj[3][2], view_proj[3][3]
                ];
                let start =  get_time_milliseconds();
                Scene::sort(&scene, view_proj_slice, &mut bus_depth, cpu_cores);
                let sort_time = get_time_milliseconds() - start;
                //////////////////////////////////
                // non-blocking (i.e., no atomic.wait)
                let _ = bus_time.try_broadcast(sort_time);
                //////////////////////////////////
            }
        }
    });

    thread_handle
}


/*
#[allow(unused_mut)]
fn launch_sorter_thread2(
    mut rx_buffer: BusReader<Vec<u8>>,
    mut rx_vp: BusReader<Mat4>,
    mut bus_depth: Bus<Vec<u32>>,
    cpu_cores: usize,
    mut bus_time: Bus<f64>,
) -> thread::JoinHandle<()> {
    // launch another thread for view-dependent splat sorting
    let thread_handle = thread::spawn({
        let mut scene = Scene::new();

        move || loop {
            // receive splat chunk from async JS worker callback
            #[cfg(feature = "async_splat_stream")]
            if let Ok(chunk) = rx_buffer.try_recv() {
                scene.buffer.extend(chunk);
                scene.splat_count = scene.buffer.len() / 32; // 32bytes per splat
            }

            // receive view proj matrix from main thread
            if let Ok(view_proj) = rx_vp.try_recv() {
                let view_proj_slice = &[
                    view_proj[0][0], view_proj[0][1], view_proj[0][2], view_proj[0][3],
                    view_proj[1][0], view_proj[1][1], view_proj[1][2], view_proj[1][3],
                    view_proj[2][0], view_proj[2][1], view_proj[2][2], view_proj[2][3],
                    view_proj[3][0], view_proj[3][1], view_proj[3][2], view_proj[3][3]
                ];
                let start =  get_time_milliseconds();
                Scene::sort2(&scene, view_proj_slice, &mut bus_depth, cpu_cores);
                let sort_time = get_time_milliseconds() - start;
                //////////////////////////////////
                // non-blocking (i.e., no atomic.wait)
                let _ = bus_time.try_broadcast(sort_time);
                //////////////////////////////////
            }
        }
    });

    thread_handle
}
*/


#[allow(unused_mut)]
pub async fn main() {
    let error_flag = Arc::new(AtomicBool::new(false));
    let error_msg = Arc::new(Mutex::new(String::new()));

    let cpu_cores = cpu_cores() as usize;
    log!("main(): cpu_cores={}", cpu_cores);

    let canvas_w = get_canvas_width();
    let canvas_h = get_canvas_height();
    log!("main(): canvas size: {}x{}", canvas_w, canvas_h);

    let window = Window::new(WindowSettings {
        title: "Gauzilla: 3D Gaussian Splatting in WASM + WebGL".to_string(),
        max_size: Some((canvas_w, canvas_h)),
        ..Default::default()
    })
    .unwrap();

    let gl = window.gl();
    log!("main(): OpenGL version: {:?}", gl.version());
    let glsl_ver = unsafe { gl.get_parameter_string(context::SHADING_LANGUAGE_VERSION) };
    log!("main(): GLSL version: {}", glsl_ver);

    let fovy = degrees(45.0);
    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(0.0, 0.0, 1.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        fovy,
        0.1,//0.2,
        10.0,//200.0,
    );
    let mut orbit_control = OrbitControl2::new(*camera.target(), 1.0, 100.0);
    let mut fly_control = FlyControl::new(0.005);
    let mut egui_control = TdCameraControl::Orbit;

    // lock-free bus for streamed scene buffer (single-send, multi-consumer)
    let mut bus_buffer = Bus::<Vec::<u8>>::new(1);
    let rx_buffer_threaded = bus_buffer.add_rx();
    let mut rx_buffer = bus_buffer.add_rx();
    let bus_buffer_rc =  Rc::new(RefCell::new(bus_buffer));

    // lock-free bus for scene buffer (single-send, single-consumer)
    let mut bus_progress = Bus::<f64>::new(10);
    let mut rx_progress = bus_progress.add_rx();
    let bus_progress_rc =  Rc::new(RefCell::new(bus_progress));

    #[cfg(feature = "async_splat_stream")]
    let worker_handle = stream_splat_in_worker(bus_buffer_rc, bus_progress_rc);
    #[cfg(feature = "async_splat_stream")]
    //let mut scene = Scene::new();
    let mut scene = Arc::new(Scene::new());
    #[cfg(not(feature = "async_splat_stream"))]
    let scene = Arc::new(load_scene().await);

    let mut gsplat_program: Option<context::Program> = None;
    let mut u_projection: Option<context::UniformLocation> = None;
    let mut u_viewport: Option<context::UniformLocation> = None;
    let mut u_focal: Option<context::UniformLocation> = None;
    let mut u_htan_fov: Option<context::UniformLocation> = None;
    let mut u_view: Option<context::UniformLocation> = None;
    let mut u_cam_pos: Option<context::UniformLocation> = None;
    let mut u_splat_scale: Option<context::UniformLocation> = None;

    let mut vertex_buffer: Option<context::WebBufferKey> = None;
    let mut a_position: u32 = 0;

    let mut texture: Option<context::WebTextureKey> = None;
    let mut u_texture: Option<context::UniformLocation> = None;

    let mut index_buffer: Option<context::WebBufferKey> = None;
    let mut a_index: u32 = 0;

    unsafe {
        let vert_shader = gl.create_shader(context::VERTEX_SHADER)
            .expect("Failed creating vertex shader");
        let frag_shader = gl.create_shader(context::FRAGMENT_SHADER)
            .expect("Failed creating fragment shader");
        /*
        let header: &str = {
            "#version 300 es
                #ifdef GL_FRAGMENT_PRECISION_HIGH
                    precision highp float;
                    precision highp int;
                    precision highp sampler2DArray;
                    precision highp sampler3D;
                #else
                    precision mediump float;
                    precision mediump int;
                    precision mediump sampler2DArray;
                    precision mediump sampler3D;
                #endif\n"
        };
        */
        let vertex_shader_source = include_str!("gsplat.vert");
        let fragment_shader_source = include_str!("gsplat.frag");
        //let vertex_shader_source = format!("{}{}", header, vertex_shader_source);
        //let fragment_shader_source = format!("{}{}", header, fragment_shader_source);

        gl.shader_source(vert_shader, &vertex_shader_source);
        gl.shader_source(frag_shader, &fragment_shader_source);
        gl.compile_shader(vert_shader);
        gl.compile_shader(frag_shader);

        let id = gl.create_program()
            .expect("Failed creating program");
        gsplat_program = Some(id);
        log!("main(): gsplat_program={:?}", gsplat_program);

        gl.attach_shader(id, vert_shader);
        gl.attach_shader(id, frag_shader);
        gl.link_program(id);

        if !gl.get_program_link_status(id) {
            let log = gl.get_shader_info_log(vert_shader);
            if !log.is_empty() {
                set_error_for_egui(
                    &error_flag, &error_msg,
                    format!("ERROR: gl.get_program_link_status(): {}", log)
                );
            }
            let log = gl.get_shader_info_log(frag_shader);
            if !log.is_empty() {
                set_error_for_egui(
                    &error_flag, &error_msg,
                    format!("ERROR: gl.get_program_link_status(): {}", log)
                );
            }
            let log = gl.get_program_info_log(id);
            if !log.is_empty() {
                set_error_for_egui(
                    &error_flag, &error_msg,
                    format!("ERROR: gl.get_program_link_status(): {}", log)
                );
            }
            //unreachable!();
        } else {
            gl.detach_shader(id, vert_shader);
            gl.detach_shader(id, frag_shader);
            gl.delete_shader(vert_shader);
            gl.delete_shader(frag_shader);

            gl.use_program(gsplat_program);
            {
                u_projection = gl.get_uniform_location(id, "projection");
                log!("main(): u_projection={:?}", u_projection);
                u_viewport = gl.get_uniform_location(id, "viewport");
                log!("main(): u_viewport={:?}", u_viewport);
                u_focal = gl.get_uniform_location(id, "focal");
                log!("main(): u_focal={:?}", u_focal);
                u_view = gl.get_uniform_location(id, "view");
                log!("main(): u_view={:?}", u_view);
                u_htan_fov = gl.get_uniform_location(id, "htan_fov");
                log!("main(): u_htan_fov={:?}", u_htan_fov);
                u_cam_pos = gl.get_uniform_location(id, "cam_pos");
                log!("main(): u_cam_pos={:?}", u_cam_pos);
                u_splat_scale = gl.get_uniform_location(id, "splat_scale");
                log!("main(): u_splat_scale={:?}", u_splat_scale);

                let triangle_vertices = &mut [ // quad
                    -1_f32, -1.0,
                    1.0, -1.0,
                    1.0, 1.0,
                    -1.0, 1.0,
                ];
                triangle_vertices.iter_mut().for_each(|v| *v *= 2.0);
                vertex_buffer = Some(gl.create_buffer().unwrap());
                log!("main(): vertex_buffer={:?}", vertex_buffer);
                gl.bind_buffer(context::ARRAY_BUFFER, vertex_buffer);
                gl.buffer_data_u8_slice(context::ARRAY_BUFFER, transmute_slice::<_, u8>(triangle_vertices), context::STATIC_DRAW);
                a_position = gl.get_attrib_location(id, "position").unwrap();
                log!("main(): a_position={:?}", a_position);
                gl.enable_vertex_attrib_array(a_position);
                gl.bind_buffer(context::ARRAY_BUFFER, vertex_buffer);
                gl.vertex_attrib_pointer_f32(a_position, 2, context::FLOAT, false, 0, 0);

                texture = Some(gl.create_texture().unwrap());
                log!("main(): texture={:?}", texture); // WebTextureKey(1v1)
                gl.bind_texture(context::TEXTURE_2D, texture);
                u_texture = gl.get_uniform_location(id, "u_texture");
                log!("main(): u_texture={:?}", u_texture);
                gl.uniform_1_i32(u_texture.as_ref(), 0); // associate the active texture unit with the uniform

                // index buffer for instanced rendering
                index_buffer = Some(gl.create_buffer().unwrap());
                log!("main(): index_buffer={:?}", index_buffer);
                //gl.bind_buffer(context::ARRAY_BUFFER, index_buffer);
                a_index = gl.get_attrib_location(id, "index").unwrap();
                log!("main(): a_index={:?}", a_index);
                gl.enable_vertex_attrib_array(a_index);
                gl.bind_buffer(context::ARRAY_BUFFER, index_buffer);
                gl.vertex_attrib_pointer_i32(a_index, 1, context::INT, 0, 0);
                gl.vertex_attrib_divisor(a_index, 1);
            }
            gl.use_program(None);

            gl.bind_texture(context::TEXTURE_2D, texture);
            gl.tex_parameter_i32(context::TEXTURE_2D, context::TEXTURE_WRAP_S, context::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(context::TEXTURE_2D, context::TEXTURE_WRAP_T, context::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(context::TEXTURE_2D, context::TEXTURE_MIN_FILTER, context::NEAREST as i32);
            gl.tex_parameter_i32(context::TEXTURE_2D, context::TEXTURE_MAG_FILTER, context::NEAREST as i32);

            #[cfg(not(feature = "async_splat_stream"))]
            gl.tex_image_2d(
                context::TEXTURE_2D,
                0,
                context::RGBA32UI as i32,
                scene.tex_width as i32,
                scene.tex_height as i32,
                0,
                context::RGBA_INTEGER,
                context::UNSIGNED_INT,
                Some(transmute_slice::<_, u8>(scene.tex_data.as_slice()))
            );

            //gl.active_texture(context::TEXTURE0);
            //gl.bind_texture(context::TEXTURE_2D, texture);
        }
    }

    // TODO: implement resize() for change in window size

    // lock-free bus for depth_index
    let mut bus_depth_threaded = Bus::<Vec<u32>>::new(10);
    let mut rx_depth = bus_depth_threaded.add_rx();

    // lock-free bus for view_proj_slice
    let mut bus_vp = Bus::<Mat4>::new(10);
    let rx_vp_threaded: BusReader<Matrix4<f32>> = bus_vp.add_rx();

    // lock-free bus for sort_time
    let mut bus_time_threaded = Bus::<f64>::new(10);
    let mut rx_time = bus_time_threaded.add_rx();

    let thread_handle = launch_sorter_thread(
        scene.clone(),
        rx_buffer_threaded,
        rx_vp_threaded,
        bus_depth_threaded,
        cpu_cores,
        bus_time_threaded,
    );

    /////////////////////////////////////////////////////////////////////////////////

    let mut gui = three_d::GUI::new(&gl);
    let mut pointer_over_gui = false;
    let mut splat_scale = 1_f32;
    let mut cam_roll = 0_f32;
    let mut prev_cam_roll = 0_f32;
    let mut flip_y = true;
    let mut frame_prev = get_time_milliseconds();
    let mut fps_ma = IncrementalMA::new(100);
    let mut sort_time = 0_f64;
    let mut sort_time_ma = IncrementalMA::new(100);
    let mut send_view_proj: bool = true;
    let mut progress = 0_f64;
    let mut s_temp = Scene::new();

    #[cfg(not(feature = "async_splat_stream"))]
    let done_streaming = true;
    #[cfg(feature = "async_splat_stream")]
    let mut done_streaming = false;

    window.render_loop(move |mut frame_input| {
        let error_flag = Arc::clone(&error_flag);
        let error_msg = Arc::clone(&error_msg);

        let now =  get_time_milliseconds();
        let fps =  1000.0 / (now - frame_prev);
        frame_prev = now;
        let fps = fps_ma.add(fps);

        if !error_flag.load(Ordering::Relaxed) {
            /////////////////////////////////////////////////////////////////////////////////////
            // receive sort_time from the second thread
            if let Ok(f) = rx_time.try_recv() {
                sort_time = sort_time_ma.add(f);
            }

            #[cfg(feature = "async_splat_stream")]
            if !done_streaming {
                // receive progress from async JS worker callback
                if let Ok(pct) = rx_progress.try_recv() {
                    progress = pct;
                }

                // receive splat binary buffer from async JS worker callback
                if let Ok(buffer) = rx_buffer.try_recv() {
                    let mut s = Scene::new();
                    s.buffer = buffer;
                    s.splat_count = s.buffer.len() / 32; // 32bytes per splat
                    s.generate_texture();
                    scene = Arc::new(s);

                    unsafe {
                        gl.bind_texture(context::TEXTURE_2D, texture);
                        gl.tex_image_2d(
                            context::TEXTURE_2D,
                            0,
                            context::RGBA32UI as i32,
                            scene.tex_width as i32,
                            scene.tex_height as i32,
                            0,
                            context::RGBA_INTEGER,
                            context::UNSIGNED_INT,
                            Some(transmute_slice::<_, u8>(scene.tex_data.as_slice()))
                        );
                    }

                    done_streaming = true;
                    send_view_proj = true;
                }

                /*
                // receive splat chunk from async JS worker callback
                if let Ok(chunk) = rx_buffer.try_recv() {
                    scene.buffer.extend(chunk);
                    scene.splat_count = scene.buffer.len() / 32; // 32bytes per splat
                }
                // FIXME
                log!("main(): progress={}", progress);
                if progress >= 1.0 {
                    log!("main(): done streaming");
                    worker_handle.terminate(); // no longer need to receive buffer

                    scene.generate_texture();
                    unsafe {
                        gl.bind_texture(context::TEXTURE_2D, texture);
                        gl.tex_image_2d(
                            context::TEXTURE_2D,
                            0,
                            context::RGBA32UI as i32,
                            scene.tex_width as i32,
                            scene.tex_height as i32,
                            0,
                            context::RGBA_INTEGER,
                            context::UNSIGNED_INT,
                            Some(transmute_slice::<_, u8>(scene.tex_data.as_slice()))
                        );
                    }

                    done_streaming = true;
                    send_view_proj = true;
                }
                */
            }

            /////////////////////////////////////////////////////////////////////////////////////

            camera.set_viewport(frame_input.viewport);

            for event in frame_input.events.iter() {
                send_view_proj = true;

                /*
                if let Event::MousePress {
                    button,
                    position,
                    modifiers,
                    ..
                } = event
                {
                    if *button == MouseButton::Right && !modifiers.ctrl {
                        log!("right mouse button pressed at {:?}", position);
                    }
                }
                */

                /*
                if let Event::MouseMotion {
                    delta,
                    button,
                    handled,
                    ..
                } = event {
                }
                */
            }

            if !pointer_over_gui {
                match egui_control {
                    TdCameraControl::Orbit => {
                        orbit_control.handle_events(&mut camera, &mut frame_input.events);
                    },
                    TdCameraControl::Fly => {
                        fly_control.handle_events(&mut camera, &mut frame_input.events);
                    },
                }
            }

            if flip_y {
                //camera.mirror_in_xz_plane(); // FIXME
                camera.roll(degrees(180.0));
                flip_y = false;
            }
            if !are_floats_equal(cam_roll, prev_cam_roll, 0.00001) {
                camera.roll(degrees(-prev_cam_roll));
                camera.roll(degrees(cam_roll));
                prev_cam_roll = cam_roll;
            }
        }

        let view_matrix: &Mat4 = camera.view();
        let view_slice = &[
            view_matrix[0][0], view_matrix[0][1], view_matrix[0][2], view_matrix[0][3],
            view_matrix[1][0], view_matrix[1][1], view_matrix[1][2], view_matrix[1][3],
            view_matrix[2][0], view_matrix[2][1], view_matrix[2][2], view_matrix[2][3],
            view_matrix[3][0], view_matrix[3][1], view_matrix[3][2], view_matrix[3][3]
        ];
        let projection_matrix: &Mat4 = camera.projection();
        let projection_slice = &[
            projection_matrix[0][0], projection_matrix[0][1], projection_matrix[0][2], projection_matrix[0][3],
            projection_matrix[1][0], projection_matrix[1][1], projection_matrix[1][2], projection_matrix[1][3],
            projection_matrix[2][0], projection_matrix[2][1], projection_matrix[2][2], projection_matrix[2][3],
            projection_matrix[3][0], projection_matrix[3][1], projection_matrix[3][2], projection_matrix[3][3]
        ];
        let w = camera.viewport().width as f32;
        let h = camera.viewport().height as f32;
        let cam_pos = camera.position();
        let fx = 0.5*projection_matrix[0][0]*w;
        let fy = -0.5*projection_matrix[1][1]*h;
        let htany = (fovy / 2.0).tan() as f32;
        let htanx = (htany/h)*w;
        //let focal = h / (2.0 * htany); // == fx == -fy

        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |gui_context| {
                pointer_over_gui = gui_context.is_using_pointer();//.is_pointer_over_area();

                if error_flag.load(Ordering::Relaxed) {
                    egui::Window::new("Error")
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(gui_context, |ui| {
                            {
                                let mutex = error_msg.lock().unwrap();
                                ui.colored_label(egui::Color32::RED, &(*mutex))
                            }
                            /*
                            if ui.button("Ok").clicked() {
                                error_flag.store(false, Ordering::Relaxed);
                            }
                            */
                        });
                } else {
                    if !done_streaming {
                        egui::Window::new("Loading...")
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(gui_context, |ui| {
                                let progress_bar = egui::ProgressBar::new(progress as f32)
                                    .show_percentage()
                                    .animate(false);
                                ui.add(progress_bar);

                            });
                    } else {
                        egui::Window::new("Gauzilla")
                            //.vscroll(true)
                            .show(gui_context, |ui| {
                            /*
                            // TODO: open a PLY file as bytes and process it
                            if ui.button("Open PLY file").clicked() {
                                let task = rfd::AsyncFileDialog::new()
                                    .add_filter("ply", &["ply"])
                                    .pick_file();
                                execute_future(async move {
                                    let file = task.await;
                                    if let Some(f) = file {
                                        let bytes = f.read().await;
                                        match Scene::parse_file_header(bytes) {
                                            Ok((file_header_size, splat_count, mut cursor)) => {

                                            },
                                            Err(s) => set_error_for_egui(
                                                &error_flag, &error_msg, String::from("ERROR: could not open the selected file.\
                                                Choose a correctly formatted PLY file for 3D Gaussian Splatting.")
                                            ),
                                        }
                                    }
                                });
                                ui.close_menu();
                            }
                            */

                            egui::Grid::new("my_grid")
                                .num_columns(2)
                                .spacing([40.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new("FPS"));
                                    ui.label(format!("{:.2}", fps));
                                    ui.end_row();

                                    ui.add(egui::Label::new("CPU Sort Time (ms)"));
                                    ui.label(format!("{:.2}", sort_time));
                                    ui.end_row();

                                    ui.add(egui::Label::new("CPU Cores"));
                                    ui.label(format!("{}", cpu_cores));
                                    ui.end_row();

                                    ui.add(egui::Label::new("GL Version"));
                                    ui.label(format!("{:?}", gl.version()));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Splat Count"));
                                    ui.label(format!("{}", scene.splat_count.to_formatted_string(&Locale::en)));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Splat Scale"));
                                    ui.add(egui::Slider::new(&mut splat_scale, 0.1..=1.0));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Invert Y"));
                                    ui.checkbox(&mut flip_y, "");
                                    ui.end_row();

                                    ui.add(egui::Label::new("Window Size"));
                                    ui.label(format!("{}x{}", w, h));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Focal"));
                                    ui.label(format!("({:.2}, {:.2})", fx, fy));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Htan FOV"));
                                    ui.label(format!("({:.2}, {:.2})", htanx, htany));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Camera Position"));
                                    ui.label(format!("({:.2}, {:.2}, {:.2})", cam_pos.x, cam_pos.y, cam_pos.z));
                                    ui.end_row();

                                    ui.add(egui::Label::new("Camera Control"));
                                    ui.horizontal(|ui| {
                                        ui.radio_value(&mut egui_control, TdCameraControl::Orbit, "Orbit");
                                        ui.radio_value(&mut egui_control, TdCameraControl::Fly, "Fly");
                                    });
                                    ui.end_row();

                                    ui.add(egui::Label::new("Camera Roll"));
                                    ui.add(egui::Slider::new(&mut cam_roll, -180.0..=180.0).suffix("°"));
                                    ui.end_row();

                                    ui.add(egui::Label::new("GitHub"));
                                    use egui::special_emojis::GITHUB;
                                    ui.hyperlink_to(
                                        format!("{GITHUB} BladeTransformerLLC/gauzilla"),
                                        "https://github.com/BladeTransformerLLC/gauzilla",
                                    );
                                    ui.end_row();
                                });
                        });
                    }
                }
            },
        );

        if !error_flag.load(Ordering::Relaxed) {
            // send view_proj to thread only when it's changed by user input
            if done_streaming && send_view_proj  {
                let view_proj = projection_matrix * view_matrix;
                //////////////////////////////////
                // non-blocking (i.e., no atomic.wait)
                let _ = bus_vp.try_broadcast(view_proj);
                //////////////////////////////////
                send_view_proj = false;
            }

            unsafe {
                gl.viewport(0, 0, w as i32, h as i32);
                gl.clear(context::COLOR_BUFFER_BIT);

                gl.use_program(gsplat_program);
                {
                    gl.disable(context::DEPTH_TEST);
                    gl.disable(context::CULL_FACE);
                    //gl.cull_face(context::FRONT);

                    // FIXME
                    gl.enable(context::BLEND);
                    /*
                    gl.clear_color(0.0, 0.0, 0.0, 1.0);
                    gl.blend_func(context::SRC_ALPHA, context::ONE_MINUS_SRC_ALPHA);
                    //gl.blend_func(context::ONE_MINUS_SRC_ALPHA, context::SRC_ALPHA);
                    */
                    /*
                    //gl.clear_color(0.0, 0.0, 0.0, 0.0);
                    gl.blend_func_separate(
                        context::ONE_MINUS_DST_ALPHA,
                        context::ONE,
                        context::ONE_MINUS_DST_ALPHA,
                        context::ONE,
                    );
                    gl.blend_equation_separate(context::FUNC_ADD, context::FUNC_ADD);
                    */

                    gl.uniform_matrix_4_f32_slice(u_projection.as_ref(), false, projection_slice);
                    gl.uniform_matrix_4_f32_slice(u_view.as_ref(), false, view_slice);
                    gl.uniform_1_i32(u_texture.as_ref(), 0); // associate the active texture unit with the uniform
                    gl.uniform_2_f32_slice(u_focal.as_ref(), &[fx.abs(), fy.abs()]);
                    gl.uniform_2_f32_slice(u_viewport.as_ref(), &[w, h]);
                    gl.uniform_2_f32_slice(u_htan_fov.as_ref(), &[htanx, htany]);
                    gl.uniform_3_f32_slice(u_cam_pos.as_ref(), &[cam_pos.x, cam_pos.y, cam_pos.z]);
                    gl.uniform_1_f32(u_splat_scale.as_ref(), splat_scale);

                    gl.active_texture(context::TEXTURE0);
                    gl.bind_texture(context::TEXTURE_2D, texture);

                    gl.enable_vertex_attrib_array(a_position);
                    gl.bind_buffer(context::ARRAY_BUFFER, vertex_buffer);
                    gl.vertex_attrib_pointer_f32(a_position, 2, context::FLOAT, false, 0, 0);

                    gl.enable_vertex_attrib_array(a_index);
                    gl.bind_buffer(context::ARRAY_BUFFER, index_buffer);
                    //////////////////////////////////
                    // non-blocking (i.e., no atomic.wait)
                    if let Ok(depth_index) = rx_depth.try_recv() {
                        gl.buffer_data_u8_slice(
                            context::ARRAY_BUFFER,
                            transmute_slice::<_, u8>(depth_index.as_slice()),
                            context::DYNAMIC_DRAW
                        );
                    }
                    //////////////////////////////////
                    gl.vertex_attrib_pointer_i32(a_index, 1, context::INT, 0, 0);
                    gl.vertex_attrib_divisor(a_index, 1);

                    gl.draw_arrays_instanced(
                        context::TRIANGLE_FAN,
                        0,
                        4,
                        scene.splat_count as i32
                    );
                }
                gl.use_program(None);

                gui.render();
                gl.flush();
            }
        } else {
            gui.render();
        }

        // Returns default frame output to end the frame
        FrameOutput::default()
    });

    // thread exit is not implemented in rustwasm yet
    // https://rustwasm.github.io/2018/10/24/multithreading-rust-and-wasm.html
    let _ = thread_handle.join();

}


