#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};
use pixels::{PixelsBuilder, SurfaceTexture, wgpu, Pixels};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::error::EventLoopError;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowAttributes, WindowId};

// This is the logical size of the window, for winit. The window will actually
// technically be 4x as many pixels as this, because of hidpi.
const WIN_SIZE: (u32, u32) = (640, 480);

// This is the logical size of the Pixels instance. This will get scaled up evenly
// to match the size of the window, which will get scaled again to match the hidpi
// factor. Confused yet?
const PIX_SIZE: (u32, u32) = (320, 240);

// We'll trigger an update and redraw this often. There's no real correct value here,
// it's just how often we want to update the game state (or whatever it is) but there
// is a wrong value: it turns out that specifying 15 milliseconds (about 60 hz) will
// drastically lengthen the time to draw a frame, due to vsync: rendering the Pixels
// will block until the next vsync, and our drawing will take nonzero time, so we'll
// end up always arriving late and waiting for the next redraw.
const TIMER_LENGTH: Duration = Duration::from_millis(20);

// The way Winit works now, we need an object that implements ApplicationHandler and it will
// receive events from the event loop. So, here is one.
#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'a>>,
    mouse_pos: (f64, f64)
}

impl<'a> ApplicationHandler for App<'a> {
    // This is the first event we get, and is sent when the application "resumes" on mobile,
    // or what we think of as "starting" everywhere else.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Some decently average window attributes: we set a title, size, and a minimum size
        // to restrict resizing.  Resizing up is fine, pixels will scale; resizing down is
        // problematic if we ever get smaller than the Pixels itself.
        let attrs = WindowAttributes::default()
            .with_title("The Thing")
            .with_inner_size(LogicalSize{ width: WIN_SIZE.0, height: WIN_SIZE.1 })
            .with_min_inner_size(LogicalSize { width: PIX_SIZE.0, height: PIX_SIZE.1 });

        // There's a problem here and this is the solution to it: Pixels needs a borrow of the window
        // (because that's a component of the SurfaceTexture) but doesn't want to own it. App can't own
        // it because we can't ensure it lives long enough. So, we have window be owned by an Arc,
        // which pixels can keep alive as long as it needs:
        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        // The Pixels instance. We need a backing surface texture the physical size of the window
        // (meaning, the real actual physical size, post-hidpi-scaling) and then we can set stuff
        // on it with a PixelsBuilder:
        let pixels = {
            let PhysicalSize { width, height } = window.inner_size();
            let surface_texture = SurfaceTexture::new(width, height, window.clone());
            PixelsBuilder::new(PIX_SIZE.0, PIX_SIZE.1, surface_texture)
                .clear_color(wgpu::Color{ r: 0.1, g: 0.1, b: 0.15, a: 1.0 })
                .build().expect("Failed to build pixels!")
        };

        // Winit makes us track the mouse position ourselves. This is a nice placeholder value for
        // before we get any mouse events
        self.mouse_pos = (-1f64, -1f64);

        self.window = Some(window.clone());
        self.pixels = Some(pixels);
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TIMER_LENGTH))
    }

    // This is called when any WindowEvent happens in the event loop, so we handle some of them:
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let our_id = self.window.as_ref().unwrap().id();
        if window_id != our_id { return }

        match event {
            // Exit if we click the little x
            WindowEvent::CloseRequested => { event_loop.exit(); }

            // Redraw if it's redrawing time
            WindowEvent::RedrawRequested => {
                // First redraw stuff into pixels' rgba buffer,
                // then have pixels draw itself into our scaled offset buffer:
                draw(self.pixels.as_mut().unwrap().frame_mut());
                self.pixels.as_ref().unwrap().render().unwrap()
            }

            // Update that the mouse moved if it did
            WindowEvent::CursorMoved { position: pos, device_id: _ } => {
                // Remember that there are two layers of scaling going on here, and this position
                // is after both of them: pos is two f64s in physical pixel coordinates.
                // To get a point in the WIN_SIZE space (in other words, to remove the hidpi
                // scaling only): pos.to_logical(window.scale_factor());
                // But it's probably more useful to store the raw physical point because
                // pixels.window_pos_to_pixel can remove both layers of scaling at once:
                self.mouse_pos = (pos.x, pos.y);
            }

            // Do something if the mouse was clicked
            WindowEvent::MouseInput { device_id: _, state: ElementState::Pressed, button: MouseButton::Left } => {
                println!("Mouse clicked:");
                println!("\tPhysical: {}, {}", self.mouse_pos.0, self.mouse_pos.1);
                if let Ok((px, py)) = self.pixels.as_mut().unwrap().window_pos_to_pixel((self.mouse_pos.0 as f32, self.mouse_pos.1 as f32)) {
                    println!("\tPixels: {}, {}", px, py)
                } else {
                    println!("\tNot within Pixels space!")
                }
            }

            // Handle keyboard events
            WindowEvent::KeyboardInput { event, .. } => {
                println!("{} {:?} ({}repeat)",
                         if event.state.is_pressed() { "Pressed" } else { "Released" },
                         event.logical_key,
                         if event.repeat { "" } else { "not " })
            }

            // Resize the texture when the window resizes (this will also handle rescaling
            // the Pixels instance)
            WindowEvent::Resized(new_size) => {
                println!("Resized to {}, {}", new_size.width, new_size.height);
                self.pixels.as_mut().unwrap().resize_surface(new_size.width, new_size.height).expect("Resize surface failure")
            }

            // Drop other events
            _ => {}
        }
    }

    // This fires when the event loop receives a batch of new events, which happens when the timer
    // fires for our redraw. We need to reschedule the times and redraw the window, if that's the
    // actual start cause. Also, call `update()` to update the game model:
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            update();
            self.window.as_ref().unwrap().request_redraw();
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TIMER_LENGTH));
        }
    }
}

// To actually run this, we create an event loop, set it to wait, and feed it an App.
// The App starts as a blank slate and the first resume event handles the initialization,
// which adds a lot of complexity (App needs to know how to exist in an uninitialized state)
// but is necessary for winit to work on mobile.
fn main() -> Result<(), EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop!");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)
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