extern crate env_logger;
extern crate id_tree;
#[macro_use]
extern crate log;
extern crate sdl2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
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

pub fn main() {
    let mut builder = Builder::from_default_env();
    builder.filter(None, LevelFilter::Debug).init();
    info!("starting up");

    let mut command;
    loop {
        {
            let mut romlauncher = rom_launcher::ROMLauncher::new();
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
