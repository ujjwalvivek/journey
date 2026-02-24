/**----------------------------------------------------------------------
*!  Action-based input handling for keyboard, mouse, and gamepad.
*----------------------------------------------------------------------**/
//? gilrs not supported on wasm
//* so we conditionally compile gamepad code only for native targets.
#[cfg(not(target_arch = "wasm32"))]
use gilrs::{Axis, Button, Event, EventType, Gilrs};
use std::collections::HashMap;
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

//? Game actions (intent-based, decoupled from hardware).
//* These are the "verbs" of the game that the player can perform.
//* They are mapped to specific keybinds in `InputMap`, but game logic only cares about the actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Attack,
    Block,
    Dash,
    Grapple,
}

//? Keyboard key enumeration (hardware-specific).
//* This is used for raw key state tracking and mapping to game actions.
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

//? Input mapping between hardware and game actions.
//* This allows for customizable keybinds and gamepad bindings.
pub struct InputMap {
    keyboard_map: HashMap<Key, GameAction>,
    #[cfg(not(target_arch = "wasm32"))]
    gamepad_map: HashMap<Button, GameAction>,
}

//? Maps hardware inputs (keys, buttons) to game actions.
//* Native builds support both keyboard and gamepad mappings, while WASM builds only support keyboard.
impl Default for InputMap {
    fn default() -> Self {
        let mut keyboard_map = HashMap::new();

        //? Default key bindings
        keyboard_map.insert(Key::A, GameAction::MoveLeft);
        keyboard_map.insert(Key::Left, GameAction::MoveLeft);
        keyboard_map.insert(Key::D, GameAction::MoveRight);
        keyboard_map.insert(Key::Right, GameAction::MoveRight);
        keyboard_map.insert(Key::W, GameAction::MoveUp);
        keyboard_map.insert(Key::Up, GameAction::MoveUp);
        keyboard_map.insert(Key::S, GameAction::MoveDown);
        keyboard_map.insert(Key::Down, GameAction::MoveDown);
        keyboard_map.insert(Key::Space, GameAction::Jump);
        keyboard_map.insert(Key::Shift, GameAction::Dash);
        keyboard_map.insert(Key::Alt, GameAction::Grapple);

        //? Default gamepad bindings (native only)
        #[cfg(not(target_arch = "wasm32"))]
        let gamepad_map = {
            let mut map: HashMap<Button, GameAction> = HashMap::new();
            map.insert(Button::South, GameAction::Jump);
            map.insert(Button::West, GameAction::Attack);
            map.insert(Button::RightTrigger, GameAction::Block);
            map.insert(Button::RightTrigger2, GameAction::Dash);
            map.insert(Button::LeftTrigger2, GameAction::Grapple);
            map
        };

        //? Construct the InputMap with default bindings.
        Self {
            keyboard_map,
            #[cfg(not(target_arch = "wasm32"))]
            gamepad_map,
        }
    }
}

//? Methods for managing input mappings.
//* Allows for runtime customization of controls.
impl InputMap {
    pub fn new() -> Self {
        Self::default()
    }

    //? Bind a keyboard key to a game action.
    pub fn bind_key(&mut self, key: Key, action: GameAction) {
        self.keyboard_map.insert(key, action);
    }

    //? Bind a gamepad button to a game action (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn bind_button(&mut self, button: Button, action: GameAction) {
        self.gamepad_map.insert(button, action);
    }

    //? Get the game action bound to a keyboard key or a gamepad button, if any.
    //* Unused in the current scenario, but provides flexibility for custom controls.
    #[allow(dead_code)]
    fn get_action_for_key(&self, key: Key) -> Option<GameAction> {
        self.keyboard_map.get(&key).copied()
    }

    #[allow(dead_code)]
    #[cfg(not(target_arch = "wasm32"))]
    fn get_action_for_button(&self, button: Button) -> Option<GameAction> {
        self.gamepad_map.get(&button).copied()
    }
}

//? Input buffer struct: (action, time_pressed)
//* Used for implementing "Input Buffering" (e.g., coyote time, jump buffering).
#[derive(Debug, Clone, Copy)]
struct BufferedInput {
    action: GameAction,
    time_pressed: f32,
}

//? Input state tracking keyboard, mouse, and gamepad.
//* bool updated by winit event handlers.
pub struct InputState {
    keys: [bool; 13],
    keys_prev: [bool; 13],
    actions: [bool; 9],
    actions_prev: [bool; 9],
    mouse_buttons: [bool; 3],
    gamepad_axes: [f32; 2],

