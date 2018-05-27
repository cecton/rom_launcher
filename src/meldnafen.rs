use id_tree::*;
use sdl2::event::Event;
use sdl2::joystick::HatState;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::cmp;
use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use app::*;
use draw::*;
use store;
use store::*;

const ENTITES: usize = 23;
const TV_XRES: i32 = 256;
const TV_YRES: i32 = 224;
const AXIS_THRESOLD: i16 = 0x4fff;
const JOYSTICK_LOCK_TIME: u32 = 100;
const JOYSTICK_LOCK_TIME_AXIS: u32 = 200;

macro_rules! set_highlight {
    ($font:expr, $value:expr) => {
        if $value {
            $font.texture.set_color_mod(255, 255, 0);
        } else {
            $font.texture.set_color_mod(255, 255, 255);
        }
    };
}

macro_rules! lock_joystick {
    ($joystick:expr, $timestamp:expr, $store:expr, $closure:expr) => {
        if $timestamp
            >= $store
                .get_state()
                .last_joystick_action
                .get(&$joystick)
                .or(Some(&0))
                .unwrap() + JOYSTICK_LOCK_TIME
        {
            $store.dispatch(Action::UpdateJoystickLastAction($timestamp, $joystick));
            $closure();
        }
    };
}

macro_rules! lock_joystick_axis {
    ($joystick:expr, $timestamp:expr, $store:expr, $closure:expr) => {
        if $timestamp
            >= $store
                .get_state()
                .last_joystick_action
                .get(&$joystick)
                .or(Some(&0))
                .unwrap() + JOYSTICK_LOCK_TIME_AXIS
        {
            $store.dispatch(Action::UpdateJoystickLastAction($timestamp, $joystick));
            $closure();
        }
    };
}

macro_rules! filter_player {
    ($store:expr, $player_index:expr, $joystick:expr) => {
        $store.get_state().players[$player_index]
            .as_ref()
            .unwrap()
            .joystick == $joystick;
    };
}

pub trait Entity {
    fn is_active(&self, _state: &State) -> bool;
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources);
    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store);
}

struct List {}

