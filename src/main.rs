extern crate sdl2;
extern crate rand;
use std::fs::File;
use std::io::prelude::*;
use sdl2::event::*;
use sdl2::rect::*;
use sdl2::keyboard::*;
use rand::Rng;

struct Chip8 {
    data_registers: [u8; 16],
    address_register: u16,
    memory: [u8; 0x1000],
    program_counter: u16,
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    frame_buffer: [[u8; 64]; 32], // Bit set white, bit unset black
    pushed_key: Option<u8>,
    running: bool,
    rng: rand::ThreadRng,
}

fn convert_address(nibble: u8, byte: u8) -> u16 {
    let mut address = nibble as u16;
    address <<= 8;
    address | byte as u16
}

fn run_optcode(chip8: &mut Chip8) {
    let optcode_byte_1 = chip8.memory[chip8.program_counter as usize];
    let optcode_nibble_1 = optcode_byte_1 >> 4;
    let optcode_nibble_2 = optcode_byte_1 & 0x0f;
    let optcode_byte_2 = chip8.memory[chip8.program_counter as usize + 1];
    let optcode_nibble_3 = optcode_byte_2 >> 4;
    let optcode_nibble_4 = optcode_byte_2 & 0x0f;
    match optcode_nibble_1 {
        0 => {
            if optcode_nibble_2 != 0x00 {
                println!("Unimplemented optcode");
                chip8.running = false;
            }
            match optcode_byte_2 {
                0xE0 => chip8.frame_buffer = [[0; 64]; 32],
                0xEE => {
                    if let Some(x) = chip8.stack.pop() {
                        chip8.program_counter = x;
                        return;
                    } else {
                        println!("Stack underflow");
                        chip8.running = false;
                    }
                }
                _ => {
                    println!("Unimplemented optcode");
                    chip8.running = false;
                }
            }
        }
        1 => {
            chip8.program_counter = convert_address(optcode_nibble_2, optcode_byte_2);
            return;
        }
        2 => {
            chip8.stack.push(chip8.program_counter + 2);
            chip8.program_counter = convert_address(optcode_nibble_2, optcode_byte_2);
            return;
        }
        3 => {
            if chip8.data_registers[optcode_nibble_2 as usize] == optcode_byte_2 {
                chip8.program_counter += 2;
            }
        }
        4 => {
            if chip8.data_registers[optcode_nibble_2 as usize] != optcode_byte_2 {
                chip8.program_counter += 2;
            }
        }
        5 => {
            if chip8.data_registers[optcode_nibble_4 as usize] != 0 {
                println!("Unimplemented optcode");
                chip8.running = false;
            }
            if chip8.data_registers[optcode_nibble_2 as usize] ==
               chip8.data_registers[optcode_nibble_3 as usize] {
                chip8.program_counter += 2;
            }
        }
        6 => chip8.data_registers[optcode_nibble_2 as usize] = optcode_byte_2,
        7 => {
            chip8.data_registers[optcode_nibble_2 as usize] =
                chip8.data_registers[optcode_nibble_2 as usize].wrapping_add(optcode_byte_2)
        }
        8 => {
            match optcode_nibble_4 {
                0 => {
                    chip8.data_registers[optcode_nibble_2 as usize] =
                        chip8.data_registers[optcode_nibble_3 as usize]
                }
                1 => {
                    chip8.data_registers[optcode_nibble_2 as usize] |=
                        chip8.data_registers[optcode_nibble_3 as usize]
                }
                2 => {
                    chip8.data_registers[optcode_nibble_2 as usize] &=
                        chip8.data_registers[optcode_nibble_3 as usize]
                }
                3 => {
                    chip8.data_registers[optcode_nibble_2 as usize] ^=
                        chip8.data_registers[optcode_nibble_3 as usize]
                }
                4 => {
                    let (added, overflow) = chip8.data_registers[optcode_nibble_2 as usize]
                        .overflowing_add(chip8.data_registers[optcode_nibble_3 as usize]);
                    chip8.data_registers[optcode_nibble_2 as usize] = added;
                    chip8.data_registers[0xF] = overflow as u8;
                }
                5 => {
                    let (subed, overflow) = chip8.data_registers[optcode_nibble_2 as usize]
                        .overflowing_sub(chip8.data_registers[optcode_nibble_3 as usize]);
                    chip8.data_registers[optcode_nibble_2 as usize] = subed;
                    chip8.data_registers[0xF] = overflow as u8;
                }
                6 => {
                    let lsb = chip8.data_registers[optcode_nibble_2 as usize] & 1;
                    chip8.data_registers[optcode_nibble_2 as usize] -= lsb;
                    chip8.data_registers[optcode_nibble_2 as usize] >>= 1;
                    chip8.data_registers[0xF] = lsb;
                }
                7 => {
                    let (subed, overflow) = chip8.data_registers[optcode_nibble_3 as usize]
                        .overflowing_sub(chip8.data_registers[optcode_nibble_2 as usize]);
                    chip8.data_registers[optcode_nibble_2 as usize] = subed;
                    chip8.data_registers[0xF] = overflow as u8;
                }
                0xE => {
                    let mut msb = chip8.data_registers[optcode_nibble_2 as usize] & 0x80;
                    chip8.data_registers[optcode_nibble_2 as usize] -= msb;
                    chip8.data_registers[optcode_nibble_2 as usize] <<= 1;
                    msb >>= 7;
                    chip8.data_registers[0xF] = msb;
                }
                _ => {
                    println!("Unimplemented optcode");
                    chip8.running = false;
                }
            }
        }
        9 => {
            if chip8.data_registers[optcode_nibble_4 as usize] != 0 {
                println!("Unimplemented optcode");
                chip8.running = false;
            }
            if chip8.data_registers[optcode_nibble_2 as usize] !=
               chip8.data_registers[optcode_nibble_3 as usize] {
                chip8.program_counter += 2;
            }
        }
        0xA => chip8.address_register = convert_address(optcode_nibble_2, optcode_byte_2),
        0xB => {
            chip8.program_counter = convert_address(optcode_nibble_2, optcode_byte_2);
            chip8.program_counter += chip8.data_registers[0] as u16;
            return;
        }
        0xC => {
            let rand: u8 = chip8.rng.gen();
            chip8.data_registers[optcode_nibble_2 as usize] = rand & optcode_byte_2;
        }
        0xD => {
            chip8.data_registers[0xF] = 0;
            for i in chip8.memory[chip8.address_register as usize..chip8.address_register as usize +
                                                                   optcode_nibble_4 as usize]
                .iter()
                .enumerate() {
                let (mut y_position, y) = i;
                y_position += chip8.data_registers[optcode_nibble_3 as usize] as usize;
                if y_position > 0x1F {
                    return;
                }
                for b in 0..8 {
                    let b_shifted = 1 << b;
                    let mut bit = y & b_shifted;
                    bit >>= b;
                    if bit == 1 {
                        let inverted = 7 - b;
                        let mut x_position =
                            chip8.data_registers[optcode_nibble_2 as usize] as usize +
                            inverted as usize;
                        x_position %= 0x40;
                        if chip8.frame_buffer[y_position][x_position] ^ bit == 0 {
                            chip8.frame_buffer[y_position][x_position] = 0;
                            chip8.data_registers[0xF] = 1;
                        } else {
                            chip8.frame_buffer[y_position][x_position] = 1;
                        }
                    }
                }
            }
        }
        0xE => {
            match optcode_byte_2 {
                0x9E => {
                    if let Some(key) = chip8.pushed_key {
                        if key == chip8.data_registers[optcode_nibble_2 as usize] {
                            chip8.program_counter += 2;
                        }
                    }
                }
                0xA1 => {
                    if let Some(key) = chip8.pushed_key {
                        if key != chip8.data_registers[optcode_nibble_2 as usize] {
                            chip8.program_counter += 2;
                        }
                    } else {
                        chip8.program_counter += 2;
                    }
                }
                _ => {
                    println!("Unimplemented optcode");
                    chip8.running = false;
                }
            }
        }
        0xF => {
            match optcode_byte_2 {
                0x07 => chip8.data_registers[optcode_nibble_2 as usize] = chip8.delay_timer,
                0x0A => {
                    if let Some(key) = chip8.pushed_key {
                        chip8.data_registers[optcode_nibble_2 as usize] = key
                    } else {
                        return;
                    }
                }
                0x15 => chip8.delay_timer = chip8.data_registers[optcode_nibble_2 as usize],
                0x18 => chip8.sound_timer = chip8.data_registers[optcode_nibble_2 as usize],
                0x1E => {
                    chip8.address_register += chip8.data_registers[optcode_nibble_2 as usize] as u16
                }
                0x29 => {
                    chip8.address_register =
                        chip8.data_registers[optcode_nibble_2 as usize] as u16 * 5
                }
                0x33 => {
                    let nums = chip8.data_registers[optcode_nibble_2 as usize];
                    chip8.memory[chip8.address_register as usize] = nums / 100;
                    chip8.memory[chip8.address_register as usize + 1] = nums % 100 / 10;
                    chip8.memory[chip8.address_register as usize + 2] = nums % 100 % 10;
                }
                0x55 => {
                    for i in 0..optcode_nibble_2 as usize + 1 {
                        chip8.memory[chip8.address_register as usize + i] = chip8.data_registers[i];
                    }
                }
                0x65 => {
                    for i in 0..optcode_nibble_2 as usize + 1 {
                        chip8.data_registers[i] = chip8.memory[chip8.address_register as usize + i];
                    }
                }
                _ => {
                    println!("Unimplemented optcode");
                    chip8.running = false;
                }
            }
        }
        _ => {
            println!("Unimplemented optcode");
            chip8.running = false;
        }
    }
    chip8.program_counter += 2;
}

