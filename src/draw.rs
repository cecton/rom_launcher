use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

pub struct Font {
    pub texture: Texture,
    pub chars: Vec<char>,
    pub w: i32,
    pub h: i32,
    pub x: i32,
    pub y: i32,
    pub line_start: i32,
    pub line_height: i32,
}

impl Font {
    pub fn new(texture: Texture, string_chars: &str) -> Font {
        let chars: Vec<char> = string_chars.chars().collect();
        let query = texture.query();
        let w = query.width as i32 / chars.len() as i32;
        let h = query.height as i32;

        Font {
            texture,
            chars,
            w,
            h,
            x: 0,
            y: 0,
            line_start: 0,
            line_height: h,
        }
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.line_start = x;
    }

    pub fn set_line_spacing(&mut self, line_spacing: f32) {
        self.line_height = (self.h as f32 * line_spacing) as i32;
    }

    #[allow(unused_must_use)]
    fn _print(&self, canvas: &mut Canvas<Window>, text: &str) -> (i32, i32) {
        let mut src = Rect::new(0, 0, self.w as u32, self.h as u32);
        let mut dst = Rect::new(self.x, self.y, self.w as u32, self.h as u32);
        for c in text.chars() {
            match self.chars.iter().enumerate().find(|&(_, x)| x == &c) {
                Some((i, _)) => {
                    src.set_x(i as i32 * self.w);
                    canvas.copy(&self.texture, src, dst);
                    let left = dst.left();
                    dst.set_x(left + self.w);
                }
                None => {
                    if c == '\n' {
                        dst.set_x(self.x);
                        dst.set_y(self.y + self.line_height);
                    }
                }
            }
        }

        (dst.left(), dst.top())
    }

    #[allow(dead_code)]
    pub fn print(&mut self, canvas: &mut Canvas<Window>, text: &str) {
        let (x, y) = self._print(canvas, text);
        self.x = x;
        self.y = y;
    }

    #[allow(dead_code)]
    pub fn println(&mut self, canvas: &mut Canvas<Window>, text: &str) {
        let (_, y) = self._print(canvas, text);
        self.x = self.line_start;
        self.y = y + self.line_height;
    }
}
