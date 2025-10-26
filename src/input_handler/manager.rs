use crate::{ScreenKey, input_handler::hook::EventHook};
use crossbeam_channel::Sender as CbSender;
use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseButton, MouseEventKind};
use std::{
    collections::HashMap,
    fmt::{self, Formatter},
    mem,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(feature = "screen_select_subscription")]
use crate::input_handler::screen_select_handler::ScreenSelectHandler;
use log::{info, warn};

//_____  all the enums, that will be sent to the subscribed hooks _____
// they desribe immidiate change

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum MouseMessage {
    Move(u16, u16),
    Pressed(MouseButton),
    Released(MouseButton),
    ScrollUp,
    ScrollDown,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum KeyMessage {
    Pressed(KeyCode),
    Released(KeyCode),
    Repeating(KeyCode),
}
#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum TerminalFocus {
    HasBeenFocused,
    HasBeenUnfocused,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SubscriptionMessage {
    Mouse {
        msg: MouseMessage,
        screen: TargetScreen,
    },
    Key {
        msg: KeyMessage,
        screen: TargetScreen,
    },
    /// new size
    Resize(u16, u16),
    /// the string is the content pasted
    Paste {
        content: Arc<String>,
        screen: TargetScreen,
    },
    TerminalWindowFocus(TerminalFocus),
    /// holds the id, of the subscription for optional removal later
    SubscriptionId(SubscriptionID),
}

impl fmt::Display for SubscriptionMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SubscriptionMessage::Mouse { msg, screen } => {
                write!(f, "Mouse event {:?} on screen {:?}", msg, screen)
            }
            SubscriptionMessage::Key { msg, screen } => {
                write!(f, "Key event {:?} on screen {:?}", msg, screen)
            }
            SubscriptionMessage::Resize(w, h) => write!(f, "Resize event: {}x{}", w, h),
            SubscriptionMessage::Paste { content, screen } => {
                write!(f, "Paste: {} to screen: {}", content, screen)
            }
            SubscriptionMessage::TerminalWindowFocus(focus) => {
                write!(f, "Terminal focus event: {:?}", focus)
            }
            SubscriptionMessage::SubscriptionId(id) => write!(f, "Subscription ID: {}", id),
        }
    }
}

// _____ all the enums, which will be sent, by the hooks _____

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum KeySubscriptionTypes {
    All,
    Specific(KeyCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Any,
    Pressed,
    Released,
    Repeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseAction {
    Any,
    Pressed,
    Released,
    //todo: add dragging?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseSubscriptionTypes {
    /// get every mouse-related subscription message
    All,
    /// all button events (pressed/released)
    Buttons,
    /// all scroll events (up/down)
    Scrolls,
    /// movement events
    Moves,
    /// any action for a specific button (left/right/middle)
    ButtonAny(MouseButtons),
    /// specific action for a specific button (e.g. Left + Pressed)
    ButtonAction(MouseButtons, MouseAction),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum SubscriptionType {
    Mouse(MouseSubscriptionTypes),
    Key(KeySubscriptionTypes, KeyAction),
    Resize,
    Paste,
    TerminalWindowFocus,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum MouseButtons {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone)]
pub enum EventManagerCommand {
    Subscribe(SubscriptionType, CbSender<SubscriptionMessage>),
    // todo: add unsubscribing (canceling subscription?)
    // Unsubscribe(SubscriptionID)
    // todo: add changing what screen is targeted
}

impl fmt::Display for EventManagerCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EventManagerCommand::Subscribe(sub_type, _) => {
                write!(f, "subscription type: {:?}", sub_type)
            }
        }
    }
}

// ______ here are all the enums, defining the state of the InputManager _____

/// defines, what screen inside of the rendere is targeted
#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum TargetScreen {
    /// a specific screen is targeted
    Screen(ScreenKey),
    /// no targeting has been specified
    None,
    /// all screens will be targeted
    Global,
}
impl fmt::Display for TargetScreen {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TargetScreen::Global => {
                write!(f, "targeting global")
            }
            TargetScreen::None => {
                write!(f, "targeting none")
            }
            TargetScreen::Screen(id) => {
                write!(f, "targeting screen with id: {}", id)
            }
        }
    }
}

/// defines all the states a mouse butten can be in
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, Hash)]
pub enum MouseButtonState {
    Pressed,
    #[default]
    Released,
    Dragging,
}

