use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use crate::ScreenKey;

/// used to identify, on what screen a keypress was detected
/// the screen is optional, because sometimes it cant be identified, what screen a keypress was done on
#[derive(PartialEq)]
pub struct ScreenKeyEvent {
    event: KeyEvent,
    screen: Option<ScreenKey>,
}

// TODO: base the shifted char function around os or keyboard layout specific modifiers
pub fn key_event_to_char(event: KeyEvent) -> Option<char> {
    if !event.is_press() {
        return None;
    }

    match event.code {
        KeyCode::Char(c) => {
            if event.modifiers.contains(KeyModifiers::SHIFT) {
                Some(shifted_char(c))
            } else {
                Some(c)
            }
        }
        _ => None,
    }
}

fn shifted_char(c: char) -> char {
    match c {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        '`' => '~',
        c => c.to_ascii_uppercase(),
    }
}

pub struct InputHandler {
    last_keyevents: HashMap<KeyCode, ScreenKeyEvent>,
    sender: Option<Sender<Vec<ScreenKeyEvent>>>,
    global_keypresses: bool,
    targeted_screen: Option<ScreenKey>,
}
impl InputHandler {
    pub fn new(global_keypresses: bool) -> Self {
        InputHandler {
            last_keyevents: HashMap::new(),
            sender: None,
            global_keypresses,
            targeted_screen: None,
        }
    }
    pub fn new_with_sender(global_keypresses: bool) -> (Self, Receiver<Vec<ScreenKeyEvent>>) {
        let (sender, receiver) = mpsc::channel();
        (
            InputHandler {
                last_keyevents: HashMap::new(),
                sender: Some(sender),
                global_keypresses,
                targeted_screen: None,
            },
            receiver,
        )
    }
    pub fn detect_changes(&mut self) -> HashMap<KeyCode, ScreenKeyEvent> {
        let current_states = self.collect_current_states();
        let mut changed = HashMap::new();
        for (keycode, new_key_event) in current_states {
            if let Some(previous_event) = self.last_keyevents.get(&keycode) {
                if previous_event.event != new_key_event {
                    changed.insert(keycode, self.key_event_to_screen(new_key_event));
                }
            }
        }
        changed
    }

    fn key_event_to_screen(&self, event: KeyEvent) -> ScreenKeyEvent {
        if let Some(screen) = self.targeted_screen {
            ScreenKeyEvent {
                event,
                screen: Some(screen),
            }
        } else {
            ScreenKeyEvent {
                event,
                screen: None,
            }
        }
    }

    fn collect_current_states(&self) -> HashMap<KeyCode, KeyEvent> {
        let mut keypresses = HashMap::new();
        while event::poll(Duration::from_millis(0)).is_ok() {
            let event = if let Ok(event) = event::read() {
                event
            } else {
                continue;
            };
            if let Event::Key(key) = event {
                keypresses.insert(key.code, key);
            };
        }
        keypresses
    }
}
