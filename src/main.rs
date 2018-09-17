extern crate env_logger;
extern crate id_tree;
#[macro_use]
extern crate log;
extern crate sdl2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate num;
extern crate serde_json;
extern crate tempfile;

use env_logger::Builder;
use log::LevelFilter;
use std::process::Command;
use tempfile::NamedTempFile;

mod app;
mod draw;
mod joystick;
mod rom_launcher;
mod store;

#[cfg(debug_assertions)]
fn initialize_logger() {
    let mut builder = Builder::from_default_env();
    builder.filter(None, LevelFilter::Debug).init();
}

#[cfg(not(debug_assertions))]
fn initialize_logger() {
    let mut builder = Builder::from_default_env();
    builder.filter(None, LevelFilter::Info).init();
}

#[cfg(debug_assertions)]
fn initialize_app() -> app::App {
    let app = app::App::new(|video| {
        video
            .window("ROMLauncher", 800, 700)
            .position(0, 0)
            .borderless()
            .build()
    });
    app
}

#[cfg(not(debug_assertions))]
fn initialize_app() -> app::App {
    let app = app::App::new(|video| {
        video
            .window("ROMLauncher", 0, 0)
            .position(0, 0)
            .fullscreen_desktop()
            .build()
    });
    app.sdl_context.mouse().show_cursor(false);
    app
}

pub fn main() {
    initialize_logger();
    info!("starting up");

    let mut command;
    loop {
        {
            let app = initialize_app();
            let mut romlauncher = rom_launcher::ROMLauncher::new(app);
            command = romlauncher.run_loop();
        }

        match command {
            Some((command, config, rom)) => {
                use std::io::Write;

                let mut file = NamedTempFile::new().expect("can't open temporary file");
                write!(file, "{}", &config).unwrap();

                let status = Command::new(command.get(0).unwrap())
                    .args(command.iter().skip(1))
                    .args(&["--appendconfig", file.path().to_str().unwrap(), &rom])
                    .status()
                    .expect("retroarch failed to start");

                info!("retroarch exited with code {}", status);
            }
            None => {
                debug!("no command received");
                break;
            }
        }
    }

    debug!("terminated");
}
