use std::fs;
use std::env;
use std::error::Error;
use std::collections::HashMap;
use bincode::{deserialize, serialize, ErrorKind};

use joystick::*;

pub const PAGE_SIZE: i32 = 15;

macro_rules! modify_player {
    ($players:expr, $joystick:expr, $closure:expr) => {
        for (i, maybe_player) in $players.iter_mut().enumerate() {
            if let Some(player) = maybe_player.as_mut() {
                if player.joystick == $joystick {
                    $closure(i, player)
                }
            }
        }
    }
}

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
    pub joysticks: HashMap<i32, JoystickInfo>,
    pub players: [Option<Player>; 10],
    pub console_configs: JoystickConfig,
    pub game_configs: JoystickConfig,
}

impl State {
    pub fn get_emulator(&self) -> &Emulator {
        &self.emulators[self.emulator_selected as usize]
    }

    pub fn get_controls(&self) -> &Vec<(String, String)> {
        &self.get_emulator().controls
    }

    pub fn get_player_index(&self, joystick_id: i32) -> usize {
        self.players
            .iter()
            .position(|x| x.as_ref().map(|x| x.joystick) == Some(joystick_id))
            .unwrap()
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
    pub controls: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct JoystickConfig(HashMap<JoystickGuid, HashMap<String, Vec<String>>>);

impl JoystickConfig {
    fn new() -> JoystickConfig {
        JoystickConfig(HashMap::new())
    }

    pub fn insert(&mut self, guid: JoystickGuid, key: String, mapping: Vec<String>) {
        if !self.0.contains_key(&guid) {
            self.0.insert(guid, HashMap::new());
        }

        self.0.get_mut(&guid).unwrap().insert(key, mapping);
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub joystick: i32,
    pub menu: PlayerMenu,
    pub grab_input: Option<(GrabControl, Vec<String>)>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PlayerMenu {
    Controls,
    Ready,
    Leave,
    Waiting,
    ConsoleControls,
    GameControls,
    ControlsExit,
}

#[derive(Copy, Clone, Debug)]
pub enum GrabControl {
    Console,
    Game,
}

/// An Enum of all the possible actions in the application
#[derive(Clone, Debug)]
pub enum Action {
    Initialize(SaveState),
    LoadRoms { roms: Result<Vec<Rom>, String> },
    NextRom { step: i32 },
    NextPage { step: i32 },
    NextEmulator { step: i32 },
    AddJoystick(JoystickInfo),
    RemoveJoystick(i32),
    LaunchGame(i32),
    AddPlayer(i32),
    NextPlayerMenu(i32),
    PrevPlayerMenu(i32),
    GoPlayerMenu(i32),
    BindJoytstickButton(i32, u8),
}

/// Reducer
fn reduce(state: State, action: Action) -> State {
    use self::Action::*;

    match action {
        Initialize(save_state) => State {
            emulator_selected: save_state.emulator_selected,
            ..state
        },
        LoadRoms { roms } => {
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
        NextRom { step } => {
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
        NextPage { step } => {
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
        NextEmulator { step } => {
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
        AddJoystick(info) => {
            let mut joysticks = state.joysticks;
            joysticks.insert(info.instance_id, info);

            State { joysticks, ..state }
        }
        RemoveJoystick(joystick_id) => {
            let mut joysticks = state.joysticks;
            joysticks.remove(&joystick_id);
            let mut players = state.players;
            let mut remove_player = None;
            let mut screen = state.screen;
            modify_player!(players, joystick_id, |i: usize, _player: &mut Player| {
                remove_player = Some(i)
            });
            if let Some(i) = remove_player {
                players[i] = None;
            }
            if players.iter().all(|x| x.is_none()) {
                screen = Screen::List;
            }

            State {
                joysticks,
                players,
                screen,
                ..state
            }
        }
        LaunchGame(..) => {
            let mut players = [None, None, None, None, None, None, None, None, None, None];

            State {
                screen: Screen::GameLauncher,
                players,
                ..state
            }
        }
        AddPlayer(joystick) => match state.players.iter().position(|x| x.is_none()) {
            None => state,
            Some(free_slot) => {
                let mut players = state.players;
                players[free_slot] = Some(Player {
                    joystick,
                    menu: PlayerMenu::Ready,
                    grab_input: None,
                });

                State { players, ..state }
            }
        },
        NextPlayerMenu(joystick_id) => {
            use self::PlayerMenu::*;

            let mut players = state.players;
            modify_player!(
                players,
                joystick_id,
                |_i: usize, player: &mut Player| match player.menu {
                    Ready => player.menu = Leave,
                    Controls => player.menu = Ready,
                    ConsoleControls => player.menu = GameControls,
                    GameControls => player.menu = ControlsExit,
                    _ => {}
                }
            );

            State { players, ..state }
        }
        PrevPlayerMenu(joystick_id) => {
            use self::PlayerMenu::*;

            let mut players = state.players;
            modify_player!(
                players,
                joystick_id,
                |_i: usize, player: &mut Player| match player.menu {
                    Leave => player.menu = Ready,
                    Ready => player.menu = Controls,
                    GameControls => player.menu = ConsoleControls,
                    ControlsExit => player.menu = GameControls,
                    _ => {}
                }
            );

            State { players, ..state }
        }
        GoPlayerMenu(joystick_id) => {
            use self::PlayerMenu::*;
            use self::GrabControl::*;

            let mut screen = state.screen;
            let mut players = state.players;
            let mut remove_player = None;
            modify_player!(
                players,
                joystick_id,
                |i: usize, player: &mut Player| match player.menu {
                    Ready => player.menu = Waiting,
                    Waiting => player.menu = Ready,
                    Controls => player.menu = ConsoleControls,
                    ControlsExit => player.menu = Controls,
                    Leave => remove_player = Some(i),
                    ConsoleControls => player.grab_input = Some((Console, Vec::new())),
                    GameControls => player.grab_input = Some((Game, Vec::new())),
                }
            );

            match remove_player {
                Some(0) => screen = Screen::List,
                Some(i) => players[i] = None,
                None => {}
            }

            State {
                screen,
                players,
                ..state
            }
        }
        BindJoytstickButton(joystick_id, button) => {
            let controls_len = state.get_controls().len();
            let emulator_id = state.get_emulator().id.clone();
            let mut players = state.players;
            let mut save_mapping = None;
            modify_player!(players, joystick_id, |_i: usize, player: &mut Player| {
                let (control, mut mapping) = player.grab_input.take().unwrap();
                if mapping.len() < controls_len {
                    mapping.push(format!("but{}", button));

                    if mapping.len() == controls_len {
                        save_mapping = Some(mapping);
                    } else {
                        player.grab_input = Some((control, mapping));
                    }
                }
            });

            let mut console_configs = state.console_configs;
            if let Some(mapping) = save_mapping {
                let guid = state.joysticks[&joystick_id].guid;
                console_configs.insert(guid, emulator_id, mapping);
            }

            State {
                players,
                console_configs,
                ..state
            }
        }
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
                controls: vec![
                    ("up".to_string(), "Up".to_string()),
                    ("down".to_string(), "Down".to_string()),
                    ("left".to_string(), "Left".to_string()),
                    ("right".to_string(), "Right".to_string()),
                    ("a".to_string(), "I".to_string()),
                    ("b".to_string(), "II".to_string()),
                    ("select".to_string(), "Select".to_string()),
                    ("start".to_string(), "Run".to_string()),
                ],
            },
            Emulator {
                id: "md".to_string(),
                name: "Mega Drive".to_string(),
                path: "~/md_roms".to_string(),
                controls: vec![],
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
            joysticks: HashMap::new(),
            players: [None, None, None, None, None, None, None, None, None, None],
            console_configs: JoystickConfig::new(),
            game_configs: JoystickConfig::new(),
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
        use self::StoreAction::*;
        let todo: Vec<_> = self.queue.drain(..).collect();

        for action in todo {
            match action {
                Simple(action) => {
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
                Thunk(f) => f(self),
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
    use self::Action::*;

    match &action {
        &Initialize { .. } | &NextEmulator { .. } => {
            store.dispatch_thunk(Box::new(|store: &mut Store| {
                let roms = get_roms(&store.get_state().get_emulator().path);
                store.dispatch(LoadRoms { roms })
            }));
            store.dispatch(NextRom { step: 0 });

            Some(action)
        }
        &NextPage { .. } => {
            store.dispatch(NextRom { step: 0 });

            Some(action)
        }
        &LaunchGame(joystick) => {
            store.dispatch(AddPlayer(joystick));

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
