/// The state of the application
#[derive(Clone, PartialEq, Debug)]
pub struct State<'a> {
    pub emulator: &'a Emulator,
    pub roms: Vec<String>,
}

#[derive(Debug)]
pub struct Emulator {
    pub id: &'static str,
    pub name: &'static str,
    pub path: &'static str,
}

impl PartialEq for Emulator {
    fn eq(&self, other: &Emulator) -> bool {
        return self.id == other.id;
    }
}

static PC_ENGINE: Emulator = Emulator {
    id: "pce",
    name: "PC Engine",
    path: "~/pce_roms",
};

/// An Enum of all the possible actions in the application
#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    Initialize {},
    LoadRoms { roms: Vec<String> },
}

/// Reducer
fn reduce<'a>(state: &State<'a>, action: Action) -> Option<State<'a>> {
    match action {
        Action::LoadRoms { roms } => {
            let mut new_state = State { roms, ..*state };
            Some(new_state)
        }
        _ => None,
    }
}

/// Store
type Middleware = Fn(Action) -> Vec<Action>;

pub struct Store<'a> {
    state: State<'a>,
    middlewares: Vec<Box<Middleware>>,
}

impl<'a> Store<'a> {
    pub fn new(middlewares: Vec<Box<Middleware>>) -> Store<'a> {
        let state = Self::get_initial_state();
        debug!("initial state: {:?}", state);

        Store { state, middlewares }
    }

    fn get_initial_state() -> State<'a> {
        State {
            emulator: &PC_ENGINE,
            roms: vec![],
        }
    }

    pub fn dispatch(&mut self, action: Action) -> Vec<Action> {
        let mut actions = vec![action];

        debug!("apply middlewares");
        for middleware in &self.middlewares {
            for action in actions.clone() {
                actions = middleware(action);
            }
        }

        debug!("dispatch actions: {:?}", actions);
        for action in actions.clone() {
            match reduce(&self.state, action) {
                Some(state) => {
                    debug!("{:?}", state);
                    self.state = state
                }
                None => debug!("state untouched"),
            }
        }

        return actions;
    }

    pub fn get_state(&self) -> &State {
        return &self.state;
    }
}

/// Store's middlewares
pub fn trigger_middleware(action: Action) -> Vec<Action> {
    match action {
        x @ Action::Initialize {} => vec![
            x,
            Action::LoadRoms {
                roms: vec![String::from("foo"), String::from("bar")],
            },
        ],
        _ => vec![action],
    }
}
