use crate::input::InputTracker;
use crate::layout::{GamepadInput, Layout};
use crate::render::Renderer;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const PRESS_ANIM_SPEED: f32  = 14.0;
const RELEASE_ANIM_SPEED: f32 = 9.0;
const BASE_WIDTH: f32  = 1280.0;
const BASE_HEIGHT: f32 = 720.0;

pub struct App {
    pub layout:   Layout,
    pub input:    InputTracker,
    pub renderer: Option<Renderer>,
    anim:         HashMap<u32, f32>,
    last_tick:    Instant,
    pub scale:    f32,
    /// Stick offset per stick control_id: (dx, dy) normalised -1..1
    pub stick_offsets: HashMap<u32, (f32, f32)>,
}

impl App {
    pub fn new() -> Self {
        Self {
            layout:        Layout::default_xbox(),
            input:         InputTracker::new(),
            renderer:      None,
            anim:          HashMap::new(),
            last_tick:     Instant::now(),
            scale:         1.0,
            stick_offsets: HashMap::new(),
        }
    }

    fn compute_scale(w: u32, h: u32) -> f32 {
        (w as f32 / BASE_WIDTH).min(h as f32 / BASE_HEIGHT)
    }

    pub fn init_renderer(&mut self, w: u32, h: u32) {
        self.scale    = Self::compute_scale(w, h);
        self.renderer = Renderer::new(w, h);
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.scale = Self::compute_scale(w, h);
        if let Some(r) = &mut self.renderer { r.resize(w, h); }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let dt  = now.duration_since(self.last_tick).as_secs_f32().min(0.05);
        self.last_tick = now;
        let pressed: HashSet<u32> = self.input.pressed_ids().copied().collect();
        let mut active = false;

        for control in &self.layout.controls {
            let prog = self.anim.entry(control.id).or_insert(0.0);
            let target = if pressed.contains(&control.id) { 1.0 } else { 0.0 };
            if (*prog - target).abs() > 0.001 {
                if pressed.contains(&control.id) {
                    *prog = (*prog + dt * PRESS_ANIM_SPEED).min(1.0);
                } else {
                    *prog = (*prog - dt * RELEASE_ANIM_SPEED).max(0.0);
                }
                active = true;
            } else {
                *prog = target;
            }
        }

        // Snap sticks back when not held
        for control in &self.layout.controls {
            if matches!(control.input,
                GamepadInput::StickLeft | GamepadInput::StickRight |
                GamepadInput::StickLeftAxis | GamepadInput::StickRightAxis) {
                let is_pressed = pressed.contains(&control.id);
                if is_pressed {
                    active = true;
                } else if let Some(off) = self.stick_offsets.get_mut(&control.id) {
                    if off.0 != 0.0 || off.1 != 0.0 {
                        off.0 *= 1.0 - dt * 12.0;
                        off.1 *= 1.0 - dt * 12.0;
                        if off.0.abs() < 0.01 { off.0 = 0.0; }
                        if off.1.abs() < 0.01 { off.1 = 0.0; }
                        active = true;
                    }
                }
            }
        }

        active
    }

    pub fn render(&mut self, buffer: &mut [u32]) {
        let scale = self.scale;
        let offsets = self.stick_offsets.clone();
        if let Some(renderer) = &mut self.renderer {
            renderer.clear();
            renderer.draw_controls(&self.layout.controls, &self.anim, scale, &offsets);
            renderer.to_argb_buffer(buffer);
        }
    }

    #[allow(dead_code)]
    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        let lx = x / self.scale;
        let ly = y / self.scale;
        self.layout.controls.iter().any(|c| c.contains(lx, ly))
    }

    #[allow(dead_code)]
    pub fn has_active_touches(&self) -> bool {
        self.input.has_active()
    }

    pub fn on_press(&mut self, touch_id: u64, x: f32, y: f32) {
        let lx = x / self.scale;
        let ly = y / self.scale;
        if let Some(ev) = self.input.press(touch_id, lx, ly, &self.layout.controls) {
            log::debug!("Press: {:?}", ev.input);
        }
    }

    pub fn on_move(&mut self, touch_id: u64, x: f32, y: f32) {
        let lx = x / self.scale;
        let ly = y / self.scale;
        // Check if this touch is on a stick
        if let Some(state) = self.input.get_touch_state(touch_id) {
            let control_id = state.control_id;
            // Find the control
            if let Some(control) = self.layout.controls.iter()
                .find(|c| c.id == control_id) {
                if matches!(control.input,
                    GamepadInput::StickLeft | GamepadInput::StickRight |
                    GamepadInput::StickLeftAxis | GamepadInput::StickRightAxis) {
                    // Compute offset from stick center
                    let dx = lx - control.x;
                    let dy = ly - control.y;
                    // Max travel in layout coords
                    let max_travel = match &control.shape {
                        crate::layout::Shape::Circle { radius } => radius * 0.6,
                        crate::layout::Shape::Ring { outer_radius, inner_radius } =>
                            outer_radius - inner_radius,
                        _ => 20.0,
                    };
                    let dist = (dx*dx + dy*dy).sqrt();
                    let capped = dist.min(max_travel);
                    let (nx, ny) = if dist > 0.0 {
                        (dx/dist * capped, dy/dist * capped)
                    } else {
                        (0.0, 0.0)
                    };
                    // Store as pixel offset (will be scaled in renderer)
                    self.stick_offsets.insert(control_id, (nx, ny));
                    // Also update the ring control
                    for c in &self.layout.controls {
                        if (c.x - control.x).abs() < 1.0 && (c.y - control.y).abs() < 1.0
                            && c.id != control_id {
                            self.stick_offsets.insert(c.id, (nx, ny));
                        }
                    }
                }
            }
        }
    }

    pub fn on_release(&mut self, touch_id: u64) {
        if let Some(ev) = self.input.release(touch_id, &self.layout.controls) {
            log::debug!("Release: {:?}", ev.input);
        }
    }
}

impl Default for App { fn default() -> Self { Self::new() } }
