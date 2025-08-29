

/// is used to make the new function for a struct implementing the ScreenBuffer trait
///
/// ## $cell_drawer_name
/// name of the celldrawer, which will be run in the receiving loop
#[macro_export]
macro_rules! ScreenBufferNew {
    ($name:ident,  $cell_drawer_name:ident  ) => {
        fn new(size: (u16, u16)) -> Self {
            let capacity = size.0 as usize * size.1 as usize;

            use std::sync::mpsc;
            use std::thread;

            // TODO: stop using a const channel_bound for screen buffer
            // let users define
            let (drawer_tx, rx) = mpsc::sync_channel::<CellDrawerCommand>(100);

            // spawn a thread that owns the receiver and the writer
            let drawer_handle = thread::spawn(move || {
                let mut drawer = $cell_drawer_name::init(rx);

                // process commands until the sender is dropped
                while let Ok(cmd) = drawer.rx.recv() {
                    match cmd {
                        CellDrawerCommand::SetString(batch, size) => drawer.set_string(batch, size),
                        CellDrawerCommand::Flush => {
                            let _ = drawer.flush();
                        }
                    }
                }
            });

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
    };
}

#[macro_export]
macro_rules! define_screen_buffer {
    (
        $name:ident,          
        $cell_drawer_name:ident            
    ) => {
        #[derive(Debug)]
        pub struct $name {
            cells: Vec<CharacterInfoList>,
            intervals: UpdateIntervalHandler,
            size: (u16, u16),
            drawer_tx: std::sync::mpsc::SyncSender<CellDrawerCommand>,

            /// todo: implement joining
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
            ScreenBufferNew!($name, $cell_drawer_name);

            fn drawer_sender(&self) -> std::sync::mpsc::SyncSender<CellDrawerCommand> {
                self.drawer_tx.clone()
            }
        }
    };
}
