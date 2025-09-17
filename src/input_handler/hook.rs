use crossterm::event::{KeyCode, MouseButton};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self},
};

use crossbeam_channel::{Receiver as CbReceiver, Select, unbounded};

use crate::{
    error::EventCommunicationError,
    input_handler::manager::{
        EventManagerCommand, EventManagerState, KeySubscriptionTypes, MouseButtonState,
        SubscriptionID, SubscriptionMessage, SubscriptionType, TargetScreen,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputButton {
    Mouse(MouseButton),
    Key(KeyCode),
}

type Callback = Box<dyn FnMut(SubscriptionMessage) + Send + 'static>;
type SubscriptionDate = (SubscriptionID, CbReceiver<SubscriptionMessage>);

pub struct EventHook {
    sender: std::sync::mpsc::Sender<EventManagerCommand>,
    input_manager_state: Arc<Mutex<EventManagerState>>,
    callbacks: Arc<Mutex<HashMap<SubscriptionID, Callback>>>,
    receivers: Arc<Mutex<Vec<SubscriptionDate>>>,
    dispatcher: Option<thread::JoinHandle<()>>,
}

impl EventHook {
    pub fn new(
        sender: std::sync::mpsc::Sender<EventManagerCommand>,
        input_manager_state: Arc<Mutex<EventManagerState>>,
    ) -> Self {
        EventHook {
            sender,
            input_manager_state,
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            receivers: Arc::new(Mutex::new(Vec::new())),
            dispatcher: None,
        }
    }

    fn ensure_dispatcher(&mut self) {
        if self.dispatcher.is_some() {
            return;
        }

        let callbacks = self.callbacks.clone();
        let receivers = self.receivers.clone();

        self.dispatcher = Some(thread::spawn(move || {
            loop {
                let snapshot: Vec<(SubscriptionID, CbReceiver<SubscriptionMessage>)> = {
                    let guard = receivers.lock().unwrap();
                    guard.iter().map(|(id, r)| (*id, r.clone())).collect()
                };

                if snapshot.is_empty() {
                    thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }

                let mut sel = Select::new();
                for (_id, r) in snapshot.iter() {
                    sel.recv(r);
                }

                let oper = sel.select();
                let idx = oper.index();
                let (sub_id, recv) = &snapshot[idx];

                match oper.recv(recv) {
                    Ok(msg) => {
                        if let SubscriptionMessage::SubscriptionId(_) = msg {
                            continue;
                        }

                        let mut cbs = callbacks.lock().unwrap();
                        if let Some(cb) = cbs.get_mut(sub_id) {
                            (cb)(msg);
                        }
                    }
                    Err(_) => {
                        let mut guard = receivers.lock().unwrap();
                        if let Some(pos) = guard.iter().position(|(id, _)| id == sub_id) {
                            guard.remove(pos);
                        }

                        let mut cbs = callbacks.lock().unwrap();
                        cbs.remove(sub_id);
                    }
                }
            }
        }));
    }

    fn subscribe(
        &self,
        sub_type: SubscriptionType,
    ) -> Result<(CbReceiver<SubscriptionMessage>, SubscriptionID), EventCommunicationError> {
        let (tx, rx) = unbounded::<SubscriptionMessage>();

        let message = EventManagerCommand::Subscribe(sub_type, tx.clone());
        self.sender.send(message.clone()).map_err(|_| {
            EventCommunicationError::FailedToSendEventManagerCommandMessage { message }
        })?;

        let timeout = std::time::Duration::from_millis(4000);
        match rx.recv_timeout(timeout) {
            Ok(SubscriptionMessage::SubscriptionId(id)) => {
                {
                    let mut guard = self.receivers.lock().unwrap();
                    guard.push((id, rx.clone()));
                }
                Ok((rx, id))
            }
            Ok(other) => Err(EventCommunicationError::ReceiveUnexpectedResponse {
                expected_type: SubscriptionMessage::SubscriptionId(0),
                received_type: other,
            }),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                Err(EventCommunicationError::DidNotReceiveIDResponse)
            }
            Err(_) => Err(EventCommunicationError::DidNotReceiveIDResponse),
        }
    }

    pub fn on_key_press<F>(
        &mut self,
        key: KeyCode,
        callback: F,
    ) -> Result<(), EventCommunicationError>
    where
        F: FnMut(SubscriptionMessage) + Send + 'static,
    {
        let sub_type = SubscriptionType::Key(KeySubscriptionTypes::Specific(key));
        let (recv, id) = self.subscribe(sub_type)?;
        {
            self.callbacks
                .lock()
                .unwrap()
                .insert(id, Box::new(callback));
        }
        {
            self.receivers.lock().unwrap().push((id, recv));
        }
        self.ensure_dispatcher();
        Ok(())
    }

    /// todo: check if works for mouse?
    pub fn is_pressed(&self, button: InputButton) -> bool {
        let st = self
            .input_manager_state
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        match button {
            InputButton::Key(key) => st.pressed_keys.contains_key(&key),
            InputButton::Mouse(btn) => match btn {
                MouseButton::Left => {
                    st.mouse_state.left_pressed == MouseButtonState::Pressed
                        || st.mouse_state.left_pressed == MouseButtonState::Dragging
                }
                MouseButton::Right => {
                    st.mouse_state.right_pressed == MouseButtonState::Pressed
                        || st.mouse_state.right_pressed == MouseButtonState::Dragging
                }
                MouseButton::Middle => {
                    st.mouse_state.middle_pressed == MouseButtonState::Pressed
                        || st.mouse_state.middle_pressed == MouseButtonState::Dragging
                }
            },
        }
    }

    pub fn is_pressed_with_screen(&self, button: InputButton) -> Option<TargetScreen> {
        let st = self
            .input_manager_state
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        match button {
            InputButton::Key(key) => st.pressed_keys.get(&key).map(|(_, screen)| *screen),
            InputButton::Mouse(btn) => {
                let pressed = match btn {
                    MouseButton::Left => {
                        st.mouse_state.left_pressed == MouseButtonState::Pressed
                            || st.mouse_state.left_pressed == MouseButtonState::Dragging
                    }
                    MouseButton::Right => {
                        st.mouse_state.right_pressed == MouseButtonState::Pressed
                            || st.mouse_state.right_pressed == MouseButtonState::Dragging
                    }
                    MouseButton::Middle => {
                        st.mouse_state.middle_pressed == MouseButtonState::Pressed
                            || st.mouse_state.middle_pressed == MouseButtonState::Dragging
                    }
                };
                if pressed {
                    Some(st.targeted_screen)
                } else {
                    None
                }
            }
        }
    }
}
