extern crate chip8;
extern crate sdl2;

use chip8::*;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum::*;
use sdl2::pixels::*;
// use sdl2::rect::*;
use sdl2::render::*;

use std::env;
use std::{thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut emulator = Chip8::new();

    emulator.initialize();
    emulator.load_rom(&args[1]);

    // Initialize and SDL context and video subsystem
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create/build our window. we start with a generous size but later
    // we set the logical size to the proper amount
    let window = video_subsystem
        .window("CHIP-8 Emulator", 64 * 12, 32 * 12)
        .position_centered()
        .build()
        .unwrap();

    // turn the window into a canvas?
    let mut canvas = window.into_canvas()
        // .present_vsync() // sync presents with refresh rate (60/122/144 hz)
        // .accelerated() // hardware acceleration
        .build()
        .unwrap();

    canvas.set_logical_size(64, 32).unwrap();
    // canvas.set_draw_color(Color::RGB(0, 0, 0)); // screen starts black
    // canvas.clear();

    // let texture = canvas.create_texture();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture(
            RGB24,
            TextureAccess::Streaming,
            chip8::DISPLAY_WIDTH as u32,
            chip8::DISPLAY_HEIGHT as u32,
        )
        .unwrap();

    // event pump... pumps out events I guess
    let mut event_pump = sdl_context.event_pump().unwrap();

    'main_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'main_loop,
                Event::KeyDown { keycode: Some(key), .. } => emulator.handle_keypress(key, true),
                Event::KeyUp { keycode: Some(key), .. } => emulator.handle_keypress(key, false),
                _ => {}
            }
        }

        // clear screen
        canvas.set_draw_color(Color::RGB(0, 0, 0)); // screen starts black
        canvas.clear();

        emulator.emulate_cycle();

        texture
            .update(None, &emulator.display, chip8::DISPLAY_WIDTH * 3)
            .unwrap();

        // copy texture to renderer (canvas)
        canvas.copy(&texture, None, None).unwrap();

        // present
        canvas.present();

        // TODO: sync at known pace. vsync is too fast
        thread::sleep(time::Duration::from_millis(10));
    }
}
