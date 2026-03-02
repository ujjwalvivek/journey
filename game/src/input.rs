/**----------------------------------------------------------------------
*!  Journey-specific input actions and default bindings.
*----------------------------------------------------------------------**/
use engine::{GameAction, Key, MouseBinding};

//? All input actions available in Journey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JourneyAction {
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

impl GameAction for JourneyAction {
    fn count() -> usize {
        9
    }

    fn index(&self) -> usize {
        *self as usize
    }

    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::MoveLeft),
            1 => Some(Self::MoveRight),
            2 => Some(Self::MoveUp),
            3 => Some(Self::MoveDown),
            4 => Some(Self::Jump),
            5 => Some(Self::Attack),
            6 => Some(Self::Block),
            7 => Some(Self::Dash),
            8 => Some(Self::Grapple),
            _ => None,
        }
    }

    fn move_negative_x() -> Option<Self> {
        Some(Self::MoveLeft)
    }
    fn move_positive_x() -> Option<Self> {
        Some(Self::MoveRight)
    }
    fn move_negative_y() -> Option<Self> {
        Some(Self::MoveUp)
    }
    fn move_positive_y() -> Option<Self> {
        Some(Self::MoveDown)
    }
}

//? Register Journey's default keyboard, mouse, and gamepad bindings.
pub fn setup_default_bindings(input: &mut engine::InputState<JourneyAction>) {
    let map = input.input_map_mut();

    //? Keyboard
    map.bind_key(Key::A, JourneyAction::MoveLeft);
    map.bind_key(Key::Left, JourneyAction::MoveLeft);
    map.bind_key(Key::D, JourneyAction::MoveRight);
    map.bind_key(Key::Right, JourneyAction::MoveRight);
    map.bind_key(Key::W, JourneyAction::MoveUp);
    map.bind_key(Key::Up, JourneyAction::MoveUp);
    map.bind_key(Key::S, JourneyAction::MoveDown);
    map.bind_key(Key::Down, JourneyAction::MoveDown);
    map.bind_key(Key::Space, JourneyAction::Jump);
    map.bind_key(Key::Shift, JourneyAction::Dash);
    map.bind_key(Key::Alt, JourneyAction::Grapple);

    //? Mouse
    map.bind_mouse(MouseBinding::Left, JourneyAction::Attack);
    map.bind_mouse(MouseBinding::Right, JourneyAction::Block);

    //? Gamepad (native only)
    #[cfg(not(target_arch = "wasm32"))]
    {
        use engine::gilrs::Button;
        map.bind_button(Button::South, JourneyAction::Jump);
        map.bind_button(Button::West, JourneyAction::Attack);
        map.bind_button(Button::RightTrigger, JourneyAction::Block);
        map.bind_button(Button::RightTrigger2, JourneyAction::Dash);
        map.bind_button(Button::LeftTrigger2, JourneyAction::Grapple);
        map.bind_button(Button::DPadUp, JourneyAction::MoveUp);
        map.bind_button(Button::DPadDown, JourneyAction::MoveDown);
        map.bind_button(Button::DPadLeft, JourneyAction::MoveLeft);
        map.bind_button(Button::DPadRight, JourneyAction::MoveRight);
    }
}
