use std::fs;
use std::env;
use std::error::Error;
use bincode::{deserialize, serialize, ErrorKind};

pub const PAGE_SIZE: i32 = 15;

/// The state of the application
#[derive(Debug)]
pub struct State {
    pub screen: Screen,
    pub page_index: i32,
    pub page_count: i32,
    pub emulator_selected: i32,
    pub emulators: Vec<Emulator>,
    pub roms: Result<Vec<Rom>, String>,
    pub rom_selected: i32,
    pub rom_count: i32,
    pub players: [Option<Player>; 10],
}

impl State {
    pub fn get_emulator(&self) -> &Emulator {
        &self.emulators[self.emulator_selected as usize]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveState {
    pub emulator_selected: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    List,
    GameLauncher,
}

#[derive(Clone, Debug)]
pub struct Emulator {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Copy, Clone, Debug)]
pub struct Player {
    pub joystick: i32,
    pub menu: PlayerMenu,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PlayerMenu {
    Controls,
    Ready,
    Leave,
    Waiting,
}

/// An Enum of all the possible actions in the application
#[derive(Clone, Debug)]
pub enum Action {
    Initialize(SaveState),
    LoadRoms { roms: Result<Vec<Rom>, String> },
    NextRom { step: i32 },
    NextPage { step: i32 },
    NextEmulator { step: i32 },
    LaunchGame,
    AddPlayer(i32),
    NextPlayerMenu(i32),
    PrevPlayerMenu(i32),
}

/// Reducer
#[allow(unreachable_patterns)]
fn reduce(state: State, action: Action) -> State {
    match action {
        Action::Initialize(save_state) => State {
            emulator_selected: save_state.emulator_selected,
            ..state
        },
        Action::LoadRoms { roms } => {
            let rom_count = match &roms {
                &Err(_) => 0,
                &Ok(ref roms) => roms.len() as i32,
            };

            State {
                page_count: (rom_count - 1).wrapping_div(PAGE_SIZE) + 1,
                page_index: 0,
                rom_selected: -1,
                rom_count,
                roms,
                ..state
            }
        }
        Action::NextRom { step } => {
            let max = if state.page_index < state.page_count - 1 {
                PAGE_SIZE
            } else {
                state.rom_count.wrapping_rem(PAGE_SIZE)
            };
            let rom_selected = state.rom_selected + step;
            if state.roms.is_err() || rom_selected < -1 {
                State {
                    rom_selected: -1,
                    ..state
                }
            } else if rom_selected >= max {
                State {
                    rom_selected: max - 1,
                    ..state
                }
            } else {
                State {
                    rom_selected,
                    ..state
                }
            }
        }
        Action::NextPage { step } => {
            let page_index = state.page_index + step;
            if state.roms.is_err() || page_index < 0 || page_index >= state.page_count {
                state
            } else {
                State {
                    page_index,
                    ..state
                }
            }
        }
        Action::NextEmulator { step } => {
            let max = state.emulators.len() as i32 - 1;
            let mut emulator_selected = state.emulator_selected + step;
            if emulator_selected < 0 {
                emulator_selected = max;
            } else if emulator_selected > max {
                emulator_selected = 0;
            }

            State {
                emulator_selected,
                ..state
            }
        }
        Action::LaunchGame => State {
            screen: Screen::GameLauncher,
            ..state
        },
        Action::AddPlayer(joystick) => match state.players.iter().position(|x| x.is_none()) {
            None => state,
            Some(free_slot) => {
                let mut players = state.players;
                players[free_slot] = Some(Player {
                    joystick,
                    menu: PlayerMenu::Ready,
                });
                State { players, ..state }
            }
        },
        Action::NextPlayerMenu(joystick_id) => {
            let mut players = state.players;
            for maybe_player in players.iter_mut() {
                if let Some(player) = maybe_player.as_mut() {
                    if player.joystick == joystick_id {
                        match player.menu {
                            PlayerMenu::Ready => player.menu = PlayerMenu::Leave,
                            _ => {}
                        }
                    }
                }
            }

            State { players, ..state }
        }
        Action::PrevPlayerMenu(joystick_id) => {
            let mut players = state.players;
            for maybe_player in players.iter_mut() {
                if let Some(player) = maybe_player.as_mut() {
                    if player.joystick == joystick_id {
                        match player.menu {
                            PlayerMenu::Leave => player.menu = PlayerMenu::Ready,
                            _ => {}
                        }
                    }
                }
            }

            State { players, ..state }
        }
        _ => state,
    }
}

/// Store
pub struct Store {
    state: Option<State>,
    queue: Vec<StoreAction>,
}

enum StoreAction {
    Simple(Action),
    Thunk(Box<Fn(&mut Store)>),
}

impl Store {
    pub fn new() -> Store {
        let state = Self::get_initial_state();
        debug!("initial state: {:?}", state);

        Store {
            state: Some(state),
            queue: vec![],
        }
    }

