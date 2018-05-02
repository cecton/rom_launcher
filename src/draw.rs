use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::rect::Rect;
use num_traits::cast::ToPrimitive;

pub struct Font {
    texture: Texture,
    chars: Vec<char>,
    width: u32,
    height: u32,
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
        }
    }

    #[allow(unused_must_use)]
    pub fn print(&self, canvas: &mut Canvas<Window>, x: i32, y: i32, text: &str) {
        let mut src = Rect::new(0, 0, self.width, self.height);
        let mut dst = Rect::new(x, y, self.width, self.height);
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
    }
}