impl Entity for List {
    fn is_active(&self, state: &State) -> bool {
        state.screen == Screen::List
    }

    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources) {
        resources.font.set_line_spacing(0.75);
        resources.font.set_pos(0, 0);

        if state.rom_selected == -1 {
            resources.font.texture.set_color_mod(255, 255, 0);
        } else {
            resources.font.texture.set_color_mod(255, 255, 255);
        }
        resources
            .font
            .println(canvas, &format!("< {} >", state.get_emulator().name));
        resources.font.println(canvas, "");

        resources.font.texture.set_color_mod(255, 255, 255);
        match &state.roms {
            &Ok(ref roms) => {
                for (i, rom) in roms.iter()
                    .skip((state.page_index * PAGE_SIZE) as usize)
                    .take(PAGE_SIZE as usize)
                    .enumerate()
                {
                    if i as i32 == state.rom_selected {
                        resources.font.texture.set_color_mod(255, 255, 0);
                    }
                    resources.font.println(canvas, &rom.name);
                    if i as i32 == state.rom_selected {
                        resources.font.texture.set_color_mod(255, 255, 255);
                    }
                }
                resources.font.println(canvas, "");
                resources.font.println(
                    canvas,
                    &format!(
                        "Page {} of {} ({} roms)",
                        state.page_index + 1,
                        state.page_count,
                        roms.len()
                    ),
                );
            }
            &Err(ref err) => {
                resources.font.texture.set_color_mod(255, 0, 0);
                resources.font.println(canvas, &err);
            }
        }
    }

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) {
        use store::Action::*;
        let rom_selected = store.get_state().rom_selected;

        match *event {
            Event::JoyHatMotion {
                state: HatState::Down,
                which,
                timestamp,
                ..
            } => lock_joystick!(which, timestamp, store, || store
                .dispatch(NextRom { timestamp, step: 1 })),
            Event::JoyAxisMotion {
                axis_idx: 1,
                value,
                which,
                timestamp,
                ..
            } if value >= AXIS_THRESOLD =>
            {
                lock_joystick_axis!(which, timestamp, store, || store
                    .dispatch(NextRom { timestamp, step: 1 }))
            }
            Event::JoyHatMotion {
                state: HatState::Up,
                which,
                timestamp,
                ..
            } => lock_joystick!(which, timestamp, store, || store.dispatch(NextRom {
                timestamp,
                step: -1
            })),
            Event::JoyAxisMotion {
                axis_idx: 1,
                value,
                which,
                timestamp,
                ..
            } if value <= -AXIS_THRESOLD =>
            {
                lock_joystick_axis!(which, timestamp, store, || store.dispatch(NextRom {
                    timestamp,
                    step: -1
                }))
            }
            Event::JoyHatMotion {
                state: HatState::Right,
                which,
                timestamp,
                ..
            } => if rom_selected == -1 {
                lock_joystick!(which, timestamp, store, || store
                    .dispatch(NextEmulator { timestamp, step: 1 }))
            } else {
                lock_joystick!(which, timestamp, store, || store
                    .dispatch(NextPage { timestamp, step: 1 }))
            },
            Event::JoyAxisMotion {
                axis_idx: 0,
                value,
                which,
                timestamp,
                ..
            } if value >= AXIS_THRESOLD =>
            {
                if rom_selected == -1 {
                    lock_joystick!(which, timestamp, store, || store
                        .dispatch(NextEmulator { timestamp, step: 1 }))
                } else {
                    lock_joystick!(which, timestamp, store, || store
                        .dispatch(NextPage { timestamp, step: 1 }))
                }
            }
            Event::JoyHatMotion {
                state: HatState::Left,
                which,
                timestamp,
                ..
            } => if rom_selected == -1 {
                lock_joystick!(which, timestamp, store, || store.dispatch(NextEmulator {
                    timestamp,
                    step: -1
                }))
            } else {
                lock_joystick!(which, timestamp, store, || store.dispatch(NextPage {
                    timestamp,
                    step: -1
                }))
            },
            Event::JoyAxisMotion {
                axis_idx: 0,
                value,
                which,
                timestamp,
                ..
            } if value <= -AXIS_THRESOLD =>
            {
                if rom_selected == -1 {
                    lock_joystick!(which, timestamp, store, || store.dispatch(NextEmulator {
                        timestamp,
                        step: -1
                    }))
                } else {
                    lock_joystick!(which, timestamp, store, || store.dispatch(NextPage {
                        timestamp,
                        step: -1
                    }))
                }
            }
            Event::JoyButtonUp {
                button_idx: 0,
                which,
                timestamp,
                ..
            } => if rom_selected > -1 {
                store.dispatch(LaunchGame(timestamp, which))
            },
            _ => {}
        }
    }
}

struct GameLauncher {
    player_colors: [Color; 10],
}

impl Entity for GameLauncher {
    fn is_active(&self, state: &State) -> bool {
        state.screen == Screen::GameLauncher
    }

    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources) {
        resources.font.set_line_spacing(1.50);
        let line_height = resources.font.line_height;
        let mut background = Rect::new(0, 0, TV_XRES as u32, line_height as u32);

        for (i, player) in state.players.iter().enumerate() {
            if player.is_some() {
                canvas.set_draw_color(self.player_colors[i]);
                canvas.fill_rect(background);
            }
            background.y += line_height;
        }
    }

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) {
        use store::Action::*;

        match *event {
            Event::JoyButtonUp {
                which,
                button_idx: 0,
                timestamp,
                ..
            } => if !store
                .get_state()
                .players
                .iter()
                .filter(|x| x.is_some())
                .any(|x| x.as_ref().unwrap().joystick == which)
            {
                store.dispatch(AddPlayer(timestamp, which));
            },
            _ => {}
        }
    }
}

