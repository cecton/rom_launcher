use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::image::LoadTexture;

pub struct App {
    pub canvas: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,
}

impl App {
    pub fn new(canvas: Canvas<Window>) -> App {
        let texture_creator = canvas.texture_creator();

        return App {
            canvas: canvas,
            texture_creator: texture_creator,
        };
    }

    pub fn load_texture(&mut self, filepath: &str) -> Texture {
        return self.texture_creator
            .load_texture(filepath)
            .expect(format!("Couldn't load texture file: {}", filepath).as_ref());
    }
}
