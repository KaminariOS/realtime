use std::rc::Rc;
use std::time::Duration;
use instant::Instant;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use crate::state::State;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod state;
mod types;
mod gui;
mod camera;
mod texture;

#[cfg(target_arch = "wasm32")]
const SAMPLE_COUNT: u32 = 1;
#[cfg(not(target_arch = "wasm32"))]
const SAMPLE_COUNT: u32 = 4;


#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {

    #[cfg(target_arch = "wasm32")] {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
    }
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window = Rc::new(window);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowExtWebSys;
        use winit::dpi::LogicalSize;
        // Retrieve current width and height dimensions of browser client window
        let get_window_size = || {
            let client_window = web_sys::window().unwrap();
            LogicalSize::new(
                client_window.inner_width().unwrap().as_f64().unwrap(),
                client_window.inner_height().unwrap().as_f64().unwrap(),
            )
        };

        let window = Rc::clone(&window);

        // Initialize winit window with current dimensions of browser client
        window.set_inner_size(get_window_size());

        let client_window = web_sys::window().unwrap();

        // Attach winit canvas to body element
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");

        // Listen for resize event on browser client. Adjust winit window dimensions
        // on event trigger
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let size = get_window_size();
            window.set_inner_size(size)
        }) as Box<dyn FnMut(_)>);
        client_window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let mut state = State::new(window.clone(), &event_loop).await;
    // let mut http_resp = None;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            // NEW!
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => if state.mouse_pressed {
                // state.camera_controller.process_mouse(delta.0, delta.1)
            }
            // UPDATED!
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() && !state.input(event, &window) => {
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                    => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => {
                        // if !state.menu_mode() {
                        //     state.open_menu()
                        // }
                        // else
                        if state.mouse_pressed {
                            window.set_cursor_grab(false).ok();
                            window.set_cursor_visible(true);
                            state.mouse_pressed = false;
                        } else {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {
                        state.gui.handle_event(&event);
                    }
                }
            }
            // UPDATED!
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();

                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::RedrawEventsCleared => {
                        // Clamp to some max framerate to avoid busy-looping too much
                        // (we might be in wgpu::PresentMode::Mailbox, thus discarding superfluous frames)
                        //
                        // winit has window.current_monitor().video_modes() but that is a list of all full screen video modes.
                        // So without extra dependencies it's a bit tricky to get the max refresh rate we can run the window on.
                        // Therefore we just go with 60fps - sorry 120hz+ folks!
                        let target_frametime = Duration::from_secs_f64(1.0 / 60.0);
                        let time_since_last_frame = state.last_update_time.elapsed();
                        if time_since_last_frame >= target_frametime {
                            window.request_redraw();
                            state.last_update_time = Instant::now();
                        } else {
                            *control_flow = ControlFlow::WaitUntil(
                                Instant::now() + target_frametime - time_since_last_frame,
                            );
                        }
            }
            _ => {}
        }
    });
}