    #[cfg(not(target_arch = "wasm32"))]
    gamepad_buttons: [bool; 9],
    input_map: InputMap,

    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<Gilrs>,
    input_buffer: Vec<BufferedInput>,
    current_time: f32,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys: [false; 13],
            keys_prev: [false; 13],
            actions: [false; 9],
            actions_prev: [false; 9],
            mouse_buttons: [false; 3],
            gamepad_axes: [0.0; 2],
            #[cfg(not(target_arch = "wasm32"))]
            gamepad_buttons: [false; 9],
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

    //? Check if a game action is currently active.
    pub fn is_action_pressed(&self, action: GameAction) -> bool {
        self.actions[action as usize]
    }

    //? Check if a game action was just pressed this frame.
    pub fn is_action_just_pressed(&self, action: GameAction) -> bool {
        let idx = action as usize;
        self.actions[idx] && !self.actions_prev[idx]
    }

    //? Check if a game action was pressed within the buffer window.
    //* Returns true if the action was pressed within `buffer_window` set durations ago.
    pub fn was_action_pressed_buffered(&self, action: GameAction, buffer_window: f32) -> bool {
        //? Check current frame first (immediate)
        if self.is_action_just_pressed(action) {
            return true;
        }

        //? Check buffer for recent presses
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

    //? Check if a mouse button is pressed.
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

        if self.is_action_pressed(GameAction::MoveLeft) {
            value -= 1.0;
        }
        if self.is_action_pressed(GameAction::MoveRight) {
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

        if self.is_action_pressed(GameAction::MoveUp) {
            value -= 1.0;
        }
        if self.is_action_pressed(GameAction::MoveDown) {
            value += 1.0;
        }

        if self.gamepad_axes[1].abs() > 0.1 {
            value = self.gamepad_axes[1];
        }

        value
    }

    //? Return true if any keyboard key or mouse button is currently down.
    pub fn any_keyboard_or_mouse(&self) -> bool {
        self.keys.iter().any(|&k| k) || self.mouse_buttons.iter().any(|&b| b)
    }

    //? Return true if any gamepad axis or button is active.
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
        self.actions_prev = self.actions;
        self.current_time += delta_time;

        //? Rebuild action state from raw keyboard state
        self.actions = [false; 9];
        for (&key, &action) in &self.input_map.keyboard_map {
            if self.keys[key as usize] {
                self.actions[action as usize] = true;
            }
        }

        //? Apply mouse button bindings
        if self.mouse_buttons[0] {
            self.actions[GameAction::Attack as usize] = true;
        }
        if self.mouse_buttons[1] {
            self.actions[GameAction::Block as usize] = true;
        }

        //? Poll and apply gamepad events (native only)
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref mut gilrs) = self.gilrs {
                while let Some(Event { event, .. }) = gilrs.next_event() {
                    match event {
                        EventType::ButtonPressed(button, _) => {
                            if let Some(action) = self.input_map.get_action_for_button(button) {
                                self.gamepad_buttons[action as usize] = true;
                            }
                        }
                        EventType::ButtonReleased(button, _) => {
                            if let Some(action) = self.input_map.get_action_for_button(button) {
                                self.gamepad_buttons[action as usize] = false;
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
            for i in 0..9 {
                if self.gamepad_buttons[i] {
                    self.actions[i] = true;
                }
            }
        }

        //? Buffer freshly pressed actions (for input buffering)
        for action_idx in 0..9 {
            let action = match action_idx {
                0 => GameAction::MoveLeft,
                1 => GameAction::MoveRight,
                2 => GameAction::MoveUp,
                3 => GameAction::MoveDown,
                4 => GameAction::Jump,
                5 => GameAction::Attack,
                6 => GameAction::Block,
                7 => GameAction::Dash,
                8 => GameAction::Grapple,
                _ => continue,
            };

            //? If action was just pressed, add to buffer
            if self.actions[action_idx] && !self.actions_prev[action_idx] {
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

    //? Get mutable access to the input map for custom bindings.
    pub fn input_map_mut(&mut self) -> &mut InputMap {
        &mut self.input_map
    }
}

//? Default implementation for InputState
//? Initializes all states to false and sets up default mappings.
impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
