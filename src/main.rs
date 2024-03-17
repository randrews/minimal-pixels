#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::time::{Duration, Instant};
use pixels::{PixelsBuilder, SurfaceTexture, wgpu};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::error::EventLoopError;
use winit::event::{ElementState, Event, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow};

// This is the logical size of the window, for winit. The window will actually
// technically be 4x as many pixels as this, because of hidpi.
const WIN_SIZE: (u32, u32) = (640, 480);

// This is the logical size of the Pixels instance. This will get scaled up evenly
// to match the size of the window, which will get scaled again to match the hidpi
// factor. Confused yet?
const PIX_SIZE: (u32, u32) = (320, 240);

fn main() -> Result<(), EventLoopError> {
    // We'll trigger an update and redraw this often
    let timer_length = Duration::from_millis(15);

    // winit now makes is track the mouse position ourselves...
    let mut mouse_pos: (f64, f64) = (-1f64, -1f64);

    // A window needs an event loop
    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop!");

    // The window itself. We set a title, size, and a minimum size to restrict resizing.
    // Resizing up is fine, pixels will scale; resizing down is problematic if we ever
    // get smaller than the Pixels itself.
    let window = winit::window::WindowBuilder::new()
        .with_title("The Thing")
        .with_inner_size(LogicalSize{ width: WIN_SIZE.0, height: WIN_SIZE.1 })
        .with_min_inner_size(LogicalSize { width: PIX_SIZE.0, height: PIX_SIZE.1 })
        .build(&event_loop)?;

    // The Pixels instance. We need a backing surface texture the physical size of the window
    // (meaning, the real actual physical size, post-hidpi-scaling) and then we can set stuff
    // on it with a PixelsBuilder:
    let mut pixels = {
        let PhysicalSize { width, height } = window.inner_size();
        let surface_texture = SurfaceTexture::new(width, height, &window);
        PixelsBuilder::new(PIX_SIZE.0, PIX_SIZE.1, surface_texture)
            .clear_color(wgpu::Color{ r: 0.1, g: 0.1, b: 0.15, a: 1.0 })
            .build().expect("Failed to build pixels!")
    };

    event_loop.run(move |event, target| {
        match event {
            // Exit if we click the little x
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => { target.exit(); }

            // Redraw if it's redrawing time
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
            } if window_id == window.id() => {
                // First redraw stuff into pixels' rgba buffer,
                // then have pixels draw itself into our scaled offset buffer:
                draw(pixels.frame_mut());
                pixels.render().unwrap()
            }

            // Start the timer on init
            Event::NewEvents(StartCause::Init) => {
                target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + timer_length));
            }

            // When the timer fires, update the world, redraw thw window based on that,
            // and restart the timer
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                update();
                window.request_redraw();
                target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + timer_length));
            }

            // Update that the mouse moved if it did
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position: pos, device_id: _ },
                window_id
            } if window_id == window.id() => {
                // Remember that there are two layers of scaling going on here, and this position
                // is after both of them: pos is two f64s in physical pixel coordinates.
                // To get a point in the WIN_SIZE space (in other words, to remove the hidpi
                // scaling only): pos.to_logical(window.scale_factor());
                // But it's probably more useful to store the raw physical point because
                // pixels.window_pos_to_pixel can remove both layers of scaling at once:
                mouse_pos = (pos.x, pos.y);
            }

            // Do something if the mouse was clicked
            Event::WindowEvent {
                window_id, event: WindowEvent::MouseInput { device_id: _, state: ElementState::Pressed, button: MouseButton::Left }
            } if window_id == window.id() => {
                println!("Mouse clicked:");
                println!("\tPhysical: {}, {}", mouse_pos.0, mouse_pos.1);
                if let Ok((px, py)) = pixels.window_pos_to_pixel((mouse_pos.0 as f32, mouse_pos.1 as f32)) {
                    println!("\tPixels: {}, {}", px, py)
                } else {
                    println!("\tNot within Pixels space!")
                }
            }

            // Handle keyboard events
            Event::WindowEvent {
                window_id, event: WindowEvent::KeyboardInput { event, .. }
            } if window_id == window.id() => {
                println!("{} {:?} ({}repeat)",
                         if event.state.is_pressed() { "Pressed" } else { "Released" },
                         event.logical_key,
                         if event.repeat { "" } else { "not " })
            }

            // Resize the texture when the window resizes (this will also handle rescaling
            // the Pixels instance)
            Event::WindowEvent {
                window_id, event: WindowEvent::Resized(new_size)
            } if window_id == window.id() => {
                println!("Resized to {}, {}", new_size.width, new_size.height);
                pixels.resize_surface(new_size.width, new_size.height).expect("Resize surface failure")
            }

            // Drop other events
            _ => {}
        }
    })
}

// Called to draw the window. It's just a big slice of RGBA bytes, PIX_SIZE in
// dimensions.
fn draw(frame: &mut [u8]) {
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        let x = (i % PIX_SIZE.0 as usize) as i16;
        let y = (i / PIX_SIZE.0 as usize) as i16;

        if x > 50 && x < 100 && y > 50 && y < 100 {
            pixel.copy_from_slice(&[0xff, 0xff, 0x50, 0xff])
        }
    }
}

fn update() {
    // Do nothing
}