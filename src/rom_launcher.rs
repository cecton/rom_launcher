use id_tree::*;
use sdl2::event::Event;
use sdl2::joystick::HatState;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::cmp;
use std::collections::{HashMap, VecDeque};
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
const JOYSTICK_LOCK_TIME: u32 = 200; // TODO: longer lock?
const JOYSTICK_LOCK_TIME_AXIS: u32 = 400; // TODO: longer lock?

macro_rules! set_highlight {
    ($canvas:expr, $font:expr, $value:expr, $text:expr) => {
        if $value {
            $font.print($canvas, "> ");
            $font.print($canvas, $text);
            $font.print($canvas, " <");
        } else {
            $font.print($canvas, "  ");
            $font.print($canvas, $text);
            $font.print($canvas, "  ");
        }
    };
}

macro_rules! lock_joystick {
    ($joystick:expr, $split:expr, $timestamp:expr, $store:expr, $closure:expr) => {
        if $timestamp
            >= $store
                .get_state()
                .last_joystick_action
                .get(&($joystick, $split))
                .or(Some(&0))
                .unwrap()
                + JOYSTICK_LOCK_TIME
        {
            $store.dispatch(Action::UpdateJoystickLastAction(
                $timestamp, $joystick, $split,
            ));
            $closure();
        }
    };
}

macro_rules! lock_joystick_axis {
    ($joystick:expr, $split:expr, $timestamp:expr, $store:expr, $closure:expr) => {
        if $timestamp
            >= $store
                .get_state()
                .last_joystick_action
                .get(&($joystick, $split))
                .or(Some(&0))
                .unwrap()
                + JOYSTICK_LOCK_TIME_AXIS
        {
            $store.dispatch(Action::UpdateJoystickLastAction(
                $timestamp, $joystick, $split,
            ));
            $closure();
        }
    };
}

