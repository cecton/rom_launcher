use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use num_traits::cast::ToPrimitive;

use app::App;
use store::{Store, Action};

const CHARS: [char; 81] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '!', '?', '(', ')', '[', ']', '<', '>', '~', '-', '_', '+', '@', ':',
    '/', '\'', '.', ',', ' ',
];
const FONT_WIDTH: u32 = 6;
const FONT_HEIGHT: u32 = 15;

pub struct Meldnafen {
    app: App,
    pub running: bool,
    store: Store,
    font: Texture,
}

impl Meldnafen {
    pub fn new(canvas: Canvas<Window>) -> Meldnafen {
        let mut app = App::new(canvas);
        let mut store = Store::new();
        store.dispatch(Action::Initialize {});
        debug!("setting up canvas and loading resources...");
        app.canvas.set_scale(3.0, 3.0).unwrap();
        let font = app.load_texture("font-12.png");

        return Meldnafen {
            app: app,
            running: true,
            store: store,
            font: font,
        };
    }

    #[allow(unused_must_use)]
    pub fn render(&mut self) {
        self.app.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.app.canvas.clear();
        self.app.canvas.set_draw_color(Color::RGB(255, 0, 0));
        self.app.canvas.draw_rect(Rect::new(0, 0, 10, 10));
        self.print(16, 16, "Hello World!");
        self.app.canvas.present();
    }

    pub fn apply_event(&mut self, event: Event) -> bool {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Q),
                ..
            } => {
                self.running = false;
                debug!("termination requested");
            }
            _ => {}
        }

        return true;
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
