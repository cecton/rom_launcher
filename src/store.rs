use serde_json;
use std;
use std::collections::HashMap;

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
    };
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
    pub last_joystick_action: HashMap<i32, u32>,
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

    pub fn get_rom(&self) -> &Rom {
        self.roms
            .as_ref()
            .unwrap()
            .get((self.page_index * PAGE_SIZE + self.rom_selected) as usize)
            .unwrap()
    }

    pub fn player_needs_setup_controls(&self, player_index: usize) -> bool {
        match self.players[player_index].as_ref() {
            Some(player) => {
                let joystick_id = &player.joystick;
                self.joystick_needs_setup_controls(*joystick_id)
            }
            None => false,
        }
    }

    pub fn joystick_needs_setup_controls(&self, joystick_id: i32) -> bool {
        let guid = &self.joysticks[&joystick_id].guid;
        let emulator_id = &self.get_emulator().id;
        let rom = &self.get_rom().file_name;

        !self.console_configs.contains_key(guid, emulator_id)
            && !self.game_configs.contains_key(guid, rom)
    }

    pub fn any_player_needs_setup_controls(&self) -> bool {
        self.players
            .iter()
            .enumerate()
            .any(|(i, _)| self.player_needs_setup_controls(i))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveState {
    emulator_selected: i32,
    emulators: Vec<Emulator>,
    console_configs: JoystickConfig,
    game_configs: JoystickConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    List,
    GameLauncher,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Emulator {
    pub id: String,
    pub name: String,
    pub path: String,
    pub controls: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct Rom {
    pub path: String,
    pub name: String,
    pub file_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HatState {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AxisState {
    Positive,
    Negative,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum JoystickEvent {
    Unassigned,
    Button(u8),
    Hat(u8, HatState),
    Axis(u8, AxisState),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JoystickConfig(HashMap<JoystickGuid, HashMap<String, Vec<JoystickEvent>>>);

impl JoystickConfig {
    fn new() -> JoystickConfig {
        JoystickConfig(HashMap::new())
    }

    pub fn insert(&mut self, guid: JoystickGuid, key: String, mapping: Vec<JoystickEvent>) {
        if !self.0.contains_key(&guid) {
            self.0.insert(guid, HashMap::new());
        }

        self.0.get_mut(&guid).unwrap().insert(key, mapping);
    }

    pub fn contains_key(&self, guid: &JoystickGuid, key: &str) -> bool {
        self.0.contains_key(guid) && self.0.get(guid).unwrap().contains_key(key)
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub joystick: i32,
    pub menu: PlayerMenu,
    pub grab_input: Option<(GrabControl, Vec<JoystickEvent>)>,
    pub grab_emulator_buttons: Option<(Option<JoystickEvent>, Option<JoystickEvent>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerMenu {
    Controls,
    Ready,
    Leave,
    Waiting,
    ConsoleControls,
    GameControls,
    ControlsExit,
}

#[derive(Clone, Debug)]
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
    BindPlayerJoystickEvent(usize, JoystickEvent),
    UpdateJoystickLastAction(i32, u32),
    BindEmulatorButton(JoystickEvent),
}

/// Reducer
fn reduce(state: State, action: Action) -> State {
    use self::Action::*;

    match action {
        Initialize(save_state) => State {
            emulator_selected: save_state.emulator_selected,
            emulators: save_state.emulators,
            console_configs: save_state.console_configs,
            game_configs: save_state.game_configs,
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
                if state.players[0]
                    .as_ref()
                    .and_then(|x| x.grab_emulator_buttons.as_ref())
                    .is_some()
                {
                    return state;
                }

                let player_needs_setup_controls = state.joystick_needs_setup_controls(joystick);
                let mut players = state.players;

                players[free_slot] = Some(Player {
                    joystick,
                    menu: if player_needs_setup_controls {
                        PlayerMenu::Controls
                    } else {
                        PlayerMenu::Ready
                    },
                    grab_input: None,
                    grab_emulator_buttons: None,
                });

                State { players, ..state }
            }
        },
        NextPlayerMenu(joystick_id) => {
            use self::PlayerMenu::*;

            let i = state.get_player_index(joystick_id);
            let player_needs_setup_controls = state.player_needs_setup_controls(i);
            let mut players = state.players;
            if let Some(player) = players[i].as_mut() {
                match player.menu {
                    Ready => player.menu = Leave,
                    Controls => if player_needs_setup_controls {
                        player.menu = Leave;
                    } else {
                        player.menu = Ready;
                    },
                    ConsoleControls => player.menu = GameControls,
                    GameControls => player.menu = ControlsExit,
                    _ => {}
                }
            }

            State { players, ..state }
        }
        PrevPlayerMenu(joystick_id) => {
            use self::PlayerMenu::*;

            let i = state.get_player_index(joystick_id);
            let player_needs_setup_controls = state.player_needs_setup_controls(i);
            let mut players = state.players;
            if let Some(player) = players[i].as_mut() {
                match player.menu {
                    Leave => if player_needs_setup_controls {
                        player.menu = Controls;
                    } else {
                        player.menu = Ready;
                    },
                    Ready => player.menu = Controls,
                    GameControls => player.menu = ConsoleControls,
                    ControlsExit => player.menu = GameControls,
                    _ => {}
                }
            }

            State { players, ..state }
        }
        GoPlayerMenu(joystick_id) => {
            use self::GrabControl::*;
            use self::PlayerMenu::*;

            let mut screen = state.screen;
            let mut players = state.players;
            let mut remove_player = None;
            modify_player!(
                players,
                joystick_id,
                |i: usize, player: &mut Player| match player.menu {
                    Ready => if i == 0 {
                        player.grab_emulator_buttons = Some((None, None));
                    } else {
                        player.menu = Waiting;
                    },
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
        BindPlayerJoystickEvent(i, event) => {
            use self::GrabControl::*;

            let controls_len = state.get_controls().len();
            let emulator_id = state.get_emulator().id.clone();
            let rom = state.get_rom().file_name.clone();
            let mut players = state.players;
            let mut console_configs = state.console_configs;
            let mut game_configs = state.game_configs;
            if let Some(player) = players[i].as_mut() {
                let guid = state.joysticks[&player.joystick].guid;
                let mut save_mapping = None;
                let (control, mut mapping) = player.grab_input.take().unwrap();
                if mapping.len() < controls_len {
                    if mapping.iter().any(|x| *x == event) {
                        mapping.push(JoystickEvent::Unassigned);
                    } else {
                        mapping.push(event);
                    }

                    if mapping.len() == controls_len {
                        save_mapping = Some((control, mapping));
                    } else {
                        player.grab_input = Some((control, mapping));
                    }

                    match save_mapping {
                        Some((Console, mapping)) => {
                            console_configs.insert(guid, emulator_id, mapping);
                        }
                        Some((Game, mapping)) => {
                            game_configs.insert(guid, rom, mapping);
                        }
                        _ => {}
                    }
                }
            }

            State {
                players,
                console_configs,
                game_configs,
                ..state
            }
        }
        UpdateJoystickLastAction(joystick_id, timestamp) => {
            let mut last_joystick_action = state.last_joystick_action;
            last_joystick_action.insert(joystick_id, timestamp);

            State {
                last_joystick_action,
                ..state
            }
        }
        BindEmulatorButton(event) => {
            let mut players = state.players;

            if let Some(player) = players[0].as_mut() {
                let (hotkey, menu) = player.grab_emulator_buttons.take().unwrap();

                if hotkey.is_none() {
                    player.grab_emulator_buttons = Some((Some(event), menu));
                } else if menu.is_none() {
                    player.grab_emulator_buttons = Some((hotkey, Some(event)));
                    player.menu = PlayerMenu::Waiting;
                }
            }

            State { players, ..state }
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
            last_joystick_action: HashMap::new(),
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

    pub fn dump(&self) -> Result<Vec<u8>, String> {
        if let Some(state) = self.state.as_ref() {
            let save_state = SaveState {
                emulator_selected: state.emulator_selected,
                emulators: state.emulators.clone(),
                console_configs: state.console_configs.clone(),
                game_configs: state.game_configs.clone(),
            };
            debug!("state dumped to: {:?}", save_state);

            Ok(serde_json::to_string(&save_state)
                .map_err(|x| format!("{}", x))?
                .into_bytes())
        } else {
            Err("state is none".to_string())
        }
    }

    pub fn load<R>(&mut self, reader: R)
    where
        R: std::io::Read,
    {
        match serde_json::from_reader(reader) {
            Ok(save_state) => {
                debug!("state loaded: {:?}", save_state);
                self.dispatch(Action::Initialize(save_state));
                self.process();
            }
            Err(err) => panic!("could not load state: {}", err),
        }
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

fn get_roms(path: &str) -> Result<Vec<Rom>, String> {
    let resolved_path = path.replace("~", std::env::home_dir().unwrap().to_str().unwrap());
    let mut roms = std::fs::read_dir(resolved_path)
        .or_else(|x| Err(format!("{}", x)))?
        .map(|x| x.unwrap().path())
        .filter(|x| x.is_file())
        .map(|x| Rom {
            path: x.to_str().unwrap().to_string(),
            name: x.file_stem().unwrap().to_os_string().into_string().unwrap(),
            file_name: x.file_name().unwrap().to_os_string().into_string().unwrap(),
        })
        .collect::<Vec<_>>();
    roms.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(roms)
}