macro_rules! translate_to_retroarch_button {
    ($event:expr) => {
        match *$event {
            Button(x) => format!("btn = {}", x),
            Hat(x, ref state) => format!(
                "btn = h{}{}",
                x,
                match *state {
                    HatState::Up => "up",
                    HatState::Down => "down",
                    HatState::Left => "left",
                    HatState::Right => "right",
                }
            ),
            Axis(x, ref state) => match *state {
                AxisState::Positive => format!("axis = +{}", x),
                AxisState::Negative => format!("axis = -{}", x),
            },
            Unassigned => panic!(),
        }
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
            .println(canvas, &format!("< {: ^38} >", state.get_emulator().name));
        resources.font.println(canvas, "");

        resources.font.texture.set_color_mod(255, 255, 255);
        match &state.roms {
            &Ok(ref roms) => {
                for (i, rom) in roms
                    .iter()
                    .skip((state.page_index * PAGE_SIZE) as usize)
                    .take(PAGE_SIZE as usize)
                    .enumerate()
                {
                    if i as i32 == state.rom_selected {
                        resources.font.texture.set_color_mod(255, 255, 0);
                    }
                    if rom.name.len() > 39 {
                        resources
                            .font
                            .println(canvas, &format!("{}...", &rom.name[..39]));
                    } else {
                        resources.font.println(canvas, &rom.name);
                    }
                    if i as i32 == state.rom_selected {
                        resources.font.texture.set_color_mod(255, 255, 255);
                    }
                }
                for _ in 0..(PAGE_SIZE - (roms.len() as i32 - state.page_index * PAGE_SIZE)) {
                    resources.font.println(canvas, "");
                }

                resources.font.println(canvas, "");
                resources.font.println(
                    canvas,
                    &format!(
                        "{: >42}",
                        &format!(
                            "Page {} of {} ({} roms)",
                            state.page_index + 1,
                            state.page_count,
                            roms.len()
                        )
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
                hat_idx,
                timestamp,
                ..
            } => {
                let split_index = hat_idx as u32;

                lock_joystick!(which, split_index, timestamp, store, || store
                    .dispatch(NextRom { timestamp, step: 1 }))
            }
            Event::JoyAxisMotion {
                axis_idx,
                value,
                which,
                timestamp,
                ..
            }
                if axis_idx % 2 == 1 && value >= AXIS_THRESOLD =>
            {
                let split_index = axis_idx as u32 / 2;

                lock_joystick_axis!(which, split_index, timestamp, store, || store
                    .dispatch(NextRom { timestamp, step: 1 }))
            }
            Event::JoyHatMotion {
                state: HatState::Up,
                which,
                hat_idx,
                timestamp,
                ..
            } => {
                let split_index = hat_idx as u32;

                lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
                    NextRom {
                        timestamp,
                        step: -1
                    }
                ))
            }
            Event::JoyAxisMotion {
                axis_idx,
                value,
                which,
                timestamp,
                ..
            }
                if axis_idx % 2 == 1 && value <= -AXIS_THRESOLD =>
            {
                let split_index = axis_idx as u32 / 2;

                lock_joystick_axis!(which, split_index, timestamp, store, || store.dispatch(
                    NextRom {
                        timestamp,
                        step: -1
                    }
                ))
            }
            Event::JoyHatMotion {
                state: HatState::Right,
                which,
                hat_idx,
                timestamp,
                ..
            } => {
                let split_index = hat_idx as u32;

                if rom_selected == -1 {
                    lock_joystick!(which, split_index, timestamp, store, || store
                        .dispatch(NextEmulator { timestamp, step: 1 }))
                } else {
                    lock_joystick!(which, split_index, timestamp, store, || store
                        .dispatch(NextPage { timestamp, step: 1 }))
                }
            }
            Event::JoyAxisMotion {
                axis_idx,
                value,
                which,
                timestamp,
                ..
            }
                if axis_idx % 2 == 0 && value >= AXIS_THRESOLD =>
            {
                let split_index = axis_idx as u32 / 2;

                if rom_selected == -1 {
                    lock_joystick!(which, split_index, timestamp, store, || store
                        .dispatch(NextEmulator { timestamp, step: 1 }))
                } else {
                    lock_joystick!(which, split_index, timestamp, store, || store
                        .dispatch(NextPage { timestamp, step: 1 }))
                }
            }
            Event::JoyHatMotion {
                state: HatState::Left,
                which,
                hat_idx,
                timestamp,
                ..
            } => {
                let split_index = hat_idx as u32;

                if rom_selected == -1 {
                    lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
                        NextEmulator {
                            timestamp,
                            step: -1
                        }
                    ))
                } else {
                    lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
                        NextPage {
                            timestamp,
                            step: -1
                        }
                    ))
                }
            }
            Event::JoyAxisMotion {
                axis_idx,
                value,
                which,
                timestamp,
                ..
            }
                if axis_idx % 2 == 0 && value <= -AXIS_THRESOLD =>
            {
                let split_index = axis_idx as u32 / 2;

                if rom_selected == -1 {
                    lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
                        NextEmulator {
                            timestamp,
                            step: -1
                        }
                    ))
                } else {
                    lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
                        NextPage {
                            timestamp,
                            step: -1
                        }
                    ))
                }
            }
            Event::JoyButtonUp {
                button_idx,
                which,
                timestamp,
                ..
            } => {
                let split_index = {
                    let state = store.get_state();
                    let split_value =
                        state.joysticks[&which].buttons / state.joysticks[&which].split;

                    if button_idx as u32 % split_value == 0 {
                        Some(button_idx as u32 / split_value)
                    } else {
                        None
                    }
                };

                if split_index.is_some() && rom_selected > -1 {
                    store.dispatch(LaunchGame(timestamp, which, split_index.unwrap()))
                }
            }
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
                button_idx,
                timestamp,
                ..
            } => {
                let split_index = {
                    let state = store.get_state();
                    let split_value =
                        state.joysticks[&which].buttons / state.joysticks[&which].split;

                    if button_idx as u32 % split_value == 0 {
                        Some(button_idx as u32 / split_value)
                    } else {
                        None
                    }
                };

                if let Some(split_index) = split_index {
                    if !store
                        .get_state()
                        .players
                        .iter()
                        .filter(|x| x.is_some())
                        .map(|x| x.as_ref().unwrap())
                        .any(|x| x.joystick == which && x.joystick_split == split_index)
                    {
                        store.dispatch(AddPlayer(timestamp, which, split_index));
                    }
                }
            }
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
            .print(canvas, &format!("{:2} ", actual_player_index + 1));

        match player.menu {
            Controls | Ready | Leave => {
                set_highlight!(canvas, resources.font, player.menu == Controls, "Controls");
                if (self.player_index == 0 && !state.all_players_are_ready())
                    || state.player_needs_setup_controls(self.player_index)
                {
                    resources.font.texture.set_color_mod(0, 0, 0);
                }
                set_highlight!(
                    canvas,
                    resources.font,
                    player.menu == Ready,
                    if self.player_index == 0 {
                        "Start"
                    } else {
                        "Ready"
                    }
                );
                resources.font.print(canvas, "        ");
                resources.font.texture.set_color_mod(255, 255, 255);
                set_highlight!(
                    canvas,
                    resources.font,
                    player.menu == Leave,
                    if self.player_index == 0 {
                        "Exit"
                    } else {
                        "Leave"
                    }
                );
            }
            ConsoleControls | GameControls | ClearConsoleControls | ControlsExit => {
                set_highlight!(
                    canvas,
                    resources.font,
                    player.menu == ConsoleControls,
                    "Console"
                );
                set_highlight!(canvas, resources.font, player.menu == GameControls, "Game");
                if !state.player_has_game_controls(self.player_index) {
                    resources.font.texture.set_color_mod(0, 0, 0);
                }
                set_highlight!(
                    canvas,
                    resources.font,
                    player.menu == ClearConsoleControls,
                    "Clear"
                );
                resources.font.print(canvas, " ");
                resources.font.texture.set_color_mod(255, 255, 255);
                set_highlight!(canvas, resources.font, player.menu == ControlsExit, "Back");
            }
            Waiting => {
                resources.font.print(canvas, "   ");
                set_highlight!(canvas, resources.font, true, "Waiting for other players...");
            }
        }
    }

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) {
        use store::Action::*;

        let (player_joystick, player_split) = store.get_state().players[self.player_index]
            .as_ref()
            .map(|x| (x.joystick, x.joystick_split))
            .unwrap();
        match *event {
            Event::JoyButtonUp {
                which,
                button_idx,
                timestamp,
                ..
            }
                if player_joystick == which =>
            {
                let split_index = {
                    let state = store.get_state();
                    let split_value =
                        state.joysticks[&which].buttons / state.joysticks[&which].split;

                    if button_idx as u32 % split_value == 0 {
                        Some(button_idx as u32 / split_value)
                    } else {
                        None
                    }
                };

                if let Some(split_index) = split_index {
                    if player_split == split_index {
                        store.dispatch(GoPlayerMenu(timestamp, which, split_index));
                    }
                }
            }
            Event::JoyHatMotion {
                hat_idx,
                which,
                state: HatState::Right,
                timestamp,
                ..
            } => if player_joystick == which {
                let split_index = hat_idx as u32;

                if player_split == split_index {
                    store.dispatch(NextPlayerMenu(timestamp, which, split_index));
                }
            },
            Event::JoyAxisMotion {
                axis_idx,
                value,
                which,
                timestamp,
                ..
            }
                if axis_idx % 2 == 0 && value >= AXIS_THRESOLD && player_joystick == which =>
            {
                let split_index = axis_idx as u32 / 2;

                if player_split == split_index {
                    lock_joystick!(which, split_index, timestamp, store, || store
                        .dispatch(NextPlayerMenu(timestamp, which, split_index)))
                }
            }
            Event::JoyHatMotion {
                hat_idx,
                which,
                state: HatState::Left,
                timestamp,
                ..
            } => if player_joystick == which {
                let split_index = hat_idx as u32;

                if player_split == split_index {
                    store.dispatch(PrevPlayerMenu(timestamp, which, split_index));
                }
            },
            Event::JoyAxisMotion {
                axis_idx,
                value,
                which,
                timestamp,
                ..
            }
                if axis_idx % 2 == 0 && value <= -AXIS_THRESOLD && player_joystick == which =>
            {
                let split_index = axis_idx as u32 / 2;

                if player_split == split_index {
                    lock_joystick!(which, split_index, timestamp, store, || store
                        .dispatch(PrevPlayerMenu(timestamp, which, split_index)))
                }
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
        let &(_, ref controls) = state.players[self.player_index]
            .as_ref()
            .unwrap()
            .grab_input
            .as_ref()
            .unwrap();
        let (_, ref input_display) =
            state.emulators[state.emulator_selected as usize].controls[controls.len()];
        resources.font.print(
            canvas,
            &format!(
                "{:2}   Press button for:   {}",
                actual_player_index + 1,
                input_display
            ),
        );
    }

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) {
        use self::JoystickEvent::*;
        use store::Action::*;

        let (player_joystick, player_split) = store.get_state().players[self.player_index]
            .as_ref()
            .map(|x| (x.joystick, x.joystick_split))
            .unwrap();
        match *event {
            Event::JoyButtonUp {
                which,
                button_idx,
                timestamp,
                ..
            }
                if player_joystick == which =>
            {
                let split_index = {
                    let state = store.get_state();
                    let split_value =
                        state.joysticks[&which].buttons / state.joysticks[&which].split;

                    button_idx as u32 / split_value
                };

                if split_index == split_index {
                    lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
                        BindPlayerJoystickEvent(timestamp, self.player_index, Button(button_idx))
                    ))
                }
            }
            Event::JoyHatMotion {
                which,
                hat_idx,
                state,
                timestamp,
                ..
            }
                if player_joystick == which
                    && (state == HatState::Up
                        || state == HatState::Down
                        || state == HatState::Left
                        || state == HatState::Right) =>
            {
                let split_index = hat_idx as u32;

                if player_split == split_index {
                    lock_joystick!(which, split_index, timestamp, store, || store.dispatch(
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
            }
            Event::JoyAxisMotion {
                which,
                axis_idx,
                value,
                timestamp,
                ..
            }
                if player_joystick == which
                    && (value <= -AXIS_THRESOLD || value >= AXIS_THRESOLD) =>
            {
                let split_index = axis_idx as u32 / 2;

                if player_split == split_index {
                    lock_joystick_axis!(which, split_index, timestamp, store, || store.dispatch(
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

        let (player_joystick, player_split) = store.get_state().players[0]
            .as_ref()
            .map(|x| (x.joystick, x.joystick_split))
            .unwrap();
        match *event {
            Event::JoyButtonUp {
                which,
                button_idx,
                timestamp,
                ..
            }
                if player_joystick == which =>
            {
                let split_index = {
                    let state = store.get_state();
                    let split_value =
                        state.joysticks[&which].buttons / state.joysticks[&which].split;

                    button_idx as u32 / split_value
                };
                let hotkey = store.get_state().players[0]
                    .as_ref()
                    .unwrap()
                    .grab_emulator_buttons
                    .as_ref()
                    .unwrap()
                    .0
                    .clone();

                if player_split == split_index {
                    lock_joystick!(which, split_index, timestamp, store, || {
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
            } => {
                store.dispatch(Quit);
                app.quit();
            }
            Event::JoyDeviceAdded {
                which, timestamp, ..
            } => {
                if let Some(info) = app.open_joystick(which) {
                    // TODO: maybe restart the application after a joystick has
                    //       been detected to ensure the correct joystick order
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

pub struct ROMLauncher {
    pub app: App,
    store: Store,
    resources: Resources,
    tree: Tree<Box<Entity>>,
}

impl ROMLauncher {
    pub fn new(mut app: App) -> ROMLauncher {
        let mut store = Store::new();
        if let Err(err) = Self::load_state(&mut store) {
            error!("{}", err);
        }
        store.dispatch(Action::NextEmulator {
            timestamp: 0,
            step: 0,
        });

        let (w, h) = app.canvas.output_size().unwrap();
        let zoom = cmp::min(w as i32 / TV_XRES, h as i32 / TV_YRES) as f32;
        debug!("setting up canvas (w: {}, h: {}, zoom: {})...", w, h, zoom);
        app.canvas.set_scale(zoom, zoom).unwrap();

        let mut viewport = app.canvas.viewport();
        viewport.x = (viewport.w - TV_XRES) / 2;
        viewport.y = (viewport.h - TV_YRES) / 2;
        viewport.w = TV_XRES;
        viewport.h = TV_YRES;
        debug!(
            "viewport: {}x{}+{}+{}",
            viewport.w, viewport.h, viewport.x, viewport.y
        );
        app.canvas.set_viewport(viewport);

        let resources = Self::load_resources(&app);
        let tree = Self::load_entites();

        ROMLauncher {
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
        let game_launcher = tree
            .insert(
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

    pub fn run_loop(&mut self) -> Option<(Vec<String>, String, String)> {
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

        self.prepare_config()
    }

    pub fn prepare_config(&self) -> Option<(Vec<String>, String, String)> {
        use self::JoystickEvent::*;
        use store::{AxisState, HatState};
        let state = self.store.get_state();

        if state.screen == store::Screen::GameLauncher {
            let mut config = "".to_string();
            let emulator = state.get_emulator();
            let emulator_id = state.get_emulator().id.clone();
            let rom = state.get_rom().clone();

            let mut joystick_order = HashMap::new();
            let mut joystick_ids: Vec<_> = state.joysticks.keys().collect();
            joystick_ids.sort();
            for (i, joystick) in joystick_ids.into_iter().enumerate() {
                joystick_order.insert(joystick, i);
            }

            for (i, player) in state
                .players
                .iter()
                .filter(|x| x.is_some())
                .map(|x| x.as_ref().unwrap())
                .enumerate()
            {
                config.push_str(&format!(
                    "input_player{}_joypad_index = {}\n",
                    i + 1,
                    joystick_order.get(&player.joystick).unwrap()
                ));

                let guid = state.joysticks[&player.joystick].guid;
                for (event, control) in state
                    .game_configs
                    .get(&guid, &player.joystick_split, &rom.file_name)
                    .or_else(|| {
                        state
                            .console_configs
                            .get(&guid, &player.joystick_split, &emulator_id)
                    }).as_ref()
                    .unwrap()
                    .iter()
                    .zip(emulator.controls.iter().map(|&(ref x, _)| x))
                {
                    config.push_str(&match *event {
                        Unassigned => format!("// input_player{}_{} unassigned\n", i + 1, control),
                        _ => format!(
                            "input_player{}_{}_{}\n",
                            i + 1,
                            control,
                            translate_to_retroarch_button!(event)
                        ),
                    });
                }

                if let Some((Some(ref hotkey), Some(ref menu))) = player.grab_emulator_buttons {
                    config.push_str(&format!(
                        "input_enable_hotkey_{}\n",
                        translate_to_retroarch_button!(hotkey)
                    ));
                    config.push_str(&format!(
                        "input_menu_toggle_{}\n",
                        translate_to_retroarch_button!(menu)
                    ));
                }

                config.push_str("\n");
            }

            config.push_str("config_save_on_exit = false\n");

            Some((emulator.command.clone(), config, rom.path))
        } else {
            None
        }
    }
}

impl Drop for ROMLauncher {
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
                if self
                    .tree
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
