/**----------------------------------------------------------------------
*!  Action-based input handling for keyboard, mouse, and gamepad.
*----------------------------------------------------------------------**/
#[cfg(not(target_arch = "wasm32"))]
use gilrs::{Axis, Button, Event, EventType, Gilrs};
use std::collections::HashMap;
use std::fmt::Debug;
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

//? Trait for game-defined action enums.
pub trait GameAction: Copy + Eq + Debug + 'static {
    //* Total number of action variants.
    fn count() -> usize;

    //* Unique index for this action variant (0-based, must be < `count()`).
    fn index(&self) -> usize;

    //* Reconstruct an action from its index. Returns `None` for out-of-range indices.
    fn from_index(index: usize) -> Option<Self>;

    fn move_negative_x() -> Option<Self> {
        None
    }

    fn move_positive_x() -> Option<Self> {
        None
    }

    fn move_negative_y() -> Option<Self> {
        None
    }

    fn move_positive_y() -> Option<Self> {
        None
    }
}

//? Mouse buttons that can be bound to game actions via `InputMap::bind_mouse()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseBinding {
    Left,
    Right,
    Middle,
}

//? Keyboard key enumeration (hardware-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    W,
    A,
    S,
    D,
    Space,
    Shift,
    Alt,
    Up,
    Down,
    Left,
    Right,
    F12,
    Escape,
}

impl Key {
    fn from_keycode(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::KeyW => Some(Key::W),
            KeyCode::KeyA => Some(Key::A),
            KeyCode::KeyS => Some(Key::S),
            KeyCode::KeyD => Some(Key::D),
            KeyCode::Space => Some(Key::Space),
            KeyCode::ShiftLeft => Some(Key::Shift),
            KeyCode::AltLeft => Some(Key::Alt),
            KeyCode::ArrowUp => Some(Key::Up),
            KeyCode::ArrowDown => Some(Key::Down),
            KeyCode::ArrowLeft => Some(Key::Left),
            KeyCode::ArrowRight => Some(Key::Right),
            KeyCode::F12 => Some(Key::F12),
            KeyCode::Escape => Some(Key::Escape),
            _ => None,
        }
    }
}

//? Input mapping between hardware inputs and game actions.
pub struct InputMap<A: GameAction> {
    keyboard_map: HashMap<Key, A>,
    mouse_map: HashMap<MouseBinding, A>,
    #[cfg(not(target_arch = "wasm32"))]
    gamepad_map: HashMap<Button, A>,
}

impl<A: GameAction> InputMap<A> {
    pub fn new() -> Self {
        Self {
            keyboard_map: HashMap::new(),
            mouse_map: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            gamepad_map: HashMap::new(),
        }
    }

    pub fn bind_key(&mut self, key: Key, action: A) {
        self.keyboard_map.insert(key, action);
    }

    pub fn bind_mouse(&mut self, button: MouseBinding, action: A) {
        self.mouse_map.insert(button, action);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn bind_button(&mut self, button: Button, action: A) {
        self.gamepad_map.insert(button, action);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get_action_for_button(&self, button: Button) -> Option<A> {
        self.gamepad_map.get(&button).copied()
    }
}

impl<A: GameAction> Default for InputMap<A> {
    fn default() -> Self {
        Self::new()
    }
}

//? Input buffer entry: (action, time_pressed).
#[derive(Debug, Clone, Copy)]
struct BufferedInput<A: GameAction> {
    action: A,
    time_pressed: f32,
}

//? Input state tracking keyboard, mouse, and gamepad.
pub struct InputState<A: GameAction> {
    keys: [bool; 13],
    keys_prev: [bool; 13],
    actions: Vec<bool>,
    actions_prev: Vec<bool>,
    mouse_buttons: [bool; 3],
    gamepad_axes: [f32; 2],

    #[cfg(not(target_arch = "wasm32"))]
    gamepad_buttons: Vec<bool>,
    input_map: InputMap<A>,

    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<Gilrs>,
    input_buffer: Vec<BufferedInput<A>>,
    current_time: f32,
}

impl<A: GameAction> InputState<A> {
    pub fn new() -> Self {
        let action_count = A::count();
        Self {
            keys: [false; 13],
            keys_prev: [false; 13],
            actions: vec![false; action_count],
            actions_prev: vec![false; action_count],
            mouse_buttons: [false; 3],
            gamepad_axes: [0.0; 2],
            #[cfg(not(target_arch = "wasm32"))]
            gamepad_buttons: vec![false; action_count],
            input_map: InputMap::new(),

            #[cfg(not(target_arch = "wasm32"))]
            gilrs: match Gilrs::new() {
                Ok(g) => Some(g),
                Err(e) => {
                    log::warn!("Failed to initialize gamepad support: {e}");
                    None
                }
            },
            input_buffer: Vec::with_capacity(8),
            current_time: 0.0,
        }
    }

    pub fn is_action_pressed(&self, action: A) -> bool {
        self.actions[action.index()]
    }

    pub fn is_action_just_pressed(&self, action: A) -> bool {
        let idx = action.index();
        self.actions[idx] && !self.actions_prev[idx]
    }

    pub fn was_action_pressed_buffered(&self, action: A, buffer_window: f32) -> bool {
        if self.is_action_just_pressed(action) {
            return true;
        }

        for buffered in &self.input_buffer {
            if buffered.action == action {
                let elapsed = self.current_time - buffered.time_pressed;
                if elapsed <= buffer_window {
                    return true;
                }
            }
        }

        false
    }

    //? Check if a raw key is pressed (for non-action keys like Escape).
    #[allow(dead_code)]
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys[key as usize]
    }

    //? Check if a raw key was just pressed this frame.
    #[allow(dead_code)]
    pub fn is_key_just_pressed(&self, key: Key) -> bool {
        let idx = key as usize;
        self.keys[idx] && !self.keys_prev[idx]
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_buttons[0],
            MouseButton::Right => self.mouse_buttons[1],
            MouseButton::Middle => self.mouse_buttons[2],
            _ => false,
        }
    }

