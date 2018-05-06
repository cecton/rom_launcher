use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use id_tree::*;
use num_traits::cast::ToPrimitive;
use std::cmp;
use std::fs::File;
use std::io::prelude::*;
use std::io;

use app::*;
use store::*;
use draw::*;

const TV_XRES: i32 = 256;
const TV_YRES: i32 = 224;

trait Entity {
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &mut Resources);
    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool;
}

#[derive(Debug)]
struct List {}

impl Entity for List {
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
                for (i, rom) in roms.iter().enumerate() {
                    if i as i32 == state.rom_selected {
                        resources.font.texture.set_color_mod(255, 255, 0);
                    }
                    resources.font.println(canvas, &rom.name);
                    if i as i32 == state.rom_selected {
                        resources.font.texture.set_color_mod(255, 255, 255);
                    }
                }
                resources.font.println(canvas, "");
                resources
                    .font
                    .println(canvas, &format!("Page x of y ({} roms)", roms.len()));
            }
            &Err(ref err) => {
                resources.font.texture.set_color_mod(255, 0, 0);
                resources.font.println(canvas, &err);
            }
        }
    }

    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool {
        let rom_selected = store.get_state().rom_selected;

        match *event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Q),
                ..
            } => app.quit(),
            Event::KeyUp {
                keycode: Some(Keycode::Down),
                ..
            } => store.dispatch(Action::NextRom { step: 1 }),
            Event::KeyUp {
                keycode: Some(Keycode::Up),
                ..
            } => store.dispatch(Action::NextRom { step: -1 }),
            Event::KeyUp {
                keycode: Some(Keycode::Right),
                ..
            } => if rom_selected == -1 {
                store.dispatch(Action::NextEmulator { step: 1 })
            },
            Event::KeyUp {
                keycode: Some(Keycode::Left),
                ..
            } => if rom_selected == -1 {
                store.dispatch(Action::NextEmulator { step: -1 })
            },
            _ => return false,
        }

        return true;
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
    pub fn new(canvas: Canvas<Window>) -> Meldnafen {
        let mut app = App::new(canvas);
        let mut store = Store::new();
        if let Err(err) = Meldnafen::load_state(&mut store) {
            error!("could not load state: {}", err);
        }

        debug!("setting up canvas...");
        let (w, h) = app.canvas.output_size().unwrap();
        let zoom = cmp::min(w as i32 / TV_XRES, h as i32 / TV_YRES)
            .to_f32()
            .unwrap();
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

        let mut tree: Tree<Box<Entity>> = TreeBuilder::new().with_node_capacity(1).build();
        tree.insert(Node::new(Box::new(List {})), AsRoot);

        tree
    }

    pub fn render(&mut self) {
        debug!("rerender");
        self.app.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.app.canvas.clear();

        let root_id = self.tree.root_node_id().unwrap();
        for node in self.tree.traverse_pre_order(&root_id).unwrap() {
            node.data().render(
                &mut self.app.canvas,
                self.store.get_state(),
                &mut self.resources,
            );
        }

        self.app.canvas.present();
    }

    pub fn apply_event(&mut self, event: Event) -> bool {
        let mut result = false;
        let root_id = self.tree.root_node_id().unwrap();
        for node in self.tree.traverse_pre_order(&root_id).unwrap() {
            result = result
                || node.data()
                    .apply_event(&event, &mut self.app, &mut self.store);
        }
        self.store.process();

        return result;
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
