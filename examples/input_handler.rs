use std::{process::exit, thread};

use crossterm::event::{KeyCode, MouseButton};
use log::info;
use ltrender::{
    CrosstermEventManager, init_logger, init_terminal,
    input_handler::{
        hook::InputButton,
        manager::{MouseButtons, MouseMessage, SubscriptionMessage, TargetScreen},
    },
    restore_terminal,
};

fn main() {
    init_terminal!();
    init_logger("./input_handler_test.log").unwrap();
    let (_input_manager, input_handler, screen_select_h) =
        CrosstermEventManager::new_with_select_sub(TargetScreen::None);
    let handle = thread::spawn(move || {
        macro_rules! try_send {
            ($message: expr) => {
                if let Err(e) = screen_select_h.send($message) {
                    info!("error sending select {:?}", e);
                    break;
                } else {
                };
            };
        }
        loop {
            if let Ok(sm) = screen_select_h.recv()
                && let SubscriptionMessage::Mouse {
                    msg,
                    screen: _screen,
                } = sm
                && let MouseMessage::Pressed(_) = msg
            {
                try_send!(Some(TargetScreen::Screen(1)));
            } else {
                try_send!(None);
            };
        }
    });
    let mut hook = input_handler.create_hook();

    hook.on_key_press(KeyCode::Char('q'), |_| exit(0)).unwrap();
    hook.on_key_press(KeyCode::Char('n'), |_| eprintln!("n"))
        .unwrap();

    hook.on_mouse_button_press(MouseButtons::Left, |m| eprintln!("{:?}", m))
        .unwrap();

    while !hook.is_pressed(InputButton::Mouse(MouseButton::Right)) {}
    while hook.is_pressed(InputButton::Mouse(MouseButton::Right)) {}
    handle.join().unwrap();
    restore_terminal().unwrap();
}
