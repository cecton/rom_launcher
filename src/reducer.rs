use state::State;
use actions::Action;

pub fn reduce(state: &State, ref action: Action) -> State {
    let mut new_state = state.clone();
    new_state.x = 1;
    return new_state;
}
