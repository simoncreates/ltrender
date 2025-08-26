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

pub fn init_terminal() -> Result<()> {
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All), Hide)?;
    execute!(stdout, EnableMouseCapture)?;
    Ok(())
}

pub fn restore_terminal() -> Result<()> {
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
