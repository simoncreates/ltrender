use std::process::exit;

use crossterm::event::KeyCode;
use ltrender::{
    CrosstermEventManager, init_logger,
    input_handler::{hook::InputButton, manager::TargetScreen},
};

fn main() {
    init_logger("./input_handler_test.log").unwrap();
    let (_input_manager, input_handler) = CrosstermEventManager::new(TargetScreen::None);
    let mut hook = input_handler.create_hook();

    hook.on_key_press(KeyCode::Char('n'), |_| eprintln!("nibber"))
        .unwrap();

    hook.on_key_press(KeyCode::Char('q'), |_| exit(0)).unwrap();

    while !hook.is_pressed(InputButton::Key(KeyCode::Char('f'))) {}
    while hook.is_pressed(InputButton::Key(KeyCode::Char('f'))) {}
}
