use sdl2;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowBuildError, WindowContext};
use sdl2::image::LoadTexture;
use sdl2::joystick::Joystick;
use std::collections::HashMap;

pub struct App {
    pub sdl_context: sdl2::Sdl,
    pub joystick: sdl2::JoystickSubsystem,
    running: bool,
    pub canvas: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,
    opened_joysticks: HashMap<i32, Joystick>,
}

impl App {
    pub fn new<F>(build_window: F) -> App
    where
        F: Fn(sdl2::VideoSubsystem) -> Result<Window, WindowBuildError>,
    {
        debug!("initializing SDL2...");
        sdl2::image::init(sdl2::image::INIT_PNG).unwrap();
        let sdl_context = sdl2::init().unwrap();
        let video = sdl_context.video().unwrap();
        let joystick = sdl_context.joystick().unwrap();
        let canvas = build_window(video).unwrap().into_canvas().build().unwrap();
        let texture_creator = canvas.texture_creator();

        return App {
            sdl_context,
            joystick,
            running: true,
            canvas,
            texture_creator,
            opened_joysticks: HashMap::new(),
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

    pub fn open_joystick(&mut self, which: u32) {
        match self.joystick.open(which) {
            Ok(x) => {
                info!("added new joystick: {}", x.name());
                self.opened_joysticks.insert(x.instance_id(), x);
            }
            Err(err) => error!("could not open joystick: {}", err),
        }
    }

    pub fn get_joystick(&self, which: i32) -> &Joystick {
        self.opened_joysticks
            .get(&which)
            .expect("received an invalid joystick instance id")
    }
}
