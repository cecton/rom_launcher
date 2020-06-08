use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::convert::TryFrom;
use std::thread::sleep;
use std::time::Duration;

use crate::app::App;

pub fn tearing_test(app: &mut App) {
    let refresh_rate = app.display_mode.refresh_rate;
    let delay = Duration::from_millis(1_000_u64 / u64::try_from(refresh_rate).unwrap() + 1);
    let (w, h) = app.canvas.output_size().unwrap();
    let colors = vec![Color::RGB(0, 0, 0), Color::RGB(255, 255, 255)];
    let mut color_it = colors.iter().cycle().cloned();
    let mut fps_counter = 0;
    let t1 = app.timer.ticks();

    info!("window size: {}x{}", w, h);
    info!("refresh rate: {} Hz", refresh_rate);
    while app.is_running() {
        if let Some(event) = app.poll_event() {
            match event {
                Event::Quit { .. }
                | Event::KeyUp {
                    keycode: Some(Keycode::Q),
                    ..
                }
                | Event::KeyUp {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::JoyButtonUp { .. } => {
                    break;
                }
                _ => {}
            }
        }

        app.canvas.set_draw_color(color_it.next().unwrap());
        app.canvas.clear();

        app.canvas.present();
        sleep(delay);

        fps_counter += 1;
    }

    info!("fps: {}", fps_counter * 1000 / (app.timer.ticks() - t1));
}
