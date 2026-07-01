use crate::layout::{Control, GamepadInput};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum PressState { Pressed, Released }

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InputEvent {
    pub control_id: u32,
    pub input: GamepadInput,
    pub state: PressState,
}

#[derive(Debug, Clone)]
pub struct StickState {
    pub control_id: u32,
    pub origin_x:   f32,
    pub origin_y:   f32,
}

pub struct InputTracker {
    /// touch_id -> control_id
    active: HashMap<u64, u32>,
    /// touch_id -> StickState (only for stick controls)
    stick_touches: HashMap<u64, StickState>,
}

impl InputTracker {
    pub fn new() -> Self {
        Self {
            active:        HashMap::new(),
            stick_touches: HashMap::new(),
        }
    }

    pub fn press(&mut self, touch_id: u64, x: f32, y: f32,
                 controls: &[Control]) -> Option<InputEvent> {
        for control in controls.iter().rev() {
            if control.contains(x, y) {
                self.active.insert(touch_id, control.id);
                // Track stick origin
                if matches!(control.input,
                    GamepadInput::StickLeft | GamepadInput::StickRight |
                    GamepadInput::StickLeftAxis | GamepadInput::StickRightAxis) {
                    self.stick_touches.insert(touch_id, StickState {
                        control_id: control.id,
                        origin_x: x,
                        origin_y: y,
                    });
                }
                return Some(InputEvent {
                    control_id: control.id,
                    input: control.input.clone(),
                    state: PressState::Pressed,
                });
            }
        }
        None
    }

    pub fn release(&mut self, touch_id: u64,
                   controls: &[Control]) -> Option<InputEvent> {
        self.stick_touches.remove(&touch_id);
        if let Some(control_id) = self.active.remove(&touch_id) {
            if let Some(control) = controls.iter().find(|c| c.id == control_id) {
                return Some(InputEvent {
                    control_id: control.id,
                    input: control.input.clone(),
                    state: PressState::Released,
                });
            }
        }
        None
    }

    pub fn get_touch_state(&self, touch_id: u64) -> Option<&StickState> {
        self.stick_touches.get(&touch_id)
    }

    pub fn pressed_ids(&self) -> impl Iterator<Item = &u32> {
        self.active.values()
    }

    pub fn has_active(&self) -> bool {
        !self.active.is_empty()
    }
}

impl Default for InputTracker {
    fn default() -> Self { Self::new() }
}
