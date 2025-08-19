#[macro_export]
macro_rules! spawn_cell_drawer {
    ($name:ident, $out:expr, $buffersize:expr) => {{
        use std::sync::mpsc;
        use std::sync::mpsc::{Receiver, Sender};
        use std::thread;

        // create the channel
        let (tx, rx) = mpsc::sync_channel::<CellDrawerCommand>($buffersize);

        // evaluate the expression here so it's moved into the thread below
        let out = $out;

        // spawn a thread that owns the receiver and the writer
        let handle = thread::spawn(move || {
            let mut drawer = $name { rx, out };

            // process commands until the sender(s) are dropped
            while let Ok(cmd) = drawer.rx.recv() {
                match cmd {
                    CellDrawerCommand::SetString(batch) => drawer.set_string(batch),
                    CellDrawerCommand::Flush => {
                        if let Err(e) = drawer.flush() {
                            log::error!("Flush error in drawer thread: {}", e);
                        }
                    }
                }
            }

            // best-effort final flush
            let _ = drawer.flush();
        });

        (tx, handle)
    }};
}
/// ## $name:
/// The name of the default drawbuffer
/// ## $drawer:
/// The name of the cell drawer
/// ## $buffersize:
/// the amount of unhandled draw commands, that can be buffered before blocking
///
///
///
/// ### info:
/// When using this macro, you are required to create a struct
/// that implements the `CellDrawer` trait and at minimum looks like this:
/// ```
///#[derive(Debug)]
/// pub struct $drawer {
///     rx: Receiver<CellDrawerCommand>,
///     out: BufWriter<Stdout>,
/// }
/// ```
#[macro_export]
macro_rules! create_drawbuffer {
    ($name:ident, $drawer:ident, $buffersize:expr) => {
        use crate::spawn_cell_drawer;
        #[derive(Debug)]
        pub struct $name {
            cells: Vec<CharacterInfoList>,
            intervals: UpdateIntervalHandler,
            size: (u16, u16),

            drawer_tx: std::sync::mpsc::SyncSender<CellDrawerCommand>,

            // todo: implement drop handler
            drawer_handle: std::thread::JoinHandle<()>,
        }

        impl ScreenBufferCore for $name {
            fn cell_info_mut(&mut self) -> &mut Vec<CharacterInfoList> {
                &mut self.cells
            }

            fn intervals_mut(&mut self) -> &mut UpdateIntervalHandler {
                &mut self.intervals
            }

            fn size(&self) -> (u16, u16) {
                self.size
            }
        }
        impl ScreenBuffer for $name {
            fn new(size: (u16, u16)) -> Self {
                let capacity = size.0 as usize * size.1 as usize;
                let out = BufWriter::new(stdout());
                let (drawer_tx, drawer_handle) = spawn_cell_drawer!($drawer, out, $buffersize);

                $name {
                    cells: vec![
                        CharacterInfoList {
                            info: HashMap::new()
                        };
                        capacity
                    ],
                    intervals: UpdateIntervalHandler::new(size.0, size.1),
                    size,
                    drawer_tx,
                    drawer_handle,
                }
            }
            fn drawer_sender(&self) -> std::sync::mpsc::SyncSender<CellDrawerCommand> {
                self.drawer_tx.clone()
            }
        }
    };
}
