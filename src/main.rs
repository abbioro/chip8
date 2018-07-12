extern crate chip8;
extern crate sdl2;

use chip8::*;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum::*;
use sdl2::pixels::*;
use sdl2::render::*;
use sdl2::audio::{AudioCallback, AudioSpecDesired};

use std::env;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 { self.volume } else { -self.volume };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut emulator = CPU::new();

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

    // TODO cleanup audio code
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),  // mono
        samples: None       // default sample size
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // Show obtained AudioSpec
        println!("{:?}", spec);

        // initialize the audio callback
        SquareWave {
            phase_inc: 100.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.05
        }
    }).unwrap();

    // event pump... pumps out events I guess
    let mut event_pump = sdl_context.event_pump().unwrap();

    'main_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'main_loop,
                Event::KeyDown { keycode: Some(key), .. } => emulator.update_keypad(key, true),
                Event::KeyUp { keycode: Some(key), .. } => emulator.update_keypad(key, false),
                _ => {}
            }
        }

        // clear screen
        canvas.set_draw_color(Color::RGB(0, 0, 0)); // screen starts black
        canvas.clear();

        emulator.emulate_cycle();
        
        if emulator.sound_timer > 0 {
            device.resume();
        } else {
            device.pause();
        }

        texture
            .update(None, &emulator.display, chip8::DISPLAY_WIDTH * 3)
            .unwrap();

        // copy texture to renderer (canvas)
        canvas.copy(&texture, None, None).unwrap();

        // present
        canvas.present();

        // TODO: sync at known pace. vsync is too fast
        // thread::sleep(time::Duration::from_millis(10));
    }
}
