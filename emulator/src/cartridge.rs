pub struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mbc_type: MbcType,
    rom_bank: usize,
    ram_bank: usize,
    ram_enabled: bool,
    banking_mode: u8,
    rom_bank_count: usize,
}

#[derive(PartialEq, Clone, Copy)]
enum MbcType {
    None,
    Mbc1,
    Mbc3,
    Mbc5,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Self {
        let mbc_type = if rom.len() > 0x147 {
            match rom[0x147] {
                0x00 => MbcType::None,
                0x01..=0x03 => MbcType::Mbc1,
                0x0F..=0x13 => MbcType::Mbc3,
                0x19..=0x1E => MbcType::Mbc5,
                _ => MbcType::None,
            }
        } else {
            MbcType::None
        };
        let ram_size = if rom.len() > 0x149 {
            match rom[0x149] {
                0x01 => 2 * 1024,
                0x02 => 8 * 1024,
                0x03 => 32 * 1024,
                0x04 => 128 * 1024,
                0x05 => 64 * 1024,
                _ => 0,
            }
        } else {
            0
        };
        let rom_bank_count = (rom.len() / 0x4000).max(2);
        Cartridge {
            rom,
            ram: vec![0; ram_size.max(8 * 1024)],
            mbc_type,
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            banking_mode: 0,
            rom_bank_count,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                if self.mbc_type == MbcType::Mbc1 && self.banking_mode == 1 {
                    let bank = (self.ram_bank << 5) % self.rom_bank_count;
                    let offset = bank * 0x4000 + addr as usize;
                    *self.rom.get(offset).unwrap_or(&0xFF)
                } else {
                    *self.rom.get(addr as usize).unwrap_or(&0xFF)
                }
            }
            0x4000..=0x7FFF => {
                let bank = self.effective_rom_bank();
                let offset = bank * 0x4000 + (addr as usize - 0x4000);
                *self.rom.get(offset).unwrap_or(&0xFF)
            }
            0xA000..=0xBFFF => {
                if !self.ram_enabled || self.ram.is_empty() {
                    0xFF
                } else {
                    let bank = if self.mbc_type == MbcType::Mbc1 && self.banking_mode == 1 {
                        self.ram_bank
                    } else {
                        0
                    };
                    let offset = (bank * 0x2000 + (addr as usize - 0xA000)) % self.ram.len();
                    self.ram[offset]
                }
            }
            _ => 0xFF,
        }
    }

    fn effective_rom_bank(&self) -> usize {
        match self.mbc_type {
            MbcType::None => 1,
            MbcType::Mbc1 => {
                let mut bank = self.rom_bank & 0x1F;
                if bank == 0 {
                    bank = 1;
                }
                if self.banking_mode == 0 {
                    bank |= self.ram_bank << 5;
                }
                bank % self.rom_bank_count.max(1)
            }
            MbcType::Mbc3 | MbcType::Mbc5 => {
                let bank = if self.rom_bank == 0 { 1 } else { self.rom_bank };
                bank % self.rom_bank_count.max(1)
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match self.mbc_type {
            MbcType::None => {}
            MbcType::Mbc1 => match addr {
                0x0000..=0x1FFF => self.ram_enabled = (val & 0x0F) == 0x0A,
                0x2000..=0x3FFF => {
                    let bits = val & 0x1F;
                    self.rom_bank = bits as usize;
                }
                0x4000..=0x5FFF => self.ram_bank = (val & 0x03) as usize,
                0x6000..=0x7FFF => self.banking_mode = val & 0x01,
                _ => {}
            },
            MbcType::Mbc3 => match addr {
                0x0000..=0x1FFF => self.ram_enabled = (val & 0x0F) == 0x0A,
                0x2000..=0x3FFF => {
                    self.rom_bank = (val & 0x7F) as usize;
                }
                0x4000..=0x5FFF => self.ram_bank = (val & 0x03) as usize,
                _ => {}
            },
            MbcType::Mbc5 => match addr {
                0x0000..=0x1FFF => self.ram_enabled = (val & 0x0F) == 0x0A,
                0x2000..=0x2FFF => {
                    self.rom_bank = (self.rom_bank & 0x100) | val as usize;
                }
                0x3000..=0x3FFF => {
                    self.rom_bank = (self.rom_bank & 0xFF) | (((val & 1) as usize) << 8);
                }
                0x4000..=0x5FFF => self.ram_bank = (val & 0x0F) as usize,
                _ => {}
            },
        }
        if let 0xA000..=0xBFFF = addr {
            if self.ram_enabled && !self.ram.is_empty() {
                let bank = if self.mbc_type == MbcType::Mbc1 && self.banking_mode == 1 {
                    self.ram_bank
                } else {
                    0
                };
                let len = self.ram.len();
                let offset = (bank * 0x2000 + (addr as usize - 0xA000)) % len;
                self.ram[offset] = val;
            }
        }
    }
}
