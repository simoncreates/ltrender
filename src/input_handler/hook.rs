use crossterm::event::{KeyCode, MouseButton};
use log::info;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self},
    time::Duration,
};

use crossbeam_channel::{Receiver as CbReceiver, TryRecvError, unbounded};

use crate::{
    error::EventCommunicationError,
    input_handler::manager::{
        EventManagerCommand, EventManagerState, KeyAction, KeySubscriptionTypes, MouseAction,
        MouseButtonState, MouseButtons, MouseSubscriptionTypes, SubscriptionID,
        SubscriptionMessage, SubscriptionType, TargetScreen,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputButton {
    Mouse(MouseButton),
    Key(KeyCode),
}

type Callback = Arc<Mutex<Box<dyn FnMut(SubscriptionMessage) + Send>>>;
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
            let mut idle_sleep = Duration::from_millis(2);
            let max_sleep = Duration::from_millis(50);

            loop {
                let snapshot: Vec<(SubscriptionID, CbReceiver<SubscriptionMessage>)> = {
                    let guard = receivers.lock().unwrap();
                    guard.iter().map(|(id, r)| (*id, r.clone())).collect()
                };

                if snapshot.is_empty() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                let mut handled_any = false;

                for (sub_id, recv) in snapshot.iter() {
                    match recv.try_recv() {
                        Ok(msg) => {
                            handled_any = true;

                            if let SubscriptionMessage::SubscriptionId(_) = msg {
                                continue;
                            }
                            let cb_arc_opt: Option<Callback> = {
                                let cbs_guard = callbacks.lock().unwrap();
                                cbs_guard.get(sub_id).cloned()
                            };

                            if let Some(cb_arc) = cb_arc_opt {
                                match cb_arc.lock() {
                                    Ok(mut cb) => {
                                        (cb)(msg);
                                    }
                                    Err(poisoned) => {
                                        log::warn!(
                                            "callback mutex poisoned for subscription {:?} with error: {:?}",
                                            sub_id,
                                            poisoned
                                        );
                                    }
                                }
                            }
                        }

                        Err(TryRecvError::Empty) => {}

                        Err(TryRecvError::Disconnected) => {
                            {
                                let mut guard = receivers.lock().unwrap();
                                if let Some(pos) = guard.iter().position(|(id, _)| id == sub_id) {
                                    guard.remove(pos);
                                }
                            }
                            {
                                let mut cbs = callbacks.lock().unwrap();
                                cbs.remove(sub_id);
                            }
                        }
                    }
                }

                if handled_any {
                    idle_sleep = Duration::from_millis(2);
                } else {
                    thread::sleep(idle_sleep);
                    idle_sleep = std::cmp::min(max_sleep, idle_sleep * 2);
                }
            }
        }));
    }

    fn communicate_subscription(
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
        let sub_type =
            SubscriptionType::Key(KeySubscriptionTypes::Specific(key), KeyAction::Pressed);
        self.subscribe(sub_type, callback)
    }

    pub fn on_mouse_button_press<F>(
        &mut self,
        msbutton: MouseButtons,
        callback: F,
    ) -> Result<(), EventCommunicationError>
    where
        F: FnMut(SubscriptionMessage) + Send + 'static,
    {
        let sub_type = SubscriptionType::Mouse(MouseSubscriptionTypes::ButtonAction(
            msbutton,
            MouseAction::Pressed,
        ));
        self.subscribe(sub_type, callback)
    }

    pub fn subscribe<F>(
        &mut self,
        sub_type: SubscriptionType,
        callback: F,
    ) -> Result<(), EventCommunicationError>
    where
        F: FnMut(SubscriptionMessage) + Send + 'static,
    {
        let (recv, id) = self.communicate_subscription(sub_type)?;
        {
            let cb_arc: Callback = Arc::new(Mutex::new(
                Box::new(callback) as Box<dyn FnMut(SubscriptionMessage) + Send>
            ));
            self.callbacks.lock().unwrap().insert(id, cb_arc);
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
        if !st.pressed_keys.is_empty() {
            info!("checking is pressed with: {:?}", st.pressed_keys)
        }
        let result = match button {
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
        };
        info!("is_pressed({:?}) -> {}", button, result);
        result
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
