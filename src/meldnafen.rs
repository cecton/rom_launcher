use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use num_traits::cast::ToPrimitive;
use id_tree::*;

use app::App;
use store::{Action, State, Store};

const CHARS: [char; 81] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '!', '?', '(', ')', '[', ']', '<', '>', '~', '-', '_', '+', '@', ':',
    '/', '\'', '.', ',', ' ',
];
const FONT_WIDTH: u32 = 6;
const FONT_HEIGHT: u32 = 15;

#[derive(Debug)]
struct List {}

impl List {
    #[allow(unused_must_use)]
    fn render(&self, canvas: &mut Canvas<Window>, state: &State) {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 0, 0));
        canvas.draw_rect(Rect::new(0, 0, 10, 10));
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

pub struct Meldnafen {
    pub app: App,
    store: Store,
    font: Texture,

    tree: Tree<List>,
    root_id: NodeId,
}

impl Meldnafen {
    pub fn new(canvas: Canvas<Window>) -> Meldnafen {
        let mut app = App::new(canvas);
        let mut store = Store::new();
        store.dispatch(Action::Initialize {});
        debug!("setting up canvas and loading resources...");
        app.canvas.set_scale(3.0, 3.0).unwrap();
        let font = app.load_texture("font-12.png");

        use id_tree::InsertBehavior::*;
        let mut tree: Tree<List> = TreeBuilder::new().with_node_capacity(1).build();
        let root_id: NodeId = tree.insert(Node::new(List {}), AsRoot).unwrap();

        return Meldnafen {
            app: app,
            store: store,
            font: font,

            tree: tree,
            root_id: root_id,
        };
    }

    pub fn render(&mut self) {
        for node in self.tree.traverse_pre_order(&self.root_id).unwrap() {
            node.data()
                .render(&mut self.app.canvas, self.store.get_state());
        }
        self.print(16, 16, "Hello World!");

        self.app.canvas.present();
    }

    pub fn apply_event(&mut self, event: Event) -> bool {
        let mut result = false;
        for node in self.tree.traverse_pre_order(&self.root_id).unwrap() {
            result = result
                || node.data()
                    .apply_event(&event, &mut self.app, &mut self.store);
        }

        return result;
    }

    #[allow(unused_must_use)]
    pub fn print(&mut self, x: i32, y: i32, text: &str) {
        let mut src = Rect::new(0, 0, FONT_WIDTH, FONT_HEIGHT);
        let mut dst = Rect::new(x, y, FONT_WIDTH, FONT_HEIGHT);
        let font_width = FONT_WIDTH.to_i32().unwrap();
        for c in text.chars() {
            match CHARS.iter().enumerate().find(|&(_, x)| x == &c) {
                Some((i, _)) => {
                    src.set_x(i.to_i32().unwrap() * font_width);
                    self.app.canvas.copy(&self.font, src, dst);
                    let left = dst.left();
                    dst.set_x(left + font_width);
                }
                _ => {}
            }
        }
    }
}
