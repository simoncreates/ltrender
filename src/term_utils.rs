use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use std::io::{Result, stdout};

#[macro_export]
macro_rules! init_terminal {
    () => {
        $crate::initial_terminal_state().unwrap();
        use $crate::restore_terminal;
        ctrlc::set_handler(|| {
            let _ = restore_terminal();
            std::process::exit(130);
        })
        .expect("Error setting Ctrl-C handler");

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = restore_terminal();
            prev_hook(panic_info);
        }));
    };
}

pub fn initial_terminal_state() -> Result<()> {
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All), Hide)?;
    execute!(stdout, EnableMouseCapture)?;
    Ok(())
}

pub fn restore_terminal() -> Result<()> {
    println!("restoring terminal");
    let mut stdout = stdout();
    execute!(
        stdout,
        Clear(ClearType::All),
        Show,
        MoveTo(0, 0),
        LeaveAlternateScreen
    )?;
    execute!(stdout, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}
