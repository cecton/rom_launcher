extern crate env_logger;
#[macro_use]
extern crate log;
extern crate num_traits;

use log::LevelFilter;
use env_logger::Builder;

extern crate sdl2;

mod app;
mod meldnafen;
mod store;
mod state;
mod actions;
mod reducer;

pub fn main() {
    let mut builder = Builder::from_default_env();
    builder.filter(None, LevelFilter::Debug).init();
    info!("starting up");

    debug!("initializing SDL2...");
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let _image_context = sdl2::image::init(sdl2::image::INIT_PNG).unwrap();
    let window = video_subsystem
        .window("meldnafen", 800, 600)
        .position(0, 0)
        .borderless()
        .build()
        .unwrap();

    let canvas = window.into_canvas().build().unwrap();

    debug!("initializing application...");
    let mut meldnafen = meldnafen::Meldnafen::new(canvas);
    meldnafen.render();

    debug!("looping over events...");
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: for event in event_pump.wait_iter() {
        if meldnafen.running {
            let rerender = meldnafen.apply_event(event);

            if !meldnafen.running {
                break 'running;
            }

            if rerender {
                meldnafen.render();
            }
        }
    }

    debug!("terminated");
}
