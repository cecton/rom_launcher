extern crate bincode;
extern crate env_logger;
extern crate id_tree;
#[macro_use]
extern crate log;
extern crate sdl2;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use log::LevelFilter;
use env_logger::Builder;

mod app;
mod meldnafen;
mod store;
mod draw;
mod joystick;

pub fn main() {
    let mut builder = Builder::from_default_env();
    builder.filter(None, LevelFilter::Debug).init();
    info!("starting up");

    let mut command;
    loop {
        {
            let mut meldnafen = meldnafen::Meldnafen::new();
            command = meldnafen.run_loop();
        }

        if command.is_none() {
            debug!("no command received");
            break;
        }
    }

    debug!("terminated");
}
