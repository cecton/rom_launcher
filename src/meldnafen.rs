use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use id_tree::*;
use num_traits::cast::ToPrimitive;
use std::cmp;

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
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 0, 0));
        canvas.draw_rect(Rect::new(0, 0, 15, 15));
        resources.font.set_line_spacing(0.75);
        resources.font.set_pos(0, 0);
        resources.font.texture.set_color_mod(255, 255, 255);
        resources
            .font
            .println(canvas, &format!("< {} >", state.emulator.name));
        resources.font.println(canvas, "");
        for rom in &state.roms {
            resources.font.println(canvas, &rom);
        }
        resources.font.println(canvas, "");
        resources
            .font
            .println(canvas, &format!("Page x of y ({} roms)", &state.roms.len()));
        resources.font.texture.set_color_mod(255, 0, 0);
        resources.font.println(canvas, "Hello\nWorld!");
    }

    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool {
        match event {
            &Event::Quit { .. }
            | &Event::KeyDown {
                keycode: Some(Keycode::Q),
                ..
            } => {
                app.quit();
            }
            _ => {}
        }

        return true;
    }
}

struct Resources {
    font: Font,
}

pub struct Meldnafen<'a> {
    pub app: App,
    store: Store<'a>,
    resources: Resources,
    tree: Tree<Box<Entity>>,
}

impl<'a> Meldnafen<'a> {
    pub fn new(canvas: Canvas<Window>) -> Meldnafen<'a> {
        let mut app = App::new(canvas);
        let mut store = Store::new(vec![Box::new(trigger_middleware)]);
        store.dispatch(Action::Initialize {});

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

        return Meldnafen {
            app,
            store,
            resources,
            tree,
        };
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

        return result;
    }
}
