use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::joystick::HatState;
use id_tree::*;
use std::cmp;
use std::fs::File;
use std::io::prelude::*;
use std::io;

use app::*;
use store::*;
use draw::*;

const TV_XRES: i32 = 256;
const TV_YRES: i32 = 224;

macro_rules! set_highlight {
    ($font:expr, $value:expr) => {
        if $value {
            $font.texture.set_color_mod(255, 255, 0);
        } else {
            $font.texture.set_color_mod(255, 255, 255);
        }
    }
}

trait Entity {
    fn is_active(&self, _state: &State) -> bool;
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources);
    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool;
}

#[derive(Debug)]
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

    fn apply_event(&self, event: &Event, _app: &mut App, store: &mut Store) -> bool {
        let rom_selected = store.get_state().rom_selected;

        match *event {
            Event::KeyUp {
                keycode: Some(Keycode::Down),
                ..
            }
            | Event::JoyHatMotion {
                state: HatState::Down,
                ..
            } => store.dispatch(Action::NextRom { step: 1 }),
            Event::KeyUp {
                keycode: Some(Keycode::Up),
                ..
            }
            | Event::JoyHatMotion {
                state: HatState::Up,
                ..
            } => store.dispatch(Action::NextRom { step: -1 }),
            Event::KeyUp {
                keycode: Some(Keycode::Right),
                ..
            }
            | Event::JoyHatMotion {
                state: HatState::Right,
                ..
            } => if rom_selected == -1 {
                store.dispatch(Action::NextEmulator { step: 1 })
            } else {
                store.dispatch(Action::NextPage { step: 1 })
            },
            Event::KeyUp {
                keycode: Some(Keycode::Left),
                ..
            }
            | Event::JoyHatMotion {
                state: HatState::Left,
                ..
            } => if rom_selected == -1 {
                store.dispatch(Action::NextEmulator { step: -1 })
            } else {
                store.dispatch(Action::NextPage { step: -1 })
            },
            Event::KeyUp {
                keycode: Some(Keycode::Return),
                ..
            }
            | Event::JoyButtonUp { button_idx: 0, .. } => if rom_selected > -1 {
                store.dispatch(Action::LaunchGame {})
            } else {
                return false;
            },
            _ => return false,
        }

        true
    }
}

#[derive(Debug)]
struct GameLauncher {}

impl Entity for GameLauncher {
    fn is_active(&self, state: &State) -> bool {
        state.screen == Screen::GameLauncher
    }

    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources) {
        resources.font.set_line_spacing(0.75);
        resources.font.set_pos(0, 0);

        let mut i = 0;
        for maybe_player in state.players.iter() {
            match *maybe_player {
                Some(player) => {
                    i += 1;
                    resources.font.texture.set_color_mod(255, 255, 255);
                    resources.font.print(canvas, &format!("{:2}:", i));
                    set_highlight!(resources.font, player.menu == PlayerMenu::Controls);
                    resources.font.print(canvas, "  Controls");
                    set_highlight!(resources.font, player.menu == PlayerMenu::Ready);
                    resources.font.print(canvas, "  Ready");
                    set_highlight!(resources.font, player.menu == PlayerMenu::Leave);
                    resources.font.print(canvas, "            Leave");
                }
                None => {}
            }
            resources.font.println(canvas, "\n");
        }

        if i == 0 {
            resources.font.set_pos(0, 0);
            resources
                .font
                .println(canvas, "Press the first button to add a player");
        }
    }

    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool {
        match *event {
            Event::JoyButtonUp {
                which,
                button_idx: 0,
                ..
            } => store.dispatch(Action::AddPlayer(which)),
            Event::JoyHatMotion {
                which,
                state: HatState::Right,
                ..
            } => store.dispatch(Action::NextPlayerMenu(which)),
            Event::JoyHatMotion {
                which,
                state: HatState::Left,
                ..
            } => store.dispatch(Action::PrevPlayerMenu(which)),
            _ => return false,
        }

        true
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

    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool {
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
            Event::JoyDeviceAdded { which, .. } => app.open_joystick(which),
            _ => {}
        }

        false
    }
}

struct Resources {
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
        if let Err(err) = Meldnafen::load_state(&mut store) {
            error!("could not load state: {}", err);
        }

        debug!("setting up canvas...");
        let (w, h) = app.canvas.output_size().unwrap();
        let zoom = cmp::min(w as i32 / TV_XRES, h as i32 / TV_YRES) as f32;
        app.canvas.set_scale(zoom, zoom).unwrap();

        let mut viewport = app.canvas.viewport();
        viewport.x = (viewport.w - TV_XRES) / 2;
        viewport.y = (viewport.h - TV_YRES) / 2;
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
        let mut file = File::open("save_state")?;
        let mut serialized_state: Vec<u8> = vec![];
        file.read_to_end(&mut serialized_state)?;
        store.load(&serialized_state);

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

        let mut tree: Tree<Box<Entity>> = TreeBuilder::new().with_node_capacity(3).build();
        let root_id = tree.insert(Node::new(Box::new(Root {})), AsRoot).unwrap();
        tree.insert(Node::new(Box::new(List {})), UnderNode(&root_id));
        tree.insert(Node::new(Box::new(GameLauncher {})), UnderNode(&root_id));

        tree
    }

    pub fn render(&mut self) {
        debug!("rerender");

        let root_id = self.tree.root_node_id().unwrap();
        for node in self.tree.traverse_pre_order(&root_id).unwrap() {
            let entity = node.data();
            let state = self.store.get_state();
            if !entity.is_active(state) {
                continue;
            }

            entity.render(&mut self.app.canvas, state, &mut self.resources);
        }

        self.app.canvas.present();
    }

    pub fn apply_event(&mut self, event: Event) -> bool {
        let mut result = false;
        let root_id = self.tree.root_node_id().unwrap();
        for node in self.tree.traverse_pre_order(&root_id).unwrap() {
            let entity = node.data();
            {
                let state = self.store.get_state();
                if !entity.is_active(state) {
                    continue;
                }
            }

            result = result || entity.apply_event(&event, &mut self.app, &mut self.store);
        }
        self.store.process();

        return result;
    }

    pub fn run_loop(&mut self) {
        debug!("looping over events...");
        let mut event_pump = self.app.sdl_context.event_pump().unwrap();
        'running: for event in event_pump.wait_iter() {
            let rerender = self.apply_event(event);

            if !self.app.is_running() {
                break 'running;
            }

            if rerender {
                self.render();
            }
        }
    }
}

impl Drop for Meldnafen {
    fn drop(&mut self) {
        info!("exiting...");
        debug!("saving state: {:?}", self.store.get_state());
        if let Err(err) = save_state(&mut self.store) {
            error!("could not write to save_sate: {}", err);
        }
    }
}

fn save_state(store: &Store) -> Result<(), io::Error> {
    let serialized_state = store.dump().expect("could not serialize state");
    let mut file = File::create("save_state")?;
    file.write_all(serialized_state.as_slice())?;

    Ok(())
}
