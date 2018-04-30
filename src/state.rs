#[derive(Clone, PartialEq, Debug)]
pub struct State {
    pub x: u32,
}

pub const INITIAL_STATE: State = State { x: 0 };
