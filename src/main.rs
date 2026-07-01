mod app;
mod input;
mod layout;
mod render;

use app::App;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::event::{ElementState, Event, MouseButton, Touch, TouchPhase, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder, WindowLevel};

#[cfg(windows)]
fn make_window_transparent(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmEnableBlurBehindWindow, DWM_BLURBEHIND};
    use windows::Win32::Graphics::Gdi::HRGN;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW,
        GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
    };
    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE,
            style | WS_EX_LAYERED.0 as isize);
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

#[cfg(windows)]
use std::cell::RefCell;

#[cfg(windows)]
struct HitTestData {
    controls: Vec<crate::layout::Control>,
    scale: f32,
}

#[cfg(windows)]
thread_local! {
    static HIT_TEST_DATA: RefCell<Option<HitTestData>> = RefCell::new(None);
    static OLD_WND_PROC: std::cell::Cell<windows::Win32::UI::WindowsAndMessaging::WNDPROC> = std::cell::Cell::new(None);
    static ACTIVE_INTERACTION: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[cfg(windows)]
fn update_hit_test_data(controls: Vec<crate::layout::Control>, scale: f32) {
    HIT_TEST_DATA.with(|cell| {
        *cell.borrow_mut() = Some(HitTestData { controls, scale });
    });
}

#[cfg(windows)]
unsafe extern "system" fn wnd_proc_subclass(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, WM_NCHITTEST, HTCLIENT, HTTRANSPARENT,
    };
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::Foundation::POINT;

    if msg == WM_NCHITTEST {
        if ACTIVE_INTERACTION.with(|cell| cell.get()) {
            return windows::Win32::Foundation::LRESULT(HTCLIENT as isize);
        }
        let x = (lparam.0 & 0xffff) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
        let mut pt = POINT { x, y };
        let _ = unsafe { ScreenToClient(hwnd, &mut pt) };

        let hit = HIT_TEST_DATA.with(|cell| {
            if let Some(data) = &*cell.borrow() {
                let lx = pt.x as f32 / data.scale;
                let ly = pt.y as f32 / data.scale;
                data.controls.iter().any(|c| c.contains(lx, ly))
            } else {
                false
            }
        });

        if hit {
            return windows::Win32::Foundation::LRESULT(HTCLIENT as isize);
        } else {
            return windows::Win32::Foundation::LRESULT(HTTRANSPARENT as isize);
        }
    }

    let old_proc = OLD_WND_PROC.with(|cell| cell.get());
    unsafe { CallWindowProcW(old_proc, hwnd, msg, wparam, lparam) }
}


fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();

    let window = Rc::new(
        WindowBuilder::new()
            .with_title("AVG")
            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .build(&event_loop)
            .unwrap(),
    );

    #[cfg(windows)]
    let _hwnd = {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let mut h: isize = 0;
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                h = win32.hwnd.get() as isize;
            }
        }
        make_window_transparent(h);

        // Subclass the window procedure to handle WM_NCHITTEST dynamically
        let hwnd_val = windows::Win32::Foundation::HWND(h as *mut _);
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowLongPtrW, GWLP_WNDPROC, WNDPROC,
            };
            let new_proc: WNDPROC = Some(wnd_proc_subclass);
            let old_proc_ptr = SetWindowLongPtrW(hwnd_val, GWLP_WNDPROC, std::mem::transmute(new_proc));
            let old_wnd_proc: WNDPROC = std::mem::transmute(old_proc_ptr);
            OLD_WND_PROC.with(|cell| cell.set(old_wnd_proc));
        }
        h
    };

    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();
    let mut app = App::new();
    let size = window.inner_size();
    app.init_renderer(size.width, size.height);
    #[cfg(windows)]
    update_hit_test_data(app.layout.controls.clone(), app.scale);

    let mut mouse_pos = (0.0f32, 0.0f32);

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => elwt.exit(),

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. }, ..
            } => {
                mouse_pos = (position.x as f32, position.y as f32);
                #[cfg(windows)]
                {
                    if ACTIVE_INTERACTION.with(|cell| cell.get()) {
                        app.on_move(0, mouse_pos.0, mouse_pos.1);
                        window.request_redraw();
                    }
                }
            }

            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button: MouseButton::Left, .. }, ..
            } => {
                match state {
                    ElementState::Pressed  => {
                        #[cfg(windows)]
                        ACTIVE_INTERACTION.with(|cell| cell.set(true));
                        app.on_press(0, mouse_pos.0, mouse_pos.1);
                    }
                    ElementState::Released => {
                        #[cfg(windows)]
                        ACTIVE_INTERACTION.with(|cell| cell.set(false));
                        app.on_release(0);
                    }
                }
                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(Touch { phase, location, id, .. }), ..
            } => {
                let (x, y) = (location.x as f32, location.y as f32);
                match phase {
                    TouchPhase::Started => {
                        #[cfg(windows)]
                        ACTIVE_INTERACTION.with(|cell| cell.set(true));
                        app.on_press(id, x, y);
                    }
                    TouchPhase::Moved => {
                        app.on_move(id, x, y);
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        app.on_release(id);
                        #[cfg(windows)]
                        {
                            let still_touching = app.input.has_active();
                            if !still_touching {
                                ACTIVE_INTERACTION.with(|cell| cell.set(false));
                            }
                        }
                    }
                }
                window.request_redraw();
            }

            Event::WindowEvent { event: WindowEvent::Resized(s), .. } => {
                app.resize(s.width, s.height);
                #[cfg(windows)]
                update_hit_test_data(app.layout.controls.clone(), app.scale);
                window.request_redraw();
            }

            Event::AboutToWait => {
                let active = app.tick();
                if active {
                    elwt.set_control_flow(ControlFlow::Poll);
                    window.request_redraw();
                } else {
                    elwt.set_control_flow(ControlFlow::Wait);
                }
            }

            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                let size = window.inner_size();
                if size.width == 0 || size.height == 0 { return; }
                let w = NonZeroU32::new(size.width).unwrap();
                let h = NonZeroU32::new(size.height).unwrap();
                surface.resize(w, h).unwrap();
                let mut buffer = surface.buffer_mut().unwrap();
                app.render(&mut buffer);
                buffer.present().unwrap();
            }
            _ => {}
        }
    }).unwrap();
}
