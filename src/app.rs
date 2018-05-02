use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::image::LoadTexture;

pub struct App {
    running: bool,
    pub canvas: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,
}

impl App {
    pub fn new(canvas: Canvas<Window>) -> App {
        let texture_creator = canvas.texture_creator();

        return App {
            running: true,
            canvas: canvas,
            texture_creator: texture_creator,
        };
    }

    pub fn load_texture(&self, filepath: &str) -> Texture {
        return self.texture_creator
            .load_texture(filepath)
            .expect(format!("Couldn't load texture file: {}", filepath).as_ref());
    }

    pub fn is_running(&self) -> bool {
        return self.running;
    }

    pub fn quit(&mut self) {
        debug!("termination requested");
        self.running = false;
    }
}
