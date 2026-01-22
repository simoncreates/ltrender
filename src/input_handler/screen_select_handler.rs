use crate::input_handler::manager::{SubscriptionMessage, TargetScreen};
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
};

pub type ScreenSelectcallback = Box<dyn FnMut(ScreenSelectHMsg) -> Option<TargetScreen> + Send>;

#[derive(Debug)]
pub enum ScreenSelectHMsg {
    Event(SubscriptionMessage),
    /// if a hook sends a forced screen selection message to
    /// the Manager, it will first have to be "approved" by the ScreenSelectHandler
    Selection(TargetScreen),
}

#[derive(Debug)]
pub struct ScreenSelectHandler {
    recv: Arc<Mutex<Receiver<ScreenSelectHMsg>>>,
    send: Arc<Sender<Option<TargetScreen>>>,
    handle: Option<JoinHandle<()>>,
}

impl ScreenSelectHandler {
    pub fn new(recv: Receiver<ScreenSelectHMsg>, send: Sender<Option<TargetScreen>>) -> Self {
        Self {
            recv: Arc::new(Mutex::new(recv)),
            send: Arc::new(send),
            handle: None,
        }
    }

    pub fn recv(&self) -> Result<ScreenSelectHMsg, mpsc::RecvError> {
        let recv = self.recv.lock().unwrap();
        recv.recv()
    }

    pub fn try_recv(&self) -> Result<ScreenSelectHMsg, mpsc::TryRecvError> {
        let recv = self.recv.lock().unwrap();
        recv.try_recv()
    }

    pub fn send(
        &self,
        target_screen: Option<TargetScreen>,
    ) -> Result<(), mpsc::SendError<Option<TargetScreen>>> {
        self.send.send(target_screen)
    }

    pub fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ScreenSelectHandler {
    fn drop(&mut self) {
        self.join();
    }
}
