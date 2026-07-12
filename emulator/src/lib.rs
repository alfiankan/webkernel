#![allow(static_mut_refs)]

mod apu;
mod cartridge;
mod cpu;
mod joypad;
mod mmu;
mod ppu;
mod timer;

use cpu::Cpu;
use joypad::Button;
use mmu::Mmu;

const CYCLES_PER_FRAME: u32 = 70224; // 4194304 Hz / 59.7275 fps

pub struct GameBoy {
    cpu: Cpu,
    mmu: Mmu,
}

impl GameBoy {
    fn new(rom: Vec<u8>) -> Self {
        GameBoy {
            cpu: Cpu::new(),
            mmu: Mmu::new(rom),
        }
    }

    fn run_frame(&mut self) {
        let mut total = 0u32;
        while total < CYCLES_PER_FRAME {
            let cycles = self.cpu.step(&mut self.mmu);
            self.mmu.tick_components(cycles);
            total += cycles;
        }
    }
}

static mut GB: Option<GameBoy> = None;
static mut ROM_BUF: Vec<u8> = Vec::new();
static mut AUDIO_BUF: Vec<f32> = Vec::new();

/// Reserve a buffer in linear memory for the host to copy ROM bytes into,
/// then call `load_rom` with the same length.
#[no_mangle]
pub extern "C" fn alloc_rom_buffer(len: usize) -> *mut u8 {
    unsafe {
        ROM_BUF = vec![0u8; len];
        ROM_BUF.as_mut_ptr()
    }
}

#[no_mangle]
pub extern "C" fn load_rom(len: usize) {
    unsafe {
        let rom = ROM_BUF[..len].to_vec();
        GB = Some(GameBoy::new(rom));
    }
}

#[no_mangle]
pub extern "C" fn run_frame() {
    unsafe {
        if let Some(gb) = GB.as_mut() {
            gb.run_frame();
        }
    }
}

#[no_mangle]
pub extern "C" fn framebuffer_ptr() -> *const u8 {
    unsafe {
        match GB.as_ref() {
            Some(gb) => gb.mmu.ppu.framebuffer.as_ptr(),
            None => core::ptr::null(),
        }
    }
}

#[no_mangle]
pub extern "C" fn framebuffer_len() -> usize {
    ppu::SCREEN_W * ppu::SCREEN_H * 4
}

#[no_mangle]
pub extern "C" fn audio_ptr() -> *const f32 {
    unsafe {
        if let Some(gb) = GB.as_mut() {
            AUDIO_BUF = gb.mmu.apu.take_buffer();
        }
        AUDIO_BUF.as_ptr()
    }
}

#[no_mangle]
pub extern "C" fn audio_len() -> usize {
    unsafe { AUDIO_BUF.len() }
}

