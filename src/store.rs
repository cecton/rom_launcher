/// The state of the application
#[derive(Clone, PartialEq, Debug)]
pub struct State {
    x: u32,
}

const INITIAL_STATE: State = State { x: 0 };

/// An Enum of all the possible actions in the application
#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    Initialize {},
}

/// Reducer
fn reduce(state: &State, action: &Action) -> State {
    let mut new_state = state.clone();
    new_state.x = 1;
    return new_state;
}

/// Store
pub struct Store {
    state: State,
}

impl Store {
    pub fn new() -> Store {
        debug!("initial state: {:?}", INITIAL_STATE);
        return Store {
            state: INITIAL_STATE,
        };
    }

    pub fn dispatch(&mut self, action: Action) -> Action {
        debug!("dispatch action {:?}", action);

        let new_state = reduce(&self.state, &action);
        if self.state == new_state {
            debug!("state untouched");
        } else {
            debug!("new state: {:?}", new_state);
            self.state = new_state;
        }

        return action;
    }

    pub fn get_state(&self) -> &State {
        return &self.state;
    }
}