fn main() {
    let mut args = std::env::args();
    args.next(); // We do not need the path of the executable.
    let mut chip8 = Chip8 {
        data_registers: [0; 16],
        address_register: 0,
        memory: [0; 0x1000],
        program_counter: 0x200, // Entry point of most programs
        stack: Vec::with_capacity(16),
        delay_timer: 0,
        sound_timer: 0,
        frame_buffer: [[0; 64]; 32],
        pushed_key: None,
        running: true,
        rng: rand::thread_rng(),
    };
    if let Some(file) = args.next() {
        match File::open(file) {
            Ok(mut input_file) => {
                let len = chip8.memory.len();
                let mut program_mem = &mut chip8.memory[0x200..len];
                input_file.read(program_mem).unwrap();
            }
            Err(error) => {
                println!("{}", error);
                return;
            }
        }
    } else {
        println!("Please provide the program you want to run");
        return;
    }
    let sdl = sdl2::init().unwrap();
    let sdl_video = sdl.video().unwrap();
    let sdl_window = sdl_video.window("Chip-8", 64 * 8, 32 * 8)
        .resizable()
        .build()
        .unwrap();
    let mut sdl_event_pump = sdl.event_pump().unwrap();
    let mut sdl_renderer = sdl_window.renderer().present_vsync().build().unwrap();
    sdl_renderer.set_logical_size(64, 32).unwrap();
    sdl_renderer.present();
    let cycles_rate = 50;
    let mut cycles_left = cycles_rate;
    let mut font = File::open("font.bin").unwrap();
    font.read(&mut chip8.memory).unwrap();
    while chip8.running {
        if cycles_left > 0 {
            cycles_left -= 1;
            run_optcode(&mut chip8);
        } else {
            while let Some(event) = sdl_event_pump.poll_event() {
                match event {
                    Event::Quit { .. } => chip8.running = false,
                    Event::KeyDown { keycode: kc, .. } => {
                        if let Some(key) = kc {
                            match key {
                                Keycode::Num0 => chip8.pushed_key = Some(0),
                                Keycode::Num1 => chip8.pushed_key = Some(1),
                                Keycode::Num2 => chip8.pushed_key = Some(2),
                                Keycode::Num3 => chip8.pushed_key = Some(3),
                                Keycode::Num4 => chip8.pushed_key = Some(4),
                                Keycode::Num5 => chip8.pushed_key = Some(5),
                                Keycode::Num6 => chip8.pushed_key = Some(6),
                                Keycode::Num7 => chip8.pushed_key = Some(7),
                                Keycode::Num8 => chip8.pushed_key = Some(8),
                                Keycode::Num9 => chip8.pushed_key = Some(9),
                                Keycode::A => chip8.pushed_key = Some(0xA),
                                Keycode::B => chip8.pushed_key = Some(0xB),
                                Keycode::C => chip8.pushed_key = Some(0xC),
                                Keycode::D => chip8.pushed_key = Some(0xD),
                                Keycode::E => chip8.pushed_key = Some(0xE),
                                Keycode::F => chip8.pushed_key = Some(0xF),
                                _ => {}
                            }
                        }
                    }
                    Event::KeyUp { .. } => chip8.pushed_key = None,
                    _ => {}
                }
            }
            sdl_renderer.clear();
            for i in chip8.frame_buffer.iter().enumerate() {
                let (y_cord, y) = i;
                for i in y.iter().enumerate() {
                    let (x_cord, x) = i;
                    if x & 1 != 0 {
                        sdl_renderer.draw_point(Point::new(x_cord as i32, y_cord as i32))
                            .unwrap();
                    }
                }
            }
            sdl_renderer.present();
            if chip8.delay_timer > 0 {
                chip8.delay_timer -= 1;
            }
            if chip8.sound_timer > 0 {
                chip8.sound_timer -= 1;
            }
            cycles_left = cycles_rate;
        }
    }
}
