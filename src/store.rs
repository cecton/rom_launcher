use state::{State, INITIAL_STATE};
use actions::Action;
use reducer::reduce;

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

        let new_state = reduce(&self.state, action);
        if self.state == new_state {
            debug!("state untouched");
        } else {
            debug!("new state: {:?}", new_state);
            self.state = new_state;
        }

        return action;
    }
}