fn button_from_code(code: u32) -> Option<Button> {
    match code {
        0 => Some(Button::Right),
        1 => Some(Button::Left),
        2 => Some(Button::Up),
        3 => Some(Button::Down),
        4 => Some(Button::A),
        5 => Some(Button::B),
        6 => Some(Button::Select),
        7 => Some(Button::Start),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn key_down(code: u32) {
    unsafe {
        if let Some(gb) = GB.as_mut() {
            if let Some(btn) = button_from_code(code) {
                gb.mmu.joypad.set(btn, true);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn key_up(code: u32) {
    unsafe {
        if let Some(gb) = GB.as_mut() {
            if let Some(btn) = button_from_code(code) {
                gb.mmu.joypad.set(btn, false);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn reset() {
    unsafe {
        if !ROM_BUF.is_empty() {
            GB = Some(GameBoy::new(ROM_BUF.clone()));
        }
    }
}

#[no_mangle]
pub extern "C" fn debug_peek(addr: u16) -> u8 {
    unsafe {
        match GB.as_ref() {
            Some(gb) => gb.mmu.read(addr),
            None => 0xFF,
        }
    }
}

#[no_mangle]
pub extern "C" fn debug_pc() -> u16 {
    unsafe {
        match GB.as_ref() {
            Some(gb) => gb.cpu.pc,
            None => 0xFFFF,
        }
    }
}

#[no_mangle]
pub extern "C" fn is_loaded() -> u32 {
    unsafe { if GB.is_some() { 1 } else { 0 } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selftest_rom_renders_expected_pixel() {
        let rom = std::fs::read("../rom/tests/selftest.gb").expect("rom");
        let mut gb = GameBoy::new(rom);
        for _ in 0..5 {
            gb.run_frame();
        }
        eprintln!("pc={:04x} lcdc={:02x} bgp={:02x} v0={:02x} v1={:02x}",
            gb.cpu.pc, gb.mmu.ppu.lcdc, gb.mmu.ppu.bgp,
            gb.mmu.ppu.read_vram(0x8000), gb.mmu.ppu.read_vram(0x8001));
        let idx = (0 * ppu::SCREEN_W + 0) * 4;
        let px = &gb.mmu.ppu.framebuffer[idx..idx + 4];
        eprintln!("pixel(0,0) = {:?}", px);
        assert_eq!(px, &[0x08, 0x18, 0x20, 255]);
    }

    #[test]
    fn irq_and_halt_resume_every_frame() {
        let rom = std::fs::read("../rom/tests/irqtest.gb").expect("rom");
        let mut gb = GameBoy::new(rom);
        for i in 1..=10 {
            gb.run_frame();
            let counter = gb.mmu.read(0xC000);
            eprintln!("frame {} counter={}", i, counter);
            assert_eq!(counter as i32, i);
        }
    }

    // WRAM addresses defined in rom/src/tetris.asm.tmpl
    const T_FIELD: u16 = 0xC000;
    const T_PIECE_TYPE: u16 = 0xC0A0;
    const T_PIECE_ROT: u16 = 0xC0A1;
    const T_PIECE_X: u16 = 0xC0A2;
    const T_PIECE_Y: u16 = 0xC0A3;
    const T_PREV_X: u16 = 0xC0A4;
    const T_PREV_Y: u16 = 0xC0A5;
    const T_PREV_ROT: u16 = 0xC0A6;
    const T_GRAV_TIMER: u16 = 0xC0A9;
    const T_GRAV_SPEED: u16 = 0xC0AA;
    const T_GAME_STATE: u16 = 0xC0AF;
    const T_SCORE0: u16 = 0xC0B9;
    const T_LINES0: u16 = 0xC0BF;

    fn load_tetris() -> GameBoy {
        let rom = std::fs::read("../rom/tests/tetris.gb").expect("tetris rom");
        GameBoy::new(rom)
    }

    fn press_and_release(gb: &mut GameBoy, btn: Button) {
        gb.mmu.joypad.set(btn, true);
        gb.run_frame();
        gb.mmu.joypad.set(btn, false);
    }

    /// Presses Start and runs frames until GAME_STATE becomes "playing" (1),
    /// or panics if it doesn't happen within a generous frame budget.
    /// start_game does enough work (clearing + redrawing the whole
    /// playfield) that it can legitimately spill across a couple of frame
    /// boundaries depending on when in a frame Start happens to be sampled.
    fn start_and_wait_for_playing(gb: &mut GameBoy) {
        gb.mmu.joypad.set(Button::Start, true);
        gb.run_frame();
        gb.mmu.joypad.set(Button::Start, false);
        for _ in 0..30 {
            if gb.mmu.read(T_GAME_STATE) == 1 {
                return;
            }
            gb.run_frame();
        }
        panic!("game did not reach playing state within 30 frames of Start");
    }

    #[test]
    fn tetris_boots_to_title_screen() {
        let mut gb = load_tetris();
        for _ in 0..10 {
            gb.run_frame();
        }
        assert_eq!(gb.mmu.read(T_GAME_STATE), 0);
    }

    #[test]
    fn tetris_start_button_spawns_first_piece() {
        let mut gb = load_tetris();
        for _ in 0..5 {
            gb.run_frame();
        }
        start_and_wait_for_playing(&mut gb);
        assert_eq!(gb.mmu.read(T_GAME_STATE), 1);
        let ptype = gb.mmu.read(T_PIECE_TYPE);
        assert!(ptype <= 6, "piece type {} out of range", ptype);
        assert_eq!(gb.mmu.read(T_PIECE_Y), 0);
    }

    #[test]
    fn tetris_piece_falls_over_time() {
        let mut gb = load_tetris();
        for _ in 0..5 {
            gb.run_frame();
        }
        start_and_wait_for_playing(&mut gb);
        let speed = gb.mmu.read(T_GRAV_SPEED) as i32;
        for _ in 0..(speed + 2) {
            gb.run_frame();
        }
        let y = gb.mmu.read(T_PIECE_Y);
        eprintln!("piece_y after gravity ticks = {}", y);
        assert!(y >= 1, "expected piece to have fallen at least one row, y={}", y);
    }

    #[test]
    fn tetris_move_left_shifts_piece() {
        let mut gb = load_tetris();
        for _ in 0..5 {
            gb.run_frame();
        }
        start_and_wait_for_playing(&mut gb);
        let x0 = gb.mmu.read(T_PIECE_X);
        press_and_release(&mut gb, Button::Left);
        gb.run_frame();
        let x1 = gb.mmu.read(T_PIECE_X);
        eprintln!("x0={} x1={}", x0, x1);
        assert_eq!(x1, x0.wrapping_sub(1));
    }

    #[test]
    fn tetris_rotate_changes_rotation_state() {
        let mut gb = load_tetris();
        for _ in 0..5 {
            gb.run_frame();
        }
        start_and_wait_for_playing(&mut gb);
        // O piece (type 1) looks the same in every rotation but the ROT
        // counter still advances; force a non-O piece for a visible effect.
        gb.mmu.write(T_PIECE_TYPE, 2); // T piece
        let r0 = gb.mmu.read(T_PIECE_ROT);
        press_and_release(&mut gb, Button::A);
        gb.run_frame();
        let r1 = gb.mmu.read(T_PIECE_ROT);
        eprintln!("r0={} r1={}", r0, r1);
        assert_eq!(r1, (r0 + 1) % 4);
    }

    #[test]
    fn tetris_full_row_clears_and_scores() {
        let mut gb = load_tetris();
        for _ in 0..5 {
            gb.run_frame();
        }
        start_and_wait_for_playing(&mut gb);

        // Zero the field, then fill row 15 solid except columns 4,5.
        for i in 0..160u16 {
            gb.mmu.write(T_FIELD + i, 0);
        }
        for col in 0..10u16 {
            if col != 4 && col != 5 {
                gb.mmu.write(T_FIELD + 15 * 10 + col, 1);
            }
        }

        // Place an O-piece (type 1) so it lands exactly on columns 4,5.
        gb.mmu.write(T_PIECE_TYPE, 1);
        gb.mmu.write(T_PIECE_ROT, 0);
        gb.mmu.write(T_PIECE_X, 3); // O rot0 dx=1,2 -> cols 4,5
        gb.mmu.write(T_PIECE_Y, 14); // dy=0,1 -> rows 14,15
        gb.mmu.write(T_PREV_X, 3);
        gb.mmu.write(T_PREV_Y, 14);
        gb.mmu.write(T_PREV_ROT, 0);
        gb.mmu.write(T_GRAV_TIMER, 1);
        gb.mmu.write(T_GRAV_SPEED, 45);

        for _ in 0..10 {
            gb.run_frame();
            if gb.mmu.read(T_LINES0 + 2) != 0 {
                break;
            }
        }

        let lines_msb = gb.mmu.read(T_LINES0);
        let lines_lsb2 = gb.mmu.read(T_LINES0 + 2);
        eprintln!("lines digits = {} {} {}", lines_msb, gb.mmu.read(T_LINES0 + 1), lines_lsb2);
        assert_eq!(lines_lsb2, 1, "expected LINES counter to read 1 after clearing one row");

        let score_digits: Vec<u8> = (0..6).map(|i| gb.mmu.read(T_SCORE0 + i)).collect();
        eprintln!("score digits = {:?}", score_digits);
        assert_eq!(score_digits, vec![0, 0, 0, 0, 4, 0], "expected score 000040");

        // Row 15 should now hold what was row 14 (only cols 4,5 set from the
        // locked piece), since the full row was removed and rows above
        // shifted down.
        for col in 0..10u16 {
            let v = gb.mmu.read(T_FIELD + 15 * 10 + col);
            if col == 4 || col == 5 {
                assert_eq!(v, 1, "col {} should be filled after shift", col);
            } else {
                assert_eq!(v, 0, "col {} should be empty after clear", col);
            }
        }
    }

    #[test]
    fn joypad_reflects_button_state() {
        let rom = std::fs::read("../rom/tests/selftest.gb").expect("rom");
        let mut gb = GameBoy::new(rom);
        gb.mmu.joypad.write(0x20); // select direction keys
        assert_eq!(gb.mmu.joypad.read() & 0x0F, 0x0F);
        gb.mmu.joypad.set(Button::Right, true);
        assert_eq!(gb.mmu.joypad.read() & 0x0F, 0x0E);
        gb.mmu.joypad.set(Button::Right, false);
        assert_eq!(gb.mmu.joypad.read() & 0x0F, 0x0F);
    }
}
