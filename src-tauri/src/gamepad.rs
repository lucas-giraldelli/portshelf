//! Controllers read natively (gilrs) and forwarded to the UI as "pad" events, since
//! the webview's Gamepad API is not always available (WebKitGTK builds without it).

use gilrs::{Axis, Button, EventType, Gilrs};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let Ok(mut gilrs) = Gilrs::new() else {
            return;
        };
        // Stick direction currently held, so a push counts once until it returns to centre.
        let (mut held_x, mut held_y) = (0i8, 0i8);
        loop {
            while let Some(event) = gilrs.next_event() {
                // Only when the shelf is the focused window: a game (or anything else) in front
                // gets the controller alone. Controllers are read system-wide, so without this
                // every open shelf would react.
                let focused = app
                    .get_webview_window("main")
                    .and_then(|w| Some(w.is_visible().ok()? && w.is_focused().ok()?))
                    .unwrap_or(false);
                if !focused {
                    continue;
                }
                let action = match event.event {
                    EventType::ButtonPressed(button, _) => match button {
                        Button::DPadLeft => Some("left"),
                        Button::DPadRight => Some("right"),
                        Button::DPadUp => Some("up"),
                        Button::DPadDown => Some("down"),
                        Button::South => Some("a"),
                        Button::East => Some("b"),
                        Button::North => Some("y"),
                        Button::West => Some("x"),
                        Button::Select => Some("select"),
                        Button::Start => Some("start"),
                        Button::LeftTrigger => Some("lb"),
                        Button::RightTrigger => Some("rb"),
                        _ => None,
                    },
                    EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                        let dir = if value > 0.6 { 1 } else if value < -0.6 { -1 } else if value.abs() < 0.3 { 0 } else { held_x };
                        let fire = dir != 0 && dir != held_x;
                        held_x = dir;
                        fire.then_some(if dir > 0 { "right" } else { "left" })
                    }
                    EventType::AxisChanged(Axis::LeftStickY, value, _) => {
                        // gilrs: up is positive.
                        let dir = if value > 0.6 { 1 } else if value < -0.6 { -1 } else if value.abs() < 0.3 { 0 } else { held_y };
                        let fire = dir != 0 && dir != held_y;
                        held_y = dir;
                        fire.then_some(if dir > 0 { "up" } else { "down" })
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    let _ = app.emit("pad", action);
                }
            }
            std::thread::sleep(Duration::from_millis(8));
        }
    });
}
