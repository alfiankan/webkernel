pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

const MODE_HBLANK: u8 = 0;
const MODE_VBLANK: u8 = 1;
const MODE_OAM: u8 = 2;
const MODE_TRANSFER: u8 = 3;

pub struct Ppu {
    pub vram: [u8; 0x2000],
    pub oam: [u8; 0xA0],

    pub lcdc: u8,
    pub stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub wy: u8,
    pub wx: u8,

    mode: u8,
    dot: u32,
    window_line: u8,

    pub framebuffer: [u8; SCREEN_W * SCREEN_H * 4],
    pub vblank_irq: bool,
    pub stat_irq: bool,
    pub frame_ready: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcdc: 0x91,
            stat: 0x85,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            mode: MODE_OAM,
            dot: 0,
            window_line: 0,
            framebuffer: [0xFF; SCREEN_W * SCREEN_H * 4],
            vblank_irq: false,
            stat_irq: false,
            frame_ready: false,
        }
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr - 0x8000) as usize]
    }
    pub fn write_vram(&mut self, addr: u16, val: u8) {
        self.vram[(addr - 0x8000) as usize] = val;
    }
    pub fn read_oam(&self, addr: u16) -> u8 {
        self.oam[(addr - 0xFE00) as usize]
    }
    pub fn write_oam(&mut self, addr: u16, val: u8) {
        self.oam[(addr - 0xFE00) as usize] = val;
    }

    fn lcd_on(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    pub fn step(&mut self, cycles: u32) {
        if !self.lcd_on() {
            self.dot = 0;
            self.ly = 0;
            self.mode = MODE_HBLANK;
            self.stat = (self.stat & 0xF8) | self.mode;
            return;
        }

        self.dot += cycles;

        match self.mode {
            MODE_OAM => {
                if self.dot >= 80 {
                    self.dot -= 80;
                    self.mode = MODE_TRANSFER;
                }
            }
            MODE_TRANSFER => {
                if self.dot >= 172 {
                    self.dot -= 172;
                    self.mode = MODE_HBLANK;
                    self.render_scanline();
                    if self.stat & 0x08 != 0 {
                        self.stat_irq = true;
                    }
                }
            }
            MODE_HBLANK => {
                if self.dot >= 204 {
                    self.dot -= 204;
                    self.ly += 1;
                    if self.ly == 144 {
                        self.mode = MODE_VBLANK;
                        self.vblank_irq = true;
                        self.frame_ready = true;
                        self.window_line = 0;
                        if self.stat & 0x10 != 0 {
                            self.stat_irq = true;
                        }
                    } else {
                        self.mode = MODE_OAM;
                        if self.stat & 0x20 != 0 {
                            self.stat_irq = true;
                        }
                    }
                    self.check_lyc();
                }
            }
            MODE_VBLANK => {
                if self.dot >= 456 {
                    self.dot -= 456;
                    self.ly += 1;
                    if self.ly > 153 {
                        self.ly = 0;
                        self.mode = MODE_OAM;
                        if self.stat & 0x20 != 0 {
                            self.stat_irq = true;
                        }
                    }
                    self.check_lyc();
                }
            }
            _ => {}
        }

        self.stat = (self.stat & 0xF8) | self.mode;
    }

    fn check_lyc(&mut self) {
        if self.ly == self.lyc {
            self.stat |= 0x04;
            if self.stat & 0x40 != 0 {
                self.stat_irq = true;
            }
        } else {
            self.stat &= !0x04;
        }
    }

    fn render_scanline(&mut self) {
        let ly = self.ly;
        if ly as usize >= SCREEN_H {
            return;
        }
        let mut bg_color_idx = [0u8; SCREEN_W];

        if self.lcdc & 0x01 != 0 {
            let use_window =
                self.lcdc & 0x20 != 0 && self.wy <= ly && self.wx <= 166;
            let bg_map_base: u16 = if self.lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 };
            let win_map_base: u16 = if self.lcdc & 0x40 != 0 { 0x9C00 } else { 0x9800 };
            let signed_tiles = self.lcdc & 0x10 == 0;

            for x in 0..SCREEN_W {
                let (map_base, tile_x, tile_y, fine_x, fine_y);
                if use_window && x as u8 + 7 >= self.wx {
                    map_base = win_map_base;
                    let wx = (x as u16 + 7).wrapping_sub(self.wx as u16);
                    tile_x = (wx / 8) as u16;
                    fine_x = (wx % 8) as u8;
                    tile_y = (self.window_line / 8) as u16;
                    fine_y = self.window_line % 8;
                } else {
                    map_base = bg_map_base;
                    let bx = (x as u16 + self.scx as u16) & 0xFF;
                    let by = (ly as u16 + self.scy as u16) & 0xFF;
                    tile_x = bx / 8;
                    fine_x = (bx % 8) as u8;
                    tile_y = by / 8;
                    fine_y = (by % 8) as u8;
                }

                let map_addr = map_base + tile_y * 32 + tile_x;
                let tile_num = self.vram[(map_addr - 0x8000) as usize];

                let tile_addr: u16 = if signed_tiles {
                    let signed = tile_num as i8 as i16;
                    (0x9000i32 + (signed as i32) * 16) as u16
                } else {
                    0x8000 + (tile_num as u16) * 16
                };
                let row_addr = tile_addr + (fine_y as u16) * 2;
                let lo = self.vram[(row_addr - 0x8000) as usize];
                let hi = self.vram[(row_addr + 1 - 0x8000) as usize];
                let bit = 7 - fine_x;
                let color_num = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
                bg_color_idx[x] = color_num;

                let color = palette_color(self.bgp, color_num);
                self.set_pixel(x, ly as usize, color);
            }
            if use_window {
                self.window_line += 1;
            }
        } else {
            for x in 0..SCREEN_W {
                self.set_pixel(x, ly as usize, [255, 255, 255]);
            }
        }

        if self.lcdc & 0x02 != 0 {
            self.render_sprites(ly, &bg_color_idx);
        }
    }

    fn render_sprites(&mut self, ly: u8, bg_color_idx: &[u8; SCREEN_W]) {
        let tall = self.lcdc & 0x04 != 0;
        let sprite_h: i16 = if tall { 16 } else { 8 };

        let mut visible: Vec<(u8, usize)> = Vec::with_capacity(10);
        for i in 0..40 {
            let base = i * 4;
            let sy = self.oam[base] as i16 - 16;
            if (ly as i16) >= sy && (ly as i16) < sy + sprite_h {
                visible.push((self.oam[base + 1], base));
                if visible.len() >= 10 {
                    break;
                }
            }
        }
        // Lower x = higher priority; draw lowest priority first so higher overwrites.
        visible.sort_by(|a, b| b.0.cmp(&a.0));

        for (_sx, base) in visible {
            let sy = self.oam[base] as i16 - 16;
            let sx = self.oam[base + 1] as i16 - 8;
            let mut tile_num = self.oam[base + 2];
            let attr = self.oam[base + 3];
            let behind_bg = attr & 0x80 != 0;
            let yflip = attr & 0x40 != 0;
            let xflip = attr & 0x20 != 0;
            let palette = if attr & 0x10 != 0 { self.obp1 } else { self.obp0 };

            let mut line = ly as i16 - sy;
            if yflip {
                line = sprite_h - 1 - line;
            }
            if tall {
                tile_num &= 0xFE;
                if line >= 8 {
                    tile_num |= 1;
                    line -= 8;
                }
            }
            let tile_addr = 0x8000u16 + (tile_num as u16) * 16 + (line as u16) * 2;
            let lo = self.vram[(tile_addr - 0x8000) as usize];
            let hi = self.vram[(tile_addr + 1 - 0x8000) as usize];

            for px in 0..8i16 {
                let bit = if xflip { px } else { 7 - px };
                let color_num = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
                if color_num == 0 {
                    continue;
                }
                let screen_x = sx + px;
                if screen_x < 0 || screen_x >= SCREEN_W as i16 {
                    continue;
                }
                if behind_bg && bg_color_idx[screen_x as usize] != 0 {
                    continue;
                }
                let color = palette_color(palette, color_num);
                self.set_pixel(screen_x as usize, ly as usize, color);
            }
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let idx = (y * SCREEN_W + x) * 4;
        self.framebuffer[idx] = rgb[0];
        self.framebuffer[idx + 1] = rgb[1];
        self.framebuffer[idx + 2] = rgb[2];
        self.framebuffer[idx + 3] = 255;
    }
}

fn palette_color(palette: u8, color_num: u8) -> [u8; 3] {
    let shade = (palette >> (color_num * 2)) & 0x03;
    match shade {
        0 => [0xE0, 0xF8, 0xD0],
        1 => [0x88, 0xC0, 0x70],
        2 => [0x34, 0x68, 0x56],
        3 => [0x08, 0x18, 0x20],
        _ => unreachable!(),
    }
}
