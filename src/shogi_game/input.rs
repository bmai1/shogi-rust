use gilrs::{Axis, Button, Event, EventType};

use super::ShogiGame;

impl ShogiGame {
    pub(super) fn poll_gamepad(&mut self) -> bool {
        let mut confirm = false;

        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            if let EventType::ButtonPressed(Button::South, _) = event {
                confirm = true;
            }
        }

        let Some((_, gamepad)) = self.gilrs.gamepads().next() else {
            return confirm;
        };

        // D-pad
        // Increase mod val (12) to make polling slower
        if gamepad.is_pressed(Button::DPadUp) {
            self.gamepad_active = true;
            if self.dpad_repeat == 0 {
                self.gamepad_cursor[0] = (self.gamepad_cursor[0] - 1).rem_euclid(9);
            }
            self.dpad_repeat = (self.dpad_repeat + 1) % 24;
        } else if gamepad.is_pressed(Button::DPadDown) {
            self.gamepad_active = true;
            if self.dpad_repeat == 0 {
                self.gamepad_cursor[0] = (self.gamepad_cursor[0] + 1).rem_euclid(9);
            }
            self.dpad_repeat = (self.dpad_repeat + 1) % 24;
        } else if gamepad.is_pressed(Button::DPadLeft) {
            self.gamepad_active = true;
            if self.dpad_repeat == 0 {
                self.gamepad_cursor[1] = (self.gamepad_cursor[1] + 1).rem_euclid(9);
            }
            self.dpad_repeat = (self.dpad_repeat + 1) % 24;
        } else if gamepad.is_pressed(Button::DPadRight) {
            self.gamepad_active = true;
            if self.dpad_repeat == 0 {
                self.gamepad_cursor[1] = (self.gamepad_cursor[1] - 1).rem_euclid(9);
            }
            self.dpad_repeat = (self.dpad_repeat + 1) % 24;
        } else {
            self.dpad_repeat = 0;
        }

        // Left stick
        let x = gamepad.value(Axis::LeftStickX);
        let y = gamepad.value(Axis::LeftStickY);

        const DEADZONE: f32 = 0.4;

        if self.stick_repeat == 0 {
            if y < -DEADZONE {
                self.gamepad_active = true;
                self.gamepad_cursor[0] =
                    (self.gamepad_cursor[0] + 1).clamp(0, 8);
            }

            if y > DEADZONE {
                self.gamepad_active = true;
                self.gamepad_cursor[0] =
                    (self.gamepad_cursor[0] - 1).clamp(0, 8);
            }

            if x < -DEADZONE {
                self.gamepad_active = true;
                self.gamepad_cursor[1] =
                    (self.gamepad_cursor[1] + 1).clamp(0, 8);
            }

            if x > DEADZONE {
                self.gamepad_active = true;
                self.gamepad_cursor[1] =
                    (self.gamepad_cursor[1] - 1).clamp(0, 8);
            }
        }
        if x.abs() > DEADZONE || y.abs() > DEADZONE {
            self.stick_repeat = (self.stick_repeat + 1) % 8;
        } else {
            self.stick_repeat = 0;
        }
        confirm
    }
}