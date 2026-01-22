use std::{collections::HashMap, marker::PhantomData, thread::JoinHandle};

use crate::{
    ScreenBuffer, UpdateIntervalHandler,
    terminal_buffer::{
        CellDrawer, CharacterInfoList, ScreenBufferCore,
        buffer_and_celldrawer::screen_buffer::CellDrawerCommand,
    },
};

#[derive(Debug)]
pub struct DefaultScreenBuffer<CD: CellDrawer + Send + 'static> {
    cells: Vec<CharacterInfoList>,
    intervals: UpdateIntervalHandler,
    size: (u16, u16),
    drawer_tx: std::sync::mpsc::SyncSender<CellDrawerCommand>,

    /// todo: implement joining
    drawer_handle: Option<JoinHandle<()>>,
    _phantom: std::marker::PhantomData<CD>,
}

impl<CD: CellDrawer + Send + 'static> ScreenBufferCore for DefaultScreenBuffer<CD> {
    fn cell_info_mut(&mut self) -> &mut Vec<CharacterInfoList> {
        &mut self.cells
    }
    fn cell_info(&self) -> &Vec<CharacterInfoList> {
        &self.cells
    }

    fn intervals_mut(&mut self) -> &mut UpdateIntervalHandler {
        &mut self.intervals
    }

    fn size(&self) -> (u16, u16) {
        self.size
    }
}

impl<CD> ScreenBuffer for DefaultScreenBuffer<CD>
where
    CD: CellDrawer + Send + 'static,
{
    type Drawer = CD;
    fn new(size: (u16, u16)) -> Self {
        let capacity = size.0 as usize * size.1 as usize;

        use std::sync::mpsc;
        use std::thread;

        // TODO: stop using a const channel_bound for screen buffer
        // let users define
        let (drawer_tx, rx) = mpsc::sync_channel::<CellDrawerCommand>(10000);

        // spawn a thread that owns the receiver and the writer
        let drawer_handle = thread::spawn(move || {
            let mut drawer = CD::init(rx);

            // process commands until the sender is dropped
            while let Ok(cmd) = drawer.recv() {
                match cmd {
                    CellDrawerCommand::SetString(batch, size) => drawer.set_string(batch, size),
                    CellDrawerCommand::Flush => {
                        let _ = drawer.flush();
                    }
                    CellDrawerCommand::Stop => {
                        break;
                    }
                }
            }
        });

        DefaultScreenBuffer {
            cells: vec![
                CharacterInfoList {
                    info: HashMap::new()
                };
                capacity
            ],
            intervals: UpdateIntervalHandler::new(size.0, size.1),
            size,
            drawer_tx,
            drawer_handle: Some(drawer_handle),
            _phantom: PhantomData,
        }
    }

    fn drawer_sender(&self) -> std::sync::mpsc::SyncSender<CellDrawerCommand> {
        self.drawer_tx.clone()
    }
    fn drop(&mut self) {
        let _ = self.drawer_tx.send(CellDrawerCommand::Stop);
        if let Some(handle) = self.drawer_handle.take() {
            let _ = handle.join();
        }
    }
}