    fn get_initial_state() -> State {
        let emulators = vec![
            Emulator {
                id: "pce".to_string(),
                name: "PC Engine".to_string(),
                path: "~/pce_roms".to_string(),
            },
            Emulator {
                id: "md".to_string(),
                name: "Mega Drive".to_string(),
                path: "~/md_roms".to_string(),
            },
        ];

        State {
            screen: Screen::List,
            emulators,
            page_index: 0,
            page_count: 1,
            emulator_selected: 0,
            roms: Ok(vec![]),
            rom_selected: -1,
            rom_count: 0,
            players: [None; 10],
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        debug!("enqueuing action: {:?}", action);
        self.queue.push(StoreAction::Simple(action));
    }

    pub fn dispatch_thunk(&mut self, f: Box<Fn(&mut Store)>) {
        debug!("enqueuing thunk action");
        self.queue.push(StoreAction::Thunk(f));
    }

    pub fn process(&mut self) {
        let todo: Vec<_> = self.queue.drain(..).collect();

        for action in todo {
            match action {
                StoreAction::Simple(action) => {
                    let mut action = Some(action);

                    debug!("applying middlewares");
                    action = trigger_middleware(self, action.unwrap());

                    if let Some(x) = action {
                        debug!("dispatching action: {:?}", x);
                        let mut state = self.state.take().unwrap();
                        state = reduce(state, x);
                        debug!("{:?}", state);
                        self.state = Some(state);
                    }
                }
                StoreAction::Thunk(f) => f(self),
            }
        }

        if !self.queue.is_empty() {
            self.process();
        }
    }

    pub fn get_state(&self) -> &State {
        self.state.as_ref().unwrap()
    }

    pub fn dump(&self) -> Result<Vec<u8>, Box<ErrorKind>> {
        let state = self.state.as_ref().unwrap();
        let save_state = SaveState {
            emulator_selected: state.emulator_selected,
        };
        debug!("state dumped to: {:?}", save_state);

        serialize(&save_state)
    }

    pub fn load(&mut self, serialized_state: &Vec<u8>) {
        let save_state: SaveState = deserialize(serialized_state).expect("could not load state");
        debug!("state loaded: {:?}", save_state);

        self.dispatch(Action::Initialize(save_state));
        self.process();
    }
}

/// Store's middlewares
fn trigger_middleware(store: &mut Store, action: Action) -> Option<Action> {
    match &action {
        &Action::Initialize { .. } | &Action::NextEmulator { .. } => {
            let f = |store: &mut Store| {
                let roms = get_roms(&store.get_state().get_emulator().path);
                store.dispatch(Action::LoadRoms { roms })
            };
            store.dispatch_thunk(Box::new(f));
            store.dispatch(Action::NextRom { step: 0 });

            Some(action)
        }
        &Action::NextPage { .. } => {
            store.dispatch(Action::NextRom { step: 0 });

            Some(action)
        }
        _ => Some(action),
    }
}

#[derive(Clone, Debug)]
pub struct Rom {
    pub path: String,
    pub name: String,
}

fn get_roms(path: &str) -> Result<Vec<Rom>, String> {
    let resolved_path = path.replace("~", env::home_dir().unwrap().to_str().unwrap());
    let mut roms = fs::read_dir(resolved_path)
        .or_else(|x| Err(String::from(x.description())))?
        .map(|x| x.unwrap().path())
        .filter(|x| x.is_file())
        .map(|x| Rom {
            path: x.to_str().unwrap().to_string(),
            name: x.file_stem().unwrap().to_os_string().into_string().unwrap(),
        })
        .collect::<Vec<_>>();
    roms.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(roms)
}