    //? Get horizontal movement axis (-1.0 to 1.0).
    pub fn get_move_x(&self) -> f32 {
        let mut value = 0.0;

        if let Some(left) = A::move_negative_x()
            && self.is_action_pressed(left)
        {
            value -= 1.0;
        }
        if let Some(right) = A::move_positive_x()
            && self.is_action_pressed(right)
        {
            value += 1.0;
        }

        if self.gamepad_axes[0].abs() > 0.1 {
            value = self.gamepad_axes[0];
        }

        value
    }

    //? Get vertical movement axis (-1.0 to 1.0).
    pub fn get_move_y(&self) -> f32 {
        let mut value = 0.0;

        if let Some(up) = A::move_negative_y()
            && self.is_action_pressed(up)
        {
            value -= 1.0;
        }
        if let Some(down) = A::move_positive_y()
            && self.is_action_pressed(down)
        {
            value += 1.0;
        }

        if self.gamepad_axes[1].abs() > 0.1 {
            value = self.gamepad_axes[1];
        }

        value
    }

    pub fn any_keyboard_or_mouse(&self) -> bool {
        self.keys.iter().any(|&k| k) || self.mouse_buttons.iter().any(|&b| b)
    }

    pub fn any_gamepad(&self) -> bool {
        if self.gamepad_axes.iter().any(|&a| a.abs() > 0.1) {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.gamepad_buttons.iter().any(|&b| b) {
                return true;
            }
        }
        false
    }

    //? Called at the start of each frame. Saves previous state, rebuilds
    //? actions from raw keyboard/mouse/gamepad, and polls gamepad events.
    //? Also buffers new pressed actions and cleans up old buffer entries.
    pub fn begin_frame(&mut self, delta_time: f32) {
        self.actions_prev.clone_from(&self.actions);
        self.current_time += delta_time;

        //? Rebuild action state from raw keyboard state
        self.actions.fill(false);
        for (&key, &action) in &self.input_map.keyboard_map {
            if self.keys[key as usize] {
                self.actions[action.index()] = true;
            }
        }

        //? Apply mouse button bindings from map
        for (&binding, &action) in &self.input_map.mouse_map {
            let idx = match binding {
                MouseBinding::Left => 0,
                MouseBinding::Right => 1,
                MouseBinding::Middle => 2,
            };
            if self.mouse_buttons[idx] {
                self.actions[action.index()] = true;
            }
        }

        //? Poll and apply gamepad events (native only)
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref mut gilrs) = self.gilrs {
                while let Some(Event { event, .. }) = gilrs.next_event() {
                    match event {
                        EventType::ButtonPressed(button, _) => {
                            if let Some(action) = self.input_map.get_action_for_button(button) {
                                self.gamepad_buttons[action.index()] = true;
                            }
                        }
                        EventType::ButtonReleased(button, _) => {
                            if let Some(action) = self.input_map.get_action_for_button(button) {
                                self.gamepad_buttons[action.index()] = false;
                            }
                        }
                        EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                            self.gamepad_axes[0] = value;
                        }
                        EventType::AxisChanged(Axis::LeftStickY, value, _) => {
                            self.gamepad_axes[1] = -value;
                        }
                        _ => {}
                    }
                }
            }

            //? Merge persistent gamepad button state into actions
            let action_count = A::count();
            for i in 0..action_count {
                if self.gamepad_buttons[i] {
                    self.actions[i] = true;
                }
            }
        }

        //? Buffer freshly pressed actions (for input buffering)
        let action_count = A::count();
        for i in 0..action_count {
            if self.actions[i] && !self.actions_prev[i]
                && let Some(action) = A::from_index(i)
            {
                self.input_buffer.push(BufferedInput {
                    action,
                    time_pressed: self.current_time,
                });
            }
        }

        //? Clean up old buffer entries (older than 1 second)
        self.input_buffer
            .retain(|buffered| self.current_time - buffered.time_pressed < 1.0);
    }

    pub fn end_frame(&mut self) {
        self.keys_prev = self.keys;
    }

    //? Handle winit keyboard events (updates raw key state only).
    pub(crate) fn handle_key_event(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;

        if let PhysicalKey::Code(keycode) = event.physical_key
            && let Some(key) = Key::from_keycode(keycode)
        {
            self.keys[key as usize] = pressed;
        }
    }

    //? Handle mouse button events (updates raw mouse state only).
    pub(crate) fn handle_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.mouse_buttons[0] = pressed,
            MouseButton::Right => self.mouse_buttons[1] = pressed,
            MouseButton::Middle => self.mouse_buttons[2] = pressed,
            _ => {}
        }
    }

    pub fn input_map_mut(&mut self) -> &mut InputMap<A> {
        &mut self.input_map
    }
}

impl<A: GameAction> Default for InputState<A> {
    fn default() -> Self {
        Self::new()
    }
}