struct PlayerMenu {
    player_index: usize,
}

impl Entity for PlayerMenu {
    fn is_active(&self, state: &State) -> bool {
        if let Some(ref player) = state.players[self.player_index] {
            if player.grab_input.is_none() && player.grab_emulator_buttons.is_none() {
                return true;
            }
        }

        false
    }

    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources) {
        use store::PlayerMenu::*;

        resources.font.set_line_spacing(1.50);
        let line_height = resources.font.line_height;
        let player = state.players[self.player_index].as_ref().unwrap();
        let actual_player_index = state
            .players
            .iter()
            .take(self.player_index)
            .filter(|x| x.is_some())
            .count();

        resources.font.texture.set_color_mod(255, 255, 255);
        resources.font.set_pos(
            0,
            line_height * self.player_index as i32 + line_height.wrapping_div(4),
        );
        resources
            .font
            .print(canvas, &format!(" {:2} >   ", actual_player_index + 1));

        match player.menu {
            Controls | Ready | Leave => {
                set_highlight!(resources.font, player.menu == Controls);
                resources.font.print(canvas, "Controls");
                if (self.player_index == 0 && state.any_player_needs_setup_controls())
                    || state.player_needs_setup_controls(self.player_index)
                {
                    resources.font.texture.set_color_mod(0, 0, 0);
                } else {
                    set_highlight!(resources.font, player.menu == Ready);
                }
                if self.player_index == 0 {
                    resources.font.print(canvas, "   Start");
                } else {
                    resources.font.print(canvas, "   Ready");
                }
                set_highlight!(resources.font, player.menu == Leave);
                if self.player_index == 0 {
                    resources.font.print(canvas, "            Exit");
                } else {
                    resources.font.print(canvas, "            Leave");
                }
            }
            ConsoleControls | GameControls | ControlsExit => {
                set_highlight!(resources.font, player.menu == ConsoleControls);
                resources.font.print(canvas, "Console");
                set_highlight!(resources.font, player.menu == GameControls);
                resources.font.print(canvas, "    Game");
                set_highlight!(resources.font, player.menu == ControlsExit);
                resources.font.print(canvas, "             Back");
            }
            Waiting => {
                set_highlight!(resources.font, true);
                resources.font.print(canvas, "Waiting");
            }
        }
    }

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) {
        use store::Action::*;

        let player_joystick = store.get_state().players[self.player_index]
            .as_ref()
            .unwrap()
            .joystick;
        match *event {
            Event::JoyButtonUp {
                which,
                button_idx: 0,
                timestamp,
                ..
            } => if player_joystick == which {
                store.dispatch(GoPlayerMenu(timestamp, which));
            },
            Event::JoyHatMotion {
                which,
                state: HatState::Right,
                timestamp,
                ..
            } => if player_joystick == which {
                store.dispatch(NextPlayerMenu(timestamp, which));
            },
            Event::JoyAxisMotion {
                axis_idx: 0,
                value,
                which,
                timestamp,
                ..
            } if value >= AXIS_THRESOLD =>
            {
                lock_joystick!(which, timestamp, store, || store
                    .dispatch(NextPlayerMenu(timestamp, which)))
            }
            Event::JoyHatMotion {
                which,
                state: HatState::Left,
                timestamp,
                ..
            } => if player_joystick == which {
                store.dispatch(PrevPlayerMenu(timestamp, which));
            },
            Event::JoyAxisMotion {
                axis_idx: 0,
                value,
                which,
                timestamp,
                ..
            } if value <= -AXIS_THRESOLD =>
            {
                lock_joystick!(which, timestamp, store, || store
                    .dispatch(PrevPlayerMenu(timestamp, which)))
            }
            _ => {}
        }
    }
}

struct PlayerGrabInput {
    player_index: usize,
}

