use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::joystick::Joystick;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{DisplayMode, Window, WindowBuildError, WindowContext};
use std::collections::HashMap;

use crate::joystick::*;

pub struct App {
    pub sdl_context: sdl2::Sdl,
    pub joystick: sdl2::JoystickSubsystem,
    pub timer: sdl2::TimerSubsystem,
    running: bool,
    pub canvas: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,
    opened_joysticks: HashMap<i32, Joystick>,
    pub display_mode: DisplayMode,
    event_pump: sdl2::EventPump,
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
        let display_mode = video.desktop_display_mode(0).unwrap();
        let joystick = sdl_context.joystick().unwrap();
        let timer = sdl_context.timer().unwrap();
        let canvas = build_window(video).unwrap().into_canvas().build().unwrap();
        let texture_creator = canvas.texture_creator();
        let event_pump = sdl_context.event_pump().unwrap();

        App {
            sdl_context,
            joystick,
            timer,
            running: true,
            canvas,
            texture_creator,
            opened_joysticks: HashMap::new(),
            display_mode,
            event_pump,
        }
    }

    pub fn load_texture(&self, filepath: &str) -> Texture {
        self.texture_creator
            .load_texture(filepath)
            .unwrap_or_else(|_| panic!("Couldn't load texture file: {}", filepath))
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn quit(&mut self) {
        debug!("termination requested");
        self.running = false;
    }

    pub fn open_joystick(&mut self, which: u32) -> Option<JoystickInfo> {
        match self.joystick.open(which) {
            Ok(joystick) => {
                let id = joystick.instance_id();
                info!(
                    "added new joystick: {} ({} buttons, {} axes, {} hats)",
                    joystick.name(),
                    joystick.num_buttons(),
                    joystick.num_axes(),
                    joystick.num_hats()
                );
                let index = self
                    .opened_joysticks
                    .values()
                    .filter(|x| x.guid() == joystick.guid() && x.attached())
                    .count();
                let joystick_info = JoystickInfo::new(&joystick, index);
                debug!("new joystick info: {:?}", joystick_info);
                self.opened_joysticks.insert(id, joystick);
                Some(joystick_info)
            }
            Err(err) => {
                error!("could not open joystick: {}", err);
                None
            }
        }
    }

    pub fn close_joystick(&mut self, which: i32) {
        self.opened_joysticks.remove(&which);
        info!("removed joystick");
    }

    pub fn wait_event(&mut self) -> Event {
        self.event_pump.wait_event()
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        self.event_pump.poll_event()
    }
}