/// holds the immediate state of the mouse
#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct MouseState {
    pub left_pressed: MouseButtonState,
    pub right_pressed: MouseButtonState,
    pub middle_pressed: MouseButtonState,
    pub mouse_pos: (u16, u16),
}

/// holds the immediate state of all crossterm event inputs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventManagerState {
    pub pressed_keys: HashMap<KeyCode, (KeyEvent, TargetScreen)>,
    pub mouse_state: MouseState,
    pub terminal_size: (u16, u16),
    pub targeted_screen: TargetScreen,
    pub is_terminal_focused: bool,
}

#[derive(Debug, Clone)]
pub struct EventHandler {
    sender: Sender<EventManagerCommand>,
    input_manager_state: Arc<Mutex<EventManagerState>>,
}

impl EventHandler {
    pub fn new(
        input_manager_state: Arc<Mutex<EventManagerState>>,
    ) -> (Self, Receiver<EventManagerCommand>) {
        let (sender, recv) = mpsc::channel();
        (
            EventHandler {
                sender,
                input_manager_state,
            },
            recv,
        )
    }

    pub fn create_hook(&self) -> EventHook {
        EventHook::new(self.sender.clone(), self.input_manager_state.clone())
    }
}

pub type SubscriptionID = usize;

pub type KeySubscription = (
    KeySubscriptionTypes,
    KeyAction,
    CbSender<SubscriptionMessage>,
);

#[derive(Debug, Default)]
pub struct EventSubscribers {
    mouse: HashMap<SubscriptionID, (MouseSubscriptionTypes, CbSender<SubscriptionMessage>)>,
    key: HashMap<SubscriptionID, KeySubscription>,
    resize: HashMap<SubscriptionID, CbSender<SubscriptionMessage>>,
    paste: HashMap<SubscriptionID, CbSender<SubscriptionMessage>>,
    terminal_focus: HashMap<SubscriptionID, CbSender<SubscriptionMessage>>,
}

pub type ScreenSelectPreprocessing = Arc<(
    Mutex<Receiver<Option<TargetScreen>>>,
    Sender<SubscriptionMessage>,
)>;

#[derive(Debug)]
pub struct CrosstermEventManager {
    state: Arc<Mutex<EventManagerState>>,
    shutdown_flag: Arc<AtomicBool>,
    subscription_idx: Arc<AtomicUsize>,
    reader_handle: Option<JoinHandle<()>>,
    global_recv: Option<Receiver<EventManagerCommand>>,
    subscribers: Arc<Mutex<EventSubscribers>>,
    #[cfg(feature = "screen_select_subscription")]
    screen_select_handler: Option<ScreenSelectPreprocessing>,
}

impl CrosstermEventManager {
    pub fn new(targeted_screen: TargetScreen) -> (Self, EventHandler) {
        CrosstermEventManager::new_with_start(targeted_screen, true)
    }
    fn new_with_start(targeted_screen: TargetScreen, start: bool) -> (Self, EventHandler) {
        let state = EventManagerState {
            pressed_keys: HashMap::new(),
            terminal_size: (0, 0),
            mouse_state: MouseState::default(),
            targeted_screen,
            is_terminal_focused: true,
        };

        let state = Arc::new(Mutex::new(state));

        let (handler, recv) = EventHandler::new(state.clone());
        let mut manager = CrosstermEventManager {
            state,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            subscription_idx: Arc::new(AtomicUsize::new(0)),
            reader_handle: None,
            global_recv: Some(recv),
            subscribers: Arc::new(Mutex::new(EventSubscribers::default())),
            #[cfg(feature = "screen_select_subscription")]
            screen_select_handler: None,
        };
        if start {
            manager.start_reader_thread();
        }
        (manager, handler)
    }
    #[cfg(feature = "screen_select_subscription")]
    pub fn new_with_select_sub(
        targeted_screen: TargetScreen,
    ) -> (Self, EventHandler, ScreenSelectHandler) {
        let (select_sender, select_recv) = mpsc::channel();
        let (mut mngr, event_handler) =
            CrosstermEventManager::new_with_start(targeted_screen, false);
        let (s_sub, r_sub) = mpsc::channel();
        mngr.screen_select_handler = Some(Arc::new((Mutex::new(select_recv), s_sub)));
        mngr.start_reader_thread();
        let ssh = ScreenSelectHandler::new(r_sub, select_sender);
        (mngr, event_handler, ssh)
    }

