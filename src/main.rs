use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder, WindowLevel};

#[cfg(windows)]
fn make_window_transparent(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmEnableBlurBehindWindow, DWM_BLURBEHIND};
    use windows::Win32::Graphics::Gdi::HRGN;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetLayeredWindowAttributes,
        SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
    };
    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
        let _ = SetLayeredWindowAttributes(hwnd, None, 255, LWA_ALPHA);
        let blur = DWM_BLURBEHIND {
            dwFlags: 1,
            fEnable: true.into(),
            hRgnBlur: HRGN::default(),
            fTransitionOnMaximized: false.into(),
        };
        let _ = DwmEnableBlurBehindWindow(hwnd, &blur);
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let window = Rc::new(
        WindowBuilder::new()
            .with_title("AVG")
            .with_inner_size(LogicalSize::new(1280.0f64, 720.0f64))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .build(&event_loop)
            .unwrap()
    );

    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                make_window_transparent(win32.hwnd.get() as isize);
            }
        }
    }

    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                elwt.exit();
            }
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                let size = window.inner_size();
                let width = NonZeroU32::new(size.width).unwrap();
                let height = NonZeroU32::new(size.height).unwrap();
                surface.resize(width, height).unwrap();

                let mut buffer = surface.buffer_mut().unwrap();

                // Fully transparent background
                for pixel in buffer.iter_mut() {
                    *pixel = 0x00000000;
                }

                // Test white button rectangle
                for y in 100u32..160u32 {
                    for x in 100u32..220u32 {
                        if x < size.width && y < size.height {
                            buffer[(y * size.width + x) as usize] = 0xB4FFFFFF;
                        }
                    }
                }

                buffer.present().unwrap();
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}