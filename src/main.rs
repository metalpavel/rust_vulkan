///
/// Enable debug logging: $env:RUST_LOG="debug"
///

mod app;

use log::*;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder};

fn main() {
    pretty_env_logger::init();
    info!("Creating app...");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Rust")
        .with_inner_size(LogicalSize::new(1024, 768))
        .build(&event_loop).unwrap();

        let mut app = unsafe { app::App::create(&window).unwrap() };
        let mut destroying = false;
        let mut minimized = false;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {

                Event::MainEventsCleared if !destroying && !minimized => unsafe { app.render(&window) }.unwrap(),

                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    if size.width == 0 || size.height == 0 {
                        minimized = true;
                    } else {
                        minimized = false;
                        app.resized = true;
                    }
                }

                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    destroying = true;
                    *control_flow = ControlFlow::Exit;
                    unsafe { app.destroy(); }
                }

                _ => {}
            }
        });
}