    fn start_reader_thread(&mut self) {
        let state = Arc::clone(&self.state);
        let shutdown = Arc::clone(&self.shutdown_flag);
        let subscription_idx = Arc::clone(&self.subscription_idx);
        let subscribers = Arc::clone(&self.subscribers);
        let global_recv = self.global_recv.take().expect("global_recv already taken");
        #[cfg(feature = "screen_select_subscription")]
        let mut screen_select_handler = mem::take(&mut self.screen_select_handler);

        // constants:
        #[cfg(feature = "screen_select_subscription")]
        let max_screen_selector_reponse_wait_time = Duration::from_millis(1);

        macro_rules! send_subscription_message_to {
            ($field:ident, $message:expr) => {
                if let Ok(mut sub) = subscribers.lock() {
                    let mut error_ids = Vec::new();
                    for (id, sender) in &sub.$field {
                        if let Err(e) = sender.send($message) {
                            error_ids.push((*id, e));
                        }
                    }
                    for (id, e) in error_ids {
                        info!(
                            "failed to send SubscriptionMessage to subscriber with id: {} and err: {}",
                            id, e
                        );
                        sub.$field.remove(&id);
                    }
                }
            };
        }
        fn send_key_subscription_message(
            message: KeyMessage,
            key_sub: &mut HashMap<SubscriptionID, KeySubscription>,
            targeted_screen: TargetScreen,
        ) {
            let keys: Vec<SubscriptionID> = key_sub.keys().cloned().collect();
            let mut error_ids = Vec::new();

            for id in keys {
                if let Some((sub_type, expected_key_action, sender)) = key_sub.get(&id) {
                    // does sub want this msg?
                    let should_send = match sub_type {
                        KeySubscriptionTypes::All => true,
                        KeySubscriptionTypes::Specific(expected_code) => match message {
                            KeyMessage::Pressed(code) => {
                                if let KeyAction::Pressed = expected_key_action
                                    && code == *expected_code
                                {
                                    true
                                } else {
                                    false
                                }
                            }
                            KeyMessage::Released(code) => {
                                if let KeyAction::Released = expected_key_action
                                    && code == *expected_code
                                {
                                    true
                                } else {
                                    false
                                }
                            }
                            KeyMessage::Repeating(code) => {
                                if let KeyAction::Repeated = expected_key_action
                                    && code == *expected_code
                                {
                                    true
                                } else {
                                    false
                                }
                            }
                        },
                    };

                    if !should_send {
                        continue;
                    }

                    if let Err(e) = sender.send(SubscriptionMessage::Key {
                        msg: message,
                        screen: targeted_screen,
                    }) {
                        error_ids.push((id, e));
                    }
                }
            }

            for (id, e) in error_ids {
                info!(
                    "failed to send SubscriptionMessage to subscriber with id: {} and err: {}",
                    id, e
                );
                key_sub.remove(&id);
            }
        }

        fn send_subscription_message_to_mouse(
            msg: MouseMessage,
            mouse_sub: &mut HashMap<
                SubscriptionID,
                (MouseSubscriptionTypes, CbSender<SubscriptionMessage>),
            >,
            targeted_screen: TargetScreen,
        ) {
            let ids: Vec<SubscriptionID> = mouse_sub.keys().cloned().collect();
            let mut error_ids: Vec<(
                SubscriptionID,
                crossbeam_channel::SendError<SubscriptionMessage>,
            )> = Vec::new();

            for id in ids {
                if let Some((sub_type, sender)) = mouse_sub.get(&id) {
                    let should_send = match sub_type {
                        MouseSubscriptionTypes::All => true,
                        MouseSubscriptionTypes::Buttons => {
                            matches!(msg, MouseMessage::Pressed(_) | MouseMessage::Released(_))
                        }
                        MouseSubscriptionTypes::Scrolls => {
                            matches!(msg, MouseMessage::ScrollUp | MouseMessage::ScrollDown)
                        }
                        MouseSubscriptionTypes::Moves => matches!(msg, MouseMessage::Move(_, _)),
                        MouseSubscriptionTypes::ButtonAny(expected_btn) => match msg {
                            MouseMessage::Pressed(b) | MouseMessage::Released(b) => match b {
                                crossterm::event::MouseButton::Left => {
                                    *expected_btn == MouseButtons::Left
                                }
                                crossterm::event::MouseButton::Right => {
                                    *expected_btn == MouseButtons::Right
                                }
                                crossterm::event::MouseButton::Middle => {
                                    *expected_btn == MouseButtons::Middle
                                }
                            },
                            _ => false,
                        },
                        MouseSubscriptionTypes::ButtonAction(expected_btn, action) => {
                            match (msg, action) {
                                (MouseMessage::Pressed(b), MouseAction::Pressed) => match b {
                                    crossterm::event::MouseButton::Left => {
                                        *expected_btn == MouseButtons::Left
                                    }
                                    crossterm::event::MouseButton::Right => {
                                        *expected_btn == MouseButtons::Right
                                    }
                                    crossterm::event::MouseButton::Middle => {
                                        *expected_btn == MouseButtons::Middle
                                    }
                                },
                                (MouseMessage::Released(b), MouseAction::Released) => match b {
                                    crossterm::event::MouseButton::Left => {
                                        *expected_btn == MouseButtons::Left
                                    }
                                    crossterm::event::MouseButton::Right => {
                                        *expected_btn == MouseButtons::Right
                                    }
                                    crossterm::event::MouseButton::Middle => {
                                        *expected_btn == MouseButtons::Middle
                                    }
                                },
                                // if action == Any, any button event matches
                                (MouseMessage::Pressed(b), MouseAction::Any)
                                | (MouseMessage::Released(b), MouseAction::Any) => match b {
                                    crossterm::event::MouseButton::Left => {
                                        *expected_btn == MouseButtons::Left
                                    }
                                    crossterm::event::MouseButton::Right => {
                                        *expected_btn == MouseButtons::Right
                                    }
                                    crossterm::event::MouseButton::Middle => {
                                        *expected_btn == MouseButtons::Middle
                                    }
                                },
                                _ => false,
                            }
                        }
                    };

                    if !should_send {
                        continue;
                    }

                    let sub_msg = SubscriptionMessage::Mouse {
                        msg,
                        screen: targeted_screen,
                    };

                    if let Err(e) = sender.send(sub_msg) {
                        error_ids.push((id, e));
                    }
                }
            }

            // remove failed subscriptions
            for (id, e) in error_ids {
                warn!(
                    "failed to send mouse SubscriptionMessage to subscriber with id: {} and err: {}",
                    id, e
                );
                mouse_sub.remove(&id);
            }
        }

        // todo: make this function consume ev, bevcause of the String
        #[cfg(feature = "screen_select_subscription")]
        fn create_subscription_message_from_ev(
            ev: &mut Event,
            screen: TargetScreen,
        ) -> SubscriptionMessage {
            match ev {
                Event::FocusGained => {
                    SubscriptionMessage::TerminalWindowFocus(TerminalFocus::HasBeenFocused)
                }
                Event::FocusLost => {
                    SubscriptionMessage::TerminalWindowFocus(TerminalFocus::HasBeenUnfocused)
                }
                Event::Key(k) => {
                    let msg = if k.is_press() {
                        KeyMessage::Pressed(k.code)
                    } else if k.is_release() {
                        KeyMessage::Released(k.code)
                    } else {
                        KeyMessage::Repeating(k.code)
                    };
                    SubscriptionMessage::Key { msg, screen }
                }
                Event::Mouse(m) => {
                    let msg = match m.kind {
                        MouseEventKind::Down(mb) => MouseMessage::Pressed(mb),
                        MouseEventKind::Up(mb) => MouseMessage::Released(mb),
                        MouseEventKind::Moved => MouseMessage::Move(m.row, m.row),
                        MouseEventKind::ScrollDown => MouseMessage::ScrollDown,
                        MouseEventKind::ScrollUp => MouseMessage::ScrollUp,
                        // todo: implement these properly
                        MouseEventKind::Drag(_) => MouseMessage::ScrollDown,
                        MouseEventKind::ScrollLeft => MouseMessage::ScrollDown,
                        MouseEventKind::ScrollRight => MouseMessage::ScrollDown,
                    };
                    SubscriptionMessage::Mouse { msg, screen }
                }
                Event::Paste(str) => SubscriptionMessage::Paste {
                    content: Arc::new(str.clone()),
                    screen,
                },
                Event::Resize(w, h) => SubscriptionMessage::Resize(*w, *h),
            }
        }

        macro_rules! send_subscription_message_to_paste {
            ($field:ident, $content:expr) => {
                let mut sub = get_subscribers!();
                let mut error_ids = Vec::new();
                let st = get_state!();
                for (id, sender) in &sub.$field {
                    if let Err(e) = sender.send(SubscriptionMessage::Paste {
                        content: $content.clone(),
                        screen: st.targeted_screen,
                    }) {
                        error_ids.push((*id, e));
                    }
                }
                for (id, e) in error_ids {
                    warn!(
                        "failed to send Paste message to subscriber with id: {} and err: {}",
                        id, e
                    );
                    sub.$field.remove(&id);
                }
            };
        }

        macro_rules! get_state {
            () => {
                if let Ok(st) = state.lock() {
                    st
                } else {
                    continue;
                }
            };
        }

        macro_rules! get_subscribers {
            () => {
                if let Ok(sb) = subscribers.lock() {
                    sb
                } else {
                    continue;
                }
            };
        }

        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                while let Ok(command) = global_recv.try_recv() {
                    info!("manager received command: {:?}", &command);

                    match command {
                        EventManagerCommand::Subscribe(subscription_type, sender) => {
                            let idx = subscription_idx.fetch_add(1, Ordering::SeqCst);

                            if let Err(e) = sender.send(SubscriptionMessage::SubscriptionId(idx)) {
                                warn!(
                                    "failed to send SubscriptionId to subscriber (type: {:?}, id: {}): {:#?}",
                                    subscription_type, idx, e
                                );
                                continue;
                            }

                            let mut sub = match subscribers.lock() {
                                Ok(s) => s,
                                Err(e) => {
                                    warn!("subscribers mutex poisoned while subscribing: {:#?}", e);
                                    continue;
                                }
                            };

                            match subscription_type {
                                SubscriptionType::Key(key_type, key_action_type) => {
                                    sub.key.insert(idx, (key_type, key_action_type, sender));
                                }
                                SubscriptionType::Mouse(mouse_type) => {
                                    sub.mouse.insert(idx, (mouse_type, sender));
                                }
                                SubscriptionType::TerminalWindowFocus => {
                                    sub.terminal_focus.insert(idx, sender);
                                }
                                SubscriptionType::Paste => {
                                    sub.paste.insert(idx, sender);
                                }
                                SubscriptionType::Resize => {
                                    sub.resize.insert(idx, sender);
                                }
                            }
                        }
                    }
                }

                if !event::poll(Duration::from_millis(10)).unwrap_or(false) {
                    continue;
                }
                let mut ev = match event::read() {
                    Ok(ev) => ev,
                    Err(_) => continue,
                };

                // after an event is received, attempt to send a message to the screen selection handler, if one is present
                // this will be done before all of the subscription message, to allow all the following ones to be sent to the correct screen directly
                #[cfg(feature = "screen_select_subscription")]
                if let Some(select_arc) = &mut screen_select_handler {
                    let mut st = get_state!();

                    let subscription_msg =
                        create_subscription_message_from_ev(&mut ev, st.targeted_screen);

                    match select_arc.1.send(subscription_msg) {
                        Ok(()) => {}
                        Err(_) => {
                            return;
                        }
                    }

                    let max_wait = max_screen_selector_reponse_wait_time;
                    if let Ok(recv) = select_arc.0.lock() {
                        use std::sync::mpsc::RecvTimeoutError;

                        match recv.recv_timeout(max_wait) {
                            Ok(opt_target_screen) => {
                                if let Some(screen) = opt_target_screen {
                                    info!("set target screen to: {:?}", screen);
                                    st.targeted_screen = screen;
                                } else {
                                    info!("selector returned None");
                                }
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                info!("timed out waiting for screen_select response");
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                info!("selector channel disconnected");
                            }
                        }
                    }
                }
                let st = get_state!();
                drop(st);
                match ev {
                    Event::Key(key_event) => {
                        let mut st = get_state!();
                        let mut sub = get_subscribers!();
                        let screen_event = (key_event, st.targeted_screen);
                        if screen_event.0.is_press() {
                            // if the key was marked as pressed already, mark it as repeating
                            let message = if st.pressed_keys.contains_key(&screen_event.0.code) {
                                KeyMessage::Repeating(screen_event.0.code)
                            } else {
                                KeyMessage::Pressed(screen_event.0.code)
                            };
                            st.pressed_keys.insert(screen_event.0.code, screen_event);
                            send_key_subscription_message(
                                message,
                                &mut sub.key,
                                st.targeted_screen,
                            );
                        } else if screen_event.0.is_release() {
                            st.pressed_keys.remove(&screen_event.0.code);
                            send_key_subscription_message(
                                KeyMessage::Released(screen_event.0.code),
                                &mut sub.key,
                                st.targeted_screen,
                            );
                        }
                    }
                    Event::FocusGained => {
                        let mut st = get_state!();
                        if !st.is_terminal_focused {
                            st.is_terminal_focused = true;
                            send_subscription_message_to!(
                                terminal_focus,
                                SubscriptionMessage::TerminalWindowFocus(
                                    TerminalFocus::HasBeenFocused
                                )
                            );
                        }
                    }
                    Event::FocusLost => {
                        let mut st = get_state!();
                        if st.is_terminal_focused {
                            st.is_terminal_focused = false;
                            send_subscription_message_to!(
                                terminal_focus,
                                SubscriptionMessage::TerminalWindowFocus(
                                    TerminalFocus::HasBeenUnfocused
                                )
                            );
                        }
                        // todo: send the release info to all the hooks, that are subscribed to key events
                        st.pressed_keys = HashMap::new()
                    }
                    Event::Mouse(mouse_event) => {
                        fn set_mouse_button(
                            state: &mut MouseState,
                            button: &MouseButton,
                            button_state: MouseButtonState,
                        ) -> bool {
                            match button {
                                MouseButton::Left => {
                                    let change = state.left_pressed != button_state;
                                    if change {
                                        state.left_pressed = button_state
                                    }
                                    change
                                }
                                MouseButton::Right => {
                                    let change = state.right_pressed != button_state;
                                    if change {
                                        state.right_pressed = button_state
                                    }
                                    change
                                }
                                MouseButton::Middle => {
                                    let change = state.middle_pressed != button_state;
                                    if change {
                                        state.middle_pressed = button_state
                                    }
                                    change
                                }
                            }
                        }

                        let mut st = get_state!();
                        let mut sub = get_subscribers!();
                        let mut maybe_msg = None;

                        match mouse_event.kind {
                            MouseEventKind::Down(b) => {
                                if set_mouse_button(
                                    &mut st.mouse_state,
                                    &b,
                                    MouseButtonState::Pressed,
                                ) {
                                    maybe_msg = Some(MouseMessage::Pressed(b));
                                }
                            }
                            MouseEventKind::Up(b) => {
                                if set_mouse_button(
                                    &mut st.mouse_state,
                                    &b,
                                    MouseButtonState::Released,
                                ) {
                                    maybe_msg = Some(MouseMessage::Released(b));
                                }
                            }
                            MouseEventKind::Drag(b) => {
                                if set_mouse_button(
                                    &mut st.mouse_state,
                                    &b,
                                    MouseButtonState::Dragging,
                                ) {
                                    // treat drag as pressed continuation
                                    maybe_msg = Some(MouseMessage::Pressed(b));
                                }
                            }
                            MouseEventKind::Moved => {
                                if (mouse_event.column, mouse_event.row) != st.mouse_state.mouse_pos
                                {
                                    maybe_msg = Some(MouseMessage::Move(
                                        mouse_event.column,
                                        mouse_event.row,
                                    ));
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                maybe_msg = Some(MouseMessage::ScrollUp);
                            }
                            MouseEventKind::ScrollDown => {
                                maybe_msg = Some(MouseMessage::ScrollDown);
                            }
                            _ => {}
                        }

                        st.mouse_state.mouse_pos = (mouse_event.column, mouse_event.row);

                        if let Some(msg) = maybe_msg {
                            let screen = st.targeted_screen;
                            send_subscription_message_to_mouse(msg, &mut sub.mouse, screen);
                        }
                    }
                    Event::Paste(content) => {
                        let shared = Arc::new(content);

                        send_subscription_message_to_paste!(paste, shared);
                    }
                    Event::Resize(w, h) => {
                        let mut st = get_state!();
                        st.terminal_size = (w, h);
                        send_subscription_message_to!(resize, SubscriptionMessage::Resize(w, h));
                    }
                }

                thread::yield_now();
            }
        });

        self.reader_handle = Some(handle);
    }

    /// stop the reader thread
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.reader_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for CrosstermEventManager {
    fn drop(&mut self) {
        self.stop();
    }
}
