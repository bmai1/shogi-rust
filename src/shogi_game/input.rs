use gilrs::{Event, EventType, Button};

use super::ShogiGame;

impl ShogiGame {
    // Drains pending gamepad events, moves the on-screen cursor with the D-pad,
    // and returns true for exactly one frame when a "confirm" button was pressed.
    pub(super) fn poll_gamepad(&mut self) -> bool {
        let mut confirm = false;
        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::ButtonPressed(Button::DPadUp, _) => {
                    self.gamepad_cursor[0] = (self.gamepad_cursor[0] - 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::DPadDown, _) => {
                    self.gamepad_cursor[0] = (self.gamepad_cursor[0] + 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::DPadLeft, _) => {
                    self.gamepad_cursor[1] = (self.gamepad_cursor[1] + 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::DPadRight, _) => {
                    self.gamepad_cursor[1] = (self.gamepad_cursor[1] - 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::South, _) => {
                    confirm = true;
                }
                _ => {}
            }
        }
        confirm
    }
}