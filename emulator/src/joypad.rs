#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

pub struct Joypad {
    select: u8,
    direction: u8,
    action: u8,
    pub irq: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            select: 0x30,
            direction: 0,
            action: 0,
            irq: false,
        }
    }

    pub fn read(&self) -> u8 {
        let mut bits: u8 = 0x0F;
        if self.select & 0x20 == 0 {
            bits &= !self.action;
        }
        if self.select & 0x10 == 0 {
            bits &= !self.direction;
        }
        0xC0 | (self.select & 0x30) | bits
    }

    pub fn write(&mut self, val: u8) {
        self.select = val & 0x30;
    }

    pub fn set(&mut self, button: Button, pressed: bool) {
        let (group, bit) = match button {
            Button::Right => (0, 0x01),
            Button::Left => (0, 0x02),
            Button::Up => (0, 0x04),
            Button::Down => (0, 0x08),
            Button::A => (1, 0x01),
            Button::B => (1, 0x02),
            Button::Select => (1, 0x04),
            Button::Start => (1, 0x08),
        };
        let field = if group == 0 {
            &mut self.direction
        } else {
            &mut self.action
        };
        let was = *field & bit != 0;
        if pressed {
            *field |= bit;
            if !was {
                self.irq = true;
            }
        } else {
            *field &= !bit;
        }
    }
}
