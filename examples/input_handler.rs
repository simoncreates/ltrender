use std::process::exit;

use crossterm::event::{KeyCode, MouseButton};
use ltrender::{
    CrosstermEventManager, init_logger, init_terminal,
    input_handler::{hook::InputButton, manager::TargetScreen},
    restore_terminal,
};

fn main() {
    init_terminal!();
    init_logger("./input_handler_test.log").unwrap();
    let (_input_manager, input_handler) = CrosstermEventManager::new(TargetScreen::None);
    let mut hook = input_handler.create_hook();

    hook.on_key_press(KeyCode::Char('q'), |_| exit(0)).unwrap();
    hook.on_key_press(KeyCode::Char('n'), |_| eprintln!("n"))
        .unwrap();

    while !hook.is_pressed(InputButton::Mouse(MouseButton::Right)) {}
    while hook.is_pressed(InputButton::Mouse(MouseButton::Right)) {}
    restore_terminal().unwrap();
}
