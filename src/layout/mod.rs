use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GamepadInput {
    ButtonA, ButtonB, ButtonX, ButtonY,
    BumperLeft, BumperRight,
    TriggerLeft, TriggerRight,
    DpadUp, DpadDown, DpadLeft, DpadRight,
    Start, Select, XboxButton,
    StickLeft, StickRight,
    StickLeftAxis, StickRightAxis,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    Circle      { radius: f32 },
    RoundedRect { width: f32, height: f32, radius: f32 },
    DpadCross   { arm_width: f32, arm_length: f32 },
    Ring        { outer_radius: f32, inner_radius: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Control {
    pub id:    u32,
    pub label: String,
    pub x:     f32,
    pub y:     f32,
    pub shape: Shape,
    pub input: GamepadInput,
    pub color: [u8; 4],
}

impl Control {
    pub fn new(id: u32, label: impl Into<String>, x: f32, y: f32,
               shape: Shape, input: GamepadInput, color: [u8; 4]) -> Self {
        Self { id, label: label.into(), x, y, shape, input, color }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        match &self.shape {
            Shape::Circle { radius } => {
                let dx = px - self.x;
                let dy = py - self.y;
                (dx * dx + dy * dy).sqrt() <= *radius
            }
            Shape::RoundedRect { width, height, .. } => {
                (px - self.x).abs() <= width  / 2.0
                    && (py - self.y).abs() <= height / 2.0
            }
            Shape::DpadCross { arm_width, arm_length } => {
                let in_h = (px - self.x).abs() <= *arm_length
                    && (py - self.y).abs() <= *arm_width;
                let in_v = (py - self.y).abs() <= *arm_length
                    && (px - self.x).abs() <= *arm_width;
                in_h || in_v
            }
            Shape::Ring { outer_radius, inner_radius } => {
                let d = ((px - self.x).powi(2) + (py - self.y).powi(2)).sqrt();
                d <= *outer_radius && d >= *inner_radius
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub controls: Vec<Control>,
}

impl Layout {
    pub fn default_xbox() -> Self {
        let mut c = Vec::new();
        let mut id = 0u32;
        let mut n = || { id += 1; id };

        // Layout designed for 1280x720
        // Padded inward so nothing clips at screen edges
        // Left edge starts at x=80, right edge ends at x=1200
        // Top row y=48, bottom sticks y=620

        // ── LT / LB (top left) ───────────────────────────────────
        c.push(Control::new(n(), "LT", 110.0, 46.0,
            Shape::RoundedRect { width: 88.0, height: 40.0, radius: 8.0 },
            GamepadInput::TriggerLeft,  [31, 41, 55, 235]));
        c.push(Control::new(n(), "LB", 110.0, 96.0,
            Shape::RoundedRect { width: 108.0, height: 36.0, radius: 8.0 },
            GamepadInput::BumperLeft,   [31, 41, 55, 235]));

        // ── RT / RB (top right) ──────────────────────────────────
        c.push(Control::new(n(), "RT", 1170.0, 46.0,
            Shape::RoundedRect { width: 88.0, height: 40.0, radius: 8.0 },
            GamepadInput::TriggerRight, [31, 41, 55, 235]));
        c.push(Control::new(n(), "RB", 1170.0, 96.0,
            Shape::RoundedRect { width: 108.0, height: 36.0, radius: 8.0 },
            GamepadInput::BumperRight,  [31, 41, 55, 235]));

        // ── Center buttons ────────────────────────────────────────
        c.push(Control::new(n(), "sel", 560.0, 52.0,
            Shape::Circle { radius: 20.0 },
            GamepadInput::Select,       [55, 65, 81, 235]));
        c.push(Control::new(n(), "xbox", 640.0, 48.0,
            Shape::Circle { radius: 24.0 },
            GamepadInput::XboxButton,   [22, 163, 74, 235]));
        c.push(Control::new(n(), "men", 720.0, 52.0,
            Shape::Circle { radius: 20.0 },
            GamepadInput::Start,        [55, 65, 81, 235]));

        // ── D-Pad (left middle) ───────────────────────────────────
        // arm_width = half thickness, arm_length = half total span
        c.push(Control::new(n(), "", 170.0, 370.0,
            Shape::DpadCross { arm_width: 22.0, arm_length: 55.0 },
            GamepadInput::DpadUp,       [42, 52, 71, 235]));

        // ── ABXY (right middle) ───────────────────────────────────
        c.push(Control::new(n(), "Y", 1100.0, 320.0,
            Shape::Circle { radius: 24.0 },
            GamepadInput::ButtonY,      [202, 138,   4, 235]));
        c.push(Control::new(n(), "X", 1055.0, 365.0,
            Shape::Circle { radius: 24.0 },
            GamepadInput::ButtonX,      [ 14, 165, 233, 235]));
        c.push(Control::new(n(), "B", 1145.0, 365.0,
            Shape::Circle { radius: 24.0 },
            GamepadInput::ButtonB,      [220,  38,  38, 235]));
        c.push(Control::new(n(), "A", 1100.0, 410.0,
            Shape::Circle { radius: 24.0 },
            GamepadInput::ButtonA,      [ 22, 163,  74, 235]));

        // ── Left thumbstick (bottom left) ─────────────────────────
        // outer=55 knob=26 dimple=5.5
        c.push(Control::new(n(), "", 185.0, 590.0,
            Shape::Ring { outer_radius: 55.0, inner_radius: 26.0 },
            GamepadInput::StickLeftAxis,  [31, 41, 55, 220]));
        c.push(Control::new(n(), "L3", 185.0, 590.0,
            Shape::Circle { radius: 26.0 },
            GamepadInput::StickLeft,      [156, 163, 175, 235]));

        // ── Right thumbstick (bottom right) ───────────────────────
        c.push(Control::new(n(), "", 1095.0, 590.0,
            Shape::Ring { outer_radius: 55.0, inner_radius: 26.0 },
            GamepadInput::StickRightAxis, [31, 41, 55, 220]));
        c.push(Control::new(n(), "R3", 1095.0, 590.0,
            Shape::Circle { radius: 26.0 },
            GamepadInput::StickRight,     [156, 163, 175, 235]));

        Self { name: "Xbox Layout".into(), controls: c }
    }
}