impl Entity for PlayerGrabInput {
    fn is_active(&self, state: &State) -> bool {
        state.players[self.player_index]
            .as_ref()
            .and_then(|x| x.grab_input.as_ref())
            .is_some()
    }

    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources) {
        resources.font.set_line_spacing(1.50);
        let line_height = resources.font.line_height;
        resources.font.texture.set_color_mod(255, 255, 255);
        resources.font.set_pos(
            0,
            line_height * self.player_index as i32 + line_height.wrapping_div(4),
        );
        let &(_, ref controls) = state.players[self.player_index]
            .as_ref()
            .unwrap()
            .grab_input
            .as_ref()
            .unwrap();
        let (_, ref input_display) =
            state.emulators[state.emulator_selected as usize].controls[controls.len()];
        resources
            .font
            .print(canvas, &format!(" {} >", input_display));
    }

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) {
        use self::JoystickEvent::*;
        use store::Action::*;

        match *event {
            Event::JoyButtonUp {
                which,
                button_idx,
                timestamp,
                ..
            } if filter_player!(store, self.player_index, which) =>
            {
                lock_joystick!(which, timestamp, store, || store.dispatch(
                    BindPlayerJoystickEvent(timestamp, self.player_index, Button(button_idx))
                ))
            }
            Event::JoyHatMotion {
                which,
                hat_idx,
                state,
                timestamp,
                ..
            } if filter_player!(store, self.player_index, which)
                && (state == HatState::Up || state == HatState::Down || state == HatState::Left
                    || state == HatState::Right) =>
            {
                lock_joystick!(which, timestamp, store, || store.dispatch(
                    BindPlayerJoystickEvent(
                        timestamp,
                        self.player_index,
                        Hat(
                            hat_idx,
                            match state {
                                HatState::Up => store::HatState::Up,
                                HatState::Down => store::HatState::Down,
                                HatState::Left => store::HatState::Left,
                                HatState::Right => store::HatState::Right,
                                _ => panic!("invalid state: {:?}", state),
                            }
                        )
                    )
                ))
            }
            Event::JoyAxisMotion {
                which,
                axis_idx,
                value,
                timestamp,
                ..
            } if filter_player!(store, self.player_index, which)
                && (value <= -AXIS_THRESOLD || value >= AXIS_THRESOLD) =>
            {
                lock_joystick_axis!(which, timestamp, store, || store.dispatch(
                    BindPlayerJoystickEvent(
                        timestamp,
                        self.player_index,
                        Axis(
                            axis_idx,
                            if value.is_positive() {
                                AxisState::Positive
                            } else {
                                AxisState::Negative
                            }
                        )
                    )
                ))
            }
            _ => {}
        }
    }
}

struct PlayerGrabEmulatorButtons;

impl Entity for PlayerGrabEmulatorButtons {
    fn is_active(&self, state: &State) -> bool {
        state.players[0]
            .as_ref()
            .and_then(|x| x.grab_emulator_buttons.as_ref())
            .is_some()
    }

    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources) {
        resources.font.set_line_spacing(1.50);
        let line_height = resources.font.line_height;
        resources.font.texture.set_color_mod(255, 255, 255);
        resources.font.set_pos(0, line_height.wrapping_div(4));
        let &(ref hotkey, ref menu) = state.players[0]
            .as_ref()
            .unwrap()
            .grab_emulator_buttons
            .as_ref()
            .unwrap();
        if hotkey.is_none() {
            resources
                .font
                .println(canvas, "    Press button for: emulator hotkey");
        } else if menu.is_none() {
            resources
                .font
                .println(canvas, "    Press button for: emulator menu");
        }
    }

    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) {
        use self::JoystickEvent::*;
        use store::Action::*;

        match *event {
            Event::JoyButtonUp {
                which,
                button_idx,
                timestamp,
                ..
            } if filter_player!(store, 0, which) =>
            {
                let hotkey = store.get_state().players[0]
                    .as_ref()
                    .unwrap()
                    .grab_emulator_buttons
                    .as_ref()
                    .unwrap()
                    .0
                    .clone();
                lock_joystick!(which, timestamp, store, || {
                    let new_joystick_event = Button(button_idx);

                    if let Some(joystick_event) = hotkey {
                        if joystick_event == new_joystick_event {
                            return;
                        }

                        app.quit();
                    }

                    store.dispatch(BindEmulatorButton(timestamp, new_joystick_event));
                })
            }
            _ => {}
        }
    }
}

