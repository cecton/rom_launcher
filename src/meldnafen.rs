use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use id_tree::*;

use app::App;
use store::{Action, State, Store};
use draw::Font;

trait Entity {
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &Resources);
    fn apply_event(&self, event: &Event, app: &mut App, store: &mut Store) -> bool;
}

#[derive(Debug)]
struct List {}

impl Entity for List {
    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State, resources: &Resources) {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 0, 0));
        canvas.draw_rect(Rect::new(0, 0, 10, 10));
        resources.font.print(canvas, 16, 16, "Hello World!");
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
        store.dispatch(Action::Initialize {});
        debug!("setting up canvas and loading resources...");
        app.canvas.set_scale(3.0, 3.0).unwrap();

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
                &self.resources,
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
