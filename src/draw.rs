use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::rect::Rect;
use num_traits::cast::ToPrimitive;

pub struct Font {
    texture: Texture,
    chars: Vec<char>,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    line_spacing: f32,
}

impl Font {
    pub fn new(texture: Texture, string_chars: &str) -> Font {
        let chars: Vec<char> = string_chars.chars().collect();
        let query = texture.query();
        let width = query.width / chars.len().to_u32().unwrap();
        let height = query.height;

        Font {
            texture,
            chars,
            width,
            height,
            x: 0,
            y: 0,
            line_spacing: 1.0,
        }
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_line_spacing(&mut self, line_spacing: f32) {
        self.line_spacing = line_spacing;
    }

    #[allow(unused_must_use)]
    fn _write(&self, canvas: &mut Canvas<Window>, text: &str) -> (i32, i32) {
        let mut src = Rect::new(0, 0, self.width, self.height);
        let mut dst = Rect::new(self.x, self.y, self.width, self.height);
        let font_width = self.width.to_i32().unwrap();
        for c in text.chars() {
            match self.chars.iter().enumerate().find(|&(_, x)| x == &c) {
                Some((i, _)) => {
                    src.set_x(i.to_i32().unwrap() * font_width);
                    canvas.copy(&self.texture, src, dst);
                    let left = dst.left();
                    dst.set_x(left + font_width);
                }
                _ => {}
            }
        }

        (dst.left(), dst.top())
    }

    pub fn print(&mut self, canvas: &mut Canvas<Window>, text: &str) {
        let (x, y) = self._write(canvas, text);
        self.x = x;
        self.y = y;
    }

    pub fn println(&mut self, canvas: &mut Canvas<Window>, text: &str) {
        let (_, y) = self._write(canvas, text);
        self.y = y
            + (self.height.to_f32().unwrap() * self.line_spacing)
                .to_i32()
                .unwrap();
    }
}