struct Root {}

impl Entity for Root {
    fn is_active(&self, _state: &State) -> bool {
        true
    }

    fn render(&self, canvas: &mut Canvas<Window>, _state: &State, _resources: &mut Resources) {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
    }

    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) {
        use store::Action::*;

        match *event {
            Event::Quit { .. }
            | Event::KeyUp {
                keycode: Some(Keycode::Q),
                ..
            }
            | Event::KeyUp {
                keycode: Some(Keycode::Escape),
                ..
            } => app.quit(),
            Event::JoyDeviceAdded {
                which, timestamp, ..
            } => {
                if let Some(info) = app.open_joystick(which) {
                    store.dispatch(AddJoystick(timestamp, info));
                }
            }
            Event::JoyDeviceRemoved {
                which, timestamp, ..
            } => {
                store.dispatch(RemoveJoystick(timestamp, which));
                app.close_joystick(which);
            }
            _ => {}
        }
    }
}

pub struct Resources {
    font: Font,
}

pub struct Meldnafen {
    pub app: App,
    store: Store,
    resources: Resources,
    tree: Tree<Box<Entity>>,
}

impl Meldnafen {
    pub fn new() -> Meldnafen {
        let mut app = App::new(|video| {
            video
                .window("meldnafen", 800, 700)
                .position(0, 0)
                .borderless()
                .build()
        });
        app.sdl_context.mouse().show_cursor(false);
        let mut store = Store::new();
        Meldnafen::load_state(&mut store);
        store.dispatch(Action::NextEmulator {
            timestamp: 0,
            step: 0,
        });

        debug!("setting up canvas...");
        let (w, h) = app.canvas.output_size().unwrap();
        let zoom = cmp::min(w as i32 / TV_XRES, h as i32 / TV_YRES) as f32;
        app.canvas.set_scale(zoom, zoom).unwrap();

        let mut viewport = app.canvas.viewport();
        viewport.x = (viewport.w - TV_XRES) / 2;
        viewport.y = (viewport.h - TV_YRES) / 2;
        viewport.w = TV_XRES;
        viewport.h = TV_YRES;
        app.canvas.set_viewport(viewport);

        let resources = Self::load_resources(&app);
        let tree = Self::load_entites();

        Meldnafen {
            app,
            store,
            resources,
            tree,
        }
    }

    fn load_state(store: &mut Store) -> Result<(), io::Error> {
        let file = File::open("state.json")?;
        store.load(file);

        Ok(())
    }

    fn load_resources(app: &App) -> Resources {
        debug!("loading resources...");
        let font = Font::new(
            app.load_texture("font-12.png"),
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?()[]<>~-_+@:/'., ",
        );

        Resources { font }
    }

