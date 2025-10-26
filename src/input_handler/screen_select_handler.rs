use crate::input_handler::manager::{SubscriptionMessage, TargetScreen};
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

pub type ScreenSelectcallback = Box<dyn FnMut(SubscriptionMessage) -> Option<TargetScreen> + Send>;

#[derive(Debug)]
pub struct ScreenSelectHandler {
    recv: Arc<Mutex<Receiver<SubscriptionMessage>>>,
    send: Arc<Sender<Option<TargetScreen>>>,
    handle: Option<JoinHandle<()>>,
}

impl ScreenSelectHandler {
    pub fn new(recv: Receiver<SubscriptionMessage>, send: Sender<Option<TargetScreen>>) -> Self {
        Self {
            recv: Arc::new(Mutex::new(recv)),
            send: Arc::new(send),
            handle: None,
        }
    }

    pub fn recv(&self) -> Result<SubscriptionMessage, mpsc::RecvError> {
        let recv = self.recv.lock().unwrap();
        recv.recv()
    }

    pub fn try_recv(&self) -> Result<SubscriptionMessage, mpsc::TryRecvError> {
        let recv = self.recv.lock().unwrap();
        recv.try_recv()
    }

    pub fn send(
        &self,
        target_screen: Option<TargetScreen>,
    ) -> Result<(), mpsc::SendError<Option<TargetScreen>>> {
        self.send.send(target_screen)
    }

    pub fn run(
        &mut self,
        callback: impl FnMut(SubscriptionMessage) -> Option<TargetScreen> + Send + 'static,
    ) {
        let recv = Arc::clone(&self.recv);
        let send = Arc::clone(&self.send);

        let handle = thread::spawn(move || {
            let mut callback = callback;
            loop {
                let msg_res = {
                    let guard = recv.lock().expect("recv mutex poisoned");
                    guard.recv()
                };

                match msg_res {
                    Ok(msg) => {
                        let result = { callback(msg) };

                        let _ = send.send(result);
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });
        self.handle = Some(handle);
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