    #[allow(unused_must_use)]
    fn load_entites() -> Tree<Box<Entity>> {
        use id_tree::InsertBehavior::*;

        let mut tree: Tree<Box<Entity>> = TreeBuilder::new().with_node_capacity(ENTITES).build();
        let root_id = tree.insert(Node::new(Box::new(Root {})), AsRoot).unwrap();
        tree.insert(Node::new(Box::new(List {})), UnderNode(&root_id));
        let player_colors = [
            Color::RGB(0xb9, 0x00, 0x00),
            Color::RGB(0x00, 0x00, 0xb9),
            Color::RGB(0x00, 0xb9, 0x00),
            Color::RGB(0xb9, 0x00, 0xb9),
            Color::RGB(0x00, 0xb9, 0xb9),
            Color::RGB(0xb9, 0x5c, 0x00),
            Color::RGB(0xb9, 0x00, 0x5c),
            Color::RGB(0x5c, 0xb9, 0x00),
            Color::RGB(0x5c, 0x00, 0xb9),
            Color::RGB(0x00, 0x5c, 0xb9),
        ];
        let game_launcher = tree.insert(
            Node::new(Box::new(GameLauncher { player_colors })),
            UnderNode(&root_id),
        ).unwrap();
        for player_index in 0..9 {
            tree.insert(
                Node::new(Box::new(PlayerMenu { player_index })),
                UnderNode(&game_launcher),
            );
            tree.insert(
                Node::new(Box::new(PlayerGrabInput { player_index })),
                UnderNode(&game_launcher),
            );
        }
        tree.insert(
            Node::new(Box::new(PlayerGrabEmulatorButtons)),
            UnderNode(&game_launcher),
        );

        tree
    }

    pub fn render(&mut self, node_ids: &Vec<NodeId>) {
        debug!("rerender");

        let state = self.store.get_state();
        for node in node_ids {
            let entity = self.tree.get(&node).unwrap().data();

            entity.render(&mut self.app.canvas, state, &mut self.resources);
        }

        self.app.canvas.present();
    }

    pub fn apply_event(&mut self, event: Event, node_ids: &Vec<NodeId>) -> bool {
        for node in node_ids {
            let entity = self.tree.get(&node).unwrap().data();

            entity.apply_event(&event, &mut self.app, &mut self.store);
        }

        return self.store.process();
    }

    pub fn collect_entities(&self) -> Vec<NodeId> {
        let root_id = self.tree.root_node_id().unwrap().clone();
        let state = self.store.get_state();
        return OnlyActiveTraversal::new(&self.tree, root_id, state).collect();
    }

    pub fn run_loop(&mut self) -> Option<String> {
        debug!("looping over events...");
        let mut rerender = true;
        let mut event_pump = self.app.sdl_context.event_pump().unwrap();
        loop {
            let node_ids = self.collect_entities();

            if rerender {
                self.render(&node_ids);
            }

            let event = event_pump.wait_event();
            rerender = self.apply_event(event, &node_ids);

            if !self.app.is_running() {
                break;
            }
        }

        None
    }
}

impl Drop for Meldnafen {
    fn drop(&mut self) {
        info!("exiting...");
        if let Err(err) = save_state(&mut self.store) {
            error!("could not write to save_sate: {}", err);
        }
    }
}

fn save_state(store: &Store) -> Result<(), String> {
    let serialized_state = store.dump().map_err(|x| format!("{}", x))?;
    let mut file = File::create("state.json").map_err(|x| format!("{}", x))?;
    file.write_all(serialized_state.as_slice())
        .map_err(|x| format!("{}", x))?;

    Ok(())
}

pub struct OnlyActiveTraversal<'a> {
    tree: &'a Tree<Box<Entity>>,
    data: VecDeque<NodeId>,
    state: &'a State,
}

impl<'a> OnlyActiveTraversal<'a> {
    pub fn new(
        tree: &'a Tree<Box<Entity>>,
        node_id: NodeId,
        state: &'a State,
    ) -> OnlyActiveTraversal<'a> {
        let mut data = VecDeque::with_capacity(ENTITES);

        data.push_front(node_id);

        OnlyActiveTraversal { tree, data, state }
    }
}

impl<'a> Iterator for OnlyActiveTraversal<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        self.data.pop_front().and_then(|node_id| {
            let node_ref = self.tree.get(&node_id).unwrap();

            for child_id in node_ref.children().iter().rev() {
                if self.tree
                    .get(child_id)
                    .ok()
                    .map_or(false, |x| x.data().is_active(self.state))
                {
                    self.data.push_front(child_id.clone());
                }
            }

            Some(node_id)
        })
    }
}
