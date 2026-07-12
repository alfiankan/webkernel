(() => {
  "use strict";

  const BUTTON_CODES = {
    right: 0,
    left: 1,
    up: 2,
    down: 3,
    a: 4,
    b: 5,
    select: 6,
    start: 7,
  };

  const KEY_MAP = {
    ArrowRight: "right",
    ArrowLeft: "left",
    ArrowUp: "up",
    ArrowDown: "down",
    KeyX: "a",
    KeyZ: "b",
    Enter: "start",
    ShiftLeft: "select",
    ShiftRight: "select",
    ShiftLeft2: "select",
  };

  const SCREEN_W = 160;
  const SCREEN_H = 144;
  const FRAME_MS = 1000 / 59.7275;
  const SAMPLE_RATE = 44100;

  const statusEl = document.getElementById("status");
  const overlayEl = document.getElementById("screenOverlay");
  const powerLed = document.getElementById("powerLed");
  const canvas = document.getElementById("screen");
  const ctx2d = canvas.getContext("2d", { alpha: false });
  const imageData = ctx2d.createImageData(SCREEN_W, SCREEN_H);

  const debuggerEl = document.getElementById("debugger");
  const regGridEl = document.getElementById("regGrid");
  const flagRowEl = document.getElementById("flagRow");
  const cpuMiscEl = document.getElementById("cpuMisc");
  const disasmEl = document.getElementById("disasmView");
  const cartInfoEl = document.getElementById("cartInfo");
  const tileCanvas = document.getElementById("tileView");
  const tileCtx = tileCanvas.getContext("2d");
  const hexdumpEl = document.getElementById("hexdumpView");
  const memRegionEl = document.getElementById("memRegion");
  const memGotoEl = document.getElementById("memGoto");

  let exports = null;
  let memory = null;
  let romLoaded = false;
  let paused = false;
  let muted = false;
  let audioCtx = null;
  let nextAudioTime = 0;
  let gainNode = null;
  let currentRomBytes = null;
  let cachedHeader = null;

  function setStatus(msg) {
    statusEl.textContent = msg;
  }

  function setOverlay(msg) {
    if (msg) {
      overlayEl.textContent = msg;
      overlayEl.classList.remove("hidden");
    } else {
      overlayEl.classList.add("hidden");
    }
  }

  async function loadWasm() {
    const resp = await fetch("pkg/gbcore.wasm");
    const bytes = await resp.arrayBuffer();
    const { instance } = await WebAssembly.instantiate(bytes, {});
    exports = instance.exports;
    memory = exports.memory;
  }

  async function loadRomBytes(bytes) {
    const ptr = exports.alloc_rom_buffer(bytes.length);
    new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
    exports.load_rom(bytes.length);
    romLoaded = true;
    powerLed.classList.add("on");
    setOverlay(null);
    currentRomBytes = bytes;
    cachedHeader = parseCartHeader(bytes);
    renderCartInfo();
    memPage = 0;
    if (debugVisible) updateDebugger(true);
  }

  async function loadDefaultCartridge() {
    setStatus("Loading TETRIS cartridge…");
    const resp = await fetch("roms/tetris.gb");
    const bytes = new Uint8Array(await resp.arrayBuffer());
    await loadRomBytes(bytes);
    setStatus("TETRIS loaded — press Start.");
  }

  function initAudio() {
    if (audioCtx) return;
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    audioCtx = new AudioCtx({ sampleRate: SAMPLE_RATE });
    gainNode = audioCtx.createGain();
    gainNode.gain.value = muted ? 0 : 0.5;
    gainNode.connect(audioCtx.destination);
    nextAudioTime = audioCtx.currentTime + 0.05;
  }

  function pumpAudio() {
    if (!audioCtx) return;
    const ptr = exports.audio_ptr();
    const len = exports.audio_len();
    if (len === 0) return;
    const frameCount = len / 2;
    const floats = new Float32Array(memory.buffer, ptr, len);
    const buffer = audioCtx.createBuffer(2, frameCount, SAMPLE_RATE);
    const left = buffer.getChannelData(0);
    const right = buffer.getChannelData(1);
    for (let i = 0; i < frameCount; i++) {
      left[i] = floats[i * 2];
      right[i] = floats[i * 2 + 1];
    }
    const source = audioCtx.createBufferSource();
    source.buffer = buffer;
    source.connect(gainNode);

    const now = audioCtx.currentTime;
    if (nextAudioTime < now) {
      nextAudioTime = now + 0.02;
    }
    source.start(nextAudioTime);
    nextAudioTime += buffer.duration;
  }

  function renderFrame() {
    const ptr = exports.framebuffer_ptr();
    const len = exports.framebuffer_len();
    const src = new Uint8ClampedArray(memory.buffer, ptr, len);
    imageData.data.set(src);
    ctx2d.putImageData(imageData, 0, 0);
  }

  let accumulator = 0;
  let lastTime = performance.now();

  function tick(now) {
    requestAnimationFrame(tick);
    if (!romLoaded) {
      lastTime = now;
      return;
    }
    if (paused) {
      lastTime = now;
      if (debugVisible) updateDebugger();
      return;
    }
    let delta = now - lastTime;
    lastTime = now;
    if (delta > 250) delta = 250; // clamp after tab was backgrounded
    accumulator += delta;

    let steps = 0;
    while (accumulator >= FRAME_MS && steps < 4) {
      exports.run_frame();
      pumpAudio();
      accumulator -= FRAME_MS;
      steps++;
    }
    if (steps > 0) {
      renderFrame();
    }
    if (debugVisible) updateDebugger();
  }

  function pressButton(name) {
    const code = BUTTON_CODES[name];
    if (code === undefined || !romLoaded) return;
    exports.key_down(code);
    highlightButton(name, true);
    if (audioCtx && audioCtx.state === "suspended") {
      audioCtx.resume();
    }
  }

  function releaseButton(name) {
    const code = BUTTON_CODES[name];
    if (code === undefined || !romLoaded) return;
    exports.key_up(code);
    highlightButton(name, false);
  }

  function highlightButton(name, on) {
    const el = document.querySelector(`[data-btn="${name}"]`);
    if (el) el.classList.toggle("pressed", on);
  }

  function isTypingTarget(el) {
    return el && (el.tagName === "INPUT" || el.tagName === "SELECT" || el.tagName === "TEXTAREA");
  }

  function setupKeyboard() {
    const held = new Set();
    window.addEventListener("keydown", (e) => {
      if (isTypingTarget(document.activeElement)) return;
      const name = KEY_MAP[e.code];
      if (!name) return;
      e.preventDefault();
      if (held.has(name)) return;
      held.add(name);
      pressButton(name);
    });
    window.addEventListener("keyup", (e) => {
      if (isTypingTarget(document.activeElement)) return;
      const name = KEY_MAP[e.code];
      if (!name) return;
      e.preventDefault();
      held.delete(name);
      releaseButton(name);
    });
  }

  // Real screen readers/quick taps can press and release an on-screen
  // button faster than one emulated frame (~16.7ms), which would make the
  // emulator's edge-triggered input never observe the press at all. Hold
  // every on-screen tap for at least this long before actually releasing.
  const MIN_HOLD_MS = 80;

  function setupOnScreenButtons() {
    const buttons = document.querySelectorAll("[data-btn]");
    buttons.forEach((btn) => {
      const name = btn.dataset.btn;
      let pressedAt = 0;
      let releaseTimer = null;

      const doRelease = () => {
        releaseTimer = null;
        releaseButton(name);
      };

      const down = (e) => {
        e.preventDefault();
        try {
          btn.setPointerCapture && btn.setPointerCapture(e.pointerId);
        } catch (_) {
          // Ignore: capture is a nice-to-have, not required for correctness.
        }
        if (releaseTimer) {
          clearTimeout(releaseTimer);
          releaseTimer = null;
        }
        pressedAt = performance.now();
        pressButton(name);
      };
      const up = (e) => {
        e.preventDefault();
        const elapsed = performance.now() - pressedAt;
        if (elapsed < MIN_HOLD_MS) {
          releaseTimer = setTimeout(doRelease, MIN_HOLD_MS - elapsed);
        } else {
          doRelease();
        }
      };
      btn.addEventListener("pointerdown", down);
      btn.addEventListener("pointerup", up);
      btn.addEventListener("pointercancel", up);
      btn.addEventListener("pointerleave", up);
      btn.addEventListener("contextmenu", (e) => e.preventDefault());
    });
  }

  // ---------------------------------------------------------------------
  // Debugger: registers, disassembly, VRAM tile viewer, memory hex dump,
  // cartridge header. Everything reads directly from wasm memory/exports,
  // so it always reflects exactly what the emulated CPU sees.
  // ---------------------------------------------------------------------

  let debugVisible = false;
  let memRegionName = "wram";
  let memPage = 0;
  let debugFrameCount = 0;

  const CART_TYPES = {
    0x00: "ROM ONLY",
    0x01: "MBC1",
    0x02: "MBC1+RAM",
    0x03: "MBC1+RAM+BATTERY",
    0x05: "MBC2",
    0x06: "MBC2+BATTERY",
    0x08: "ROM+RAM",
    0x09: "ROM+RAM+BATTERY",
    0x0b: "MMM01",
    0x0c: "MMM01+RAM",
    0x0d: "MMM01+RAM+BATTERY",
    0x0f: "MBC3+TIMER+BATTERY",
    0x10: "MBC3+TIMER+RAM+BATTERY",
    0x11: "MBC3",
    0x12: "MBC3+RAM",
    0x13: "MBC3+RAM+BATTERY",
    0x19: "MBC5",
    0x1a: "MBC5+RAM",
    0x1b: "MBC5+RAM+BATTERY",
    0x1c: "MBC5+RUMBLE",
    0x1d: "MBC5+RUMBLE+RAM",
    0x1e: "MBC5+RUMBLE+RAM+BATTERY",
    0x20: "MBC6",
    0x22: "MBC7+SENSOR+RUMBLE+RAM+BATTERY",
    0xfc: "POCKET CAMERA",
    0xfd: "BANDAI TAMA5",
    0xfe: "HuC3",
    0xff: "HuC1+RAM+BATTERY",
  };
  const ROM_SIZES = {
    0x00: "32 KB (2 banks)",
    0x01: "64 KB (4 banks)",
    0x02: "128 KB (8 banks)",
    0x03: "256 KB (16 banks)",
    0x04: "512 KB (32 banks)",
    0x05: "1 MB (64 banks)",
    0x06: "2 MB (128 banks)",
    0x07: "4 MB (256 banks)",
    0x08: "8 MB (512 banks)",
  };
  const RAM_SIZES = {
    0x00: "None",
    0x01: "2 KB",
    0x02: "8 KB",
    0x03: "32 KB (4 banks)",
    0x04: "128 KB (16 banks)",
    0x05: "64 KB (8 banks)",
  };

  function parseCartHeader(bytes) {
    const titleBytes = bytes.slice(0x134, 0x144);
    let title = "";
    for (const b of titleBytes) {
      if (b >= 32 && b < 127) title += String.fromCharCode(b);
    }
    title = title.trim();

    const cgbFlag = bytes[0x143];
    const cartType = bytes[0x147];
    const romSizeCode = bytes[0x148];
    const ramSizeCode = bytes[0x149];
    const destCode = bytes[0x14a];
    const headerChecksum = bytes[0x14d];

    let computedChecksum = 0;
    for (let i = 0x134; i <= 0x14c; i++) {
      computedChecksum = (computedChecksum - bytes[i] - 1) & 0xff;
    }

    const globalStored = ((bytes[0x14e] << 8) | bytes[0x14f]) & 0xffff;
    let globalComputed = 0;
    for (let i = 0; i < bytes.length; i++) {
      if (i === 0x14e || i === 0x14f) continue;
      globalComputed = (globalComputed + bytes[i]) & 0xffff;
    }

    return {
      title: title || "(untitled)",
      cgb: cgbFlag === 0xc0 ? "CGB only" : cgbFlag === 0x80 ? "CGB compatible" : "DMG",
      cartType: CART_TYPES[cartType] || `Unknown ($${cartType.toString(16)})`,
      romSize: ROM_SIZES[romSizeCode] || `Unknown ($${romSizeCode.toString(16)})`,
      actualSize: `${(bytes.length / 1024).toFixed(0)} KB (${bytes.length.toLocaleString()} bytes)`,
      ramSize: RAM_SIZES[ramSizeCode] || `Unknown ($${ramSizeCode.toString(16)})`,
      destination: destCode === 0 ? "Japan" : "Non-Japan",
      headerChecksum: `$${headerChecksum.toString(16).toUpperCase().padStart(2, "0")} (${
        computedChecksum === headerChecksum ? "valid" : "INVALID"
      })`,
      globalChecksum: `$${globalStored.toString(16).toUpperCase().padStart(4, "0")} (${
        globalComputed === globalStored ? "valid" : "mismatch"
      })`,
    };
  }

  function renderCartInfo() {
    if (!cachedHeader) {
      cartInfoEl.innerHTML = "<dt>—</dt><dd>No cartridge loaded</dd>";
      return;
    }
    const h = cachedHeader;
    const rows = [
      ["Title", h.title],
      ["Type", h.cgb],
      ["MBC", h.cartType],
      ["ROM size", h.romSize],
      ["File size", h.actualSize],
      ["RAM size", h.ramSize],
      ["Region", h.destination],
      ["Header checksum", h.headerChecksum],
      ["Global checksum", h.globalChecksum],
    ];
    cartInfoEl.innerHTML = rows.map(([k, v]) => `<dt>${k}</dt><dd>${escapeHtml(String(v))}</dd>`).join("");
  }

  function escapeHtml(s) {
    return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  }

  function hex(v, width) {
    return "$" + v.toString(16).toUpperCase().padStart(width, "0");
  }

  function renderRegisters() {
    const a = exports.reg_a(),
      f = exports.reg_f(),
      b = exports.reg_b(),
      c = exports.reg_c(),
      d = exports.reg_d(),
      e = exports.reg_e(),
      h = exports.reg_h(),
      l = exports.reg_l(),
      sp = exports.reg_sp(),
      pc = exports.reg_pc();

    const cells = [
      ["A", a, 2],
      ["F", f, 2],
      ["B", b, 2],
      ["C", c, 2],
      ["D", d, 2],
      ["E", e, 2],
      ["H", h, 2],
      ["L", l, 2],
      ["SP", sp, 4],
      ["PC", pc, 4],
    ];
    regGridEl.innerHTML = cells
      .map(([name, val, w]) => `<div class="reg-cell"><span class="reg-name">${name}</span><span class="reg-value">${hex(val, w)}</span></div>`)
      .join("");

    const flags = [
      ["Z", (f & 0x80) !== 0],
      ["N", (f & 0x40) !== 0],
      ["H", (f & 0x20) !== 0],
      ["C", (f & 0x10) !== 0],
    ];
    flagRowEl.innerHTML = flags.map(([name, set]) => `<span class="flag-pill${set ? " set" : ""}">${name}</span>`).join("");

    const ime = exports.reg_ime();
    const halted = exports.reg_halted();
    const lo = exports.total_cycles_lo();
    const hi = exports.total_cycles_hi();
    const totalCycles = hi * 4294967296 + lo;
    cpuMiscEl.textContent = `IME ${ime ? "on" : "off"} · ${halted ? "HALTED" : "running"} · ${totalCycles.toLocaleString()} cycles`;
  }

  function renderDisasm() {
    if (!window.GBDisasm) return;
    const pc = exports.reg_pc();
    const peek = (addr) => exports.debug_peek(addr & 0xffff);
    const lines = [];
    let addr = pc;
    for (let i = 0; i < 14; i++) {
      const { text, len } = window.GBDisasm.disassemble(peek, addr);
      const line = `${addr.toString(16).toUpperCase().padStart(4, "0")}  ${text}`;
      lines.push(i === 0 ? `<span class="cur">${escapeHtml(line)}</span>` : escapeHtml(line));
      addr = (addr + Math.max(len, 1)) & 0xffff;
    }
    disasmEl.innerHTML = lines.join("\n");
  }

  const TILE_SHADES = [
    [224, 248, 208],
    [136, 192, 112],
    [52, 104, 86],
    [8, 24, 32],
  ];

  function renderTileViewer() {
    const ptr = exports.vram_ptr();
    const len = exports.vram_len();
    if (!ptr) return;
    const vram = new Uint8Array(memory.buffer, ptr, len);
    const scale = 2;
    const img = tileCtx.createImageData(256, 384);
    for (let t = 0; t < 384; t++) {
      const tileCol = t % 16;
      const tileRow = (t / 16) | 0;
      const base = t * 16;
      for (let y = 0; y < 8; y++) {
        const lo = vram[base + y * 2];
        const hi = vram[base + y * 2 + 1];
        for (let x = 0; x < 8; x++) {
          const bit = 7 - x;
          const color = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
          const [r, g, b] = TILE_SHADES[color];
          const px0 = (tileCol * 8 + x) * scale;
          const py0 = (tileRow * 8 + y) * scale;
          for (let sy = 0; sy < scale; sy++) {
            const rowStart = ((py0 + sy) * 256 + px0) * 4;
            for (let sx = 0; sx < scale; sx++) {
              const idx = rowStart + sx * 4;
              img.data[idx] = r;
              img.data[idx + 1] = g;
              img.data[idx + 2] = b;
              img.data[idx + 3] = 255;
            }
          }
        }
      }
    }
    tileCtx.putImageData(img, 0, 0);
  }

  function memRegionBytes(name) {
    switch (name) {
      case "wram": {
        const ptr = exports.wram_ptr();
        return ptr ? { base: 0xc000, bytes: new Uint8Array(memory.buffer, ptr, exports.wram_len()) } : null;
      }
      case "vram": {
        const ptr = exports.vram_ptr();
        return ptr ? { base: 0x8000, bytes: new Uint8Array(memory.buffer, ptr, exports.vram_len()) } : null;
      }
      case "oam": {
        const ptr = exports.oam_ptr();
        return ptr ? { base: 0xfe00, bytes: new Uint8Array(memory.buffer, ptr, exports.oam_len()) } : null;
      }
      case "hram": {
        const ptr = exports.hram_ptr();
        return ptr ? { base: 0xff80, bytes: new Uint8Array(memory.buffer, ptr, exports.hram_len()) } : null;
      }
      case "rom":
        return currentRomBytes ? { base: 0x0000, bytes: currentRomBytes } : null;
      default:
        return null;
    }
  }

  const BYTES_PER_ROW = 16;
  const ROWS_PER_PAGE = 16;
  const PAGE_SIZE = BYTES_PER_ROW * ROWS_PER_PAGE;

  function renderHexDump() {
    const region = memRegionBytes(memRegionName);
    if (!region) {
      hexdumpEl.textContent = "(not available)";
      return;
    }
    const { base, bytes } = region;
    const maxPage = Math.max(0, Math.ceil(bytes.length / PAGE_SIZE) - 1);
    memPage = Math.min(Math.max(0, memPage), maxPage);
    const start = memPage * PAGE_SIZE;

    const lines = [];
    for (let row = 0; row < ROWS_PER_PAGE; row++) {
      const rowOffset = start + row * BYTES_PER_ROW;
      if (rowOffset >= bytes.length) break;
      const hexParts = [];
      const asciiParts = [];
      for (let col = 0; col < BYTES_PER_ROW; col++) {
        const off = rowOffset + col;
        if (off < bytes.length) {
          const v = bytes[off];
          hexParts.push(v.toString(16).padStart(2, "0"));
          asciiParts.push(v >= 32 && v < 127 ? String.fromCharCode(v) : ".");
        } else {
          hexParts.push("  ");
          asciiParts.push(" ");
        }
      }
      const addr = base + rowOffset;
      lines.push(`${addr.toString(16).toUpperCase().padStart(4, "0")}  ${hexParts.join(" ")}  ${asciiParts.join("")}`);
    }
    hexdumpEl.textContent = lines.join("\n") || "(empty)";
  }

  function gotoMemAddress(text) {
    const cleaned = text.trim().replace(/^\$|^0x/i, "");
    const addr = parseInt(cleaned, 16);
    if (Number.isNaN(addr)) return;
    const region = memRegionBytes(memRegionName);
    if (!region) return;
    const offset = Math.max(0, addr - region.base);
    memPage = Math.floor(offset / PAGE_SIZE);
    renderHexDump();
  }

  function updateDebugger(force) {
    if (!romLoaded) return;
    renderRegisters();
    renderDisasm();
    debugFrameCount++;
    if (force || debugFrameCount % 6 === 0) {
      renderTileViewer();
      renderHexDump();
    }
  }

  function setupDebugger() {
    document.getElementById("btnDebugger").addEventListener("click", () => {
      debugVisible = !debugVisible;
      debuggerEl.hidden = !debugVisible;
      if (debugVisible) updateDebugger(true);
    });

    document.getElementById("dbgStep").addEventListener("click", () => {
      if (!romLoaded) return;
      // HALT idles 4 cycles at a time until the next interrupt. Stepping
      // "through" a HALT to wherever the CPU next does something real is far
      // more useful than making the user click thousands of times to get
      // there one idle tick at a time — cap the run so a program that
      // disabled interrupts and genuinely never wakes can't hang the button.
      let guard = 0;
      do {
        exports.step_instruction();
        guard++;
      } while (exports.reg_halted() && guard < 20000);
      renderFrame();
      updateDebugger(true);
    });

    document.getElementById("dbgStepFrame").addEventListener("click", () => {
      if (!romLoaded) return;
      exports.run_frame();
      pumpAudio();
      renderFrame();
      updateDebugger(true);
    });

    memRegionEl.addEventListener("change", (e) => {
      memRegionName = e.target.value;
      memPage = 0;
      renderHexDump();
    });

    document.getElementById("memPrev").addEventListener("click", () => {
      memPage = Math.max(0, memPage - 1);
      renderHexDump();
    });
    document.getElementById("memNext").addEventListener("click", () => {
      memPage++;
      renderHexDump();
    });

    memGotoEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter") gotoMemAddress(memGotoEl.value);
    });
    memGotoEl.addEventListener("blur", () => gotoMemAddress(memGotoEl.value));
  }

  function setupUiControls() {
    const pauseBtn = document.getElementById("btnPauseResume");
    pauseBtn.addEventListener("click", () => {
      paused = !paused;
      pauseBtn.textContent = paused ? "Resume" : "Pause";
      setOverlay(paused ? "Paused" : null);
    });

    document.getElementById("btnReset").addEventListener("click", () => {
      if (!romLoaded) return;
      exports.reset();
      setOverlay(null);
    });

    const muteBtn = document.getElementById("btnMute");
    muteBtn.addEventListener("click", () => {
      muted = !muted;
      muteBtn.textContent = muted ? "Unmute" : "Mute";
      if (gainNode) gainNode.gain.value = muted ? 0 : 0.5;
    });

    document.getElementById("romInput").addEventListener("change", async (e) => {
      const file = e.target.files[0];
      if (!file) return;
      const bytes = new Uint8Array(await file.arrayBuffer());
      await loadRomBytes(bytes);
      setStatus(`Loaded ${file.name} — press Start.`);
    });
  }

  async function main() {
    setupKeyboard();
    setupOnScreenButtons();
    setupUiControls();
    setupDebugger();

    window.addEventListener(
      "pointerdown",
      () => {
        initAudio();
        if (audioCtx && audioCtx.state === "suspended") audioCtx.resume();
      },
      { once: true }
    );
    window.addEventListener(
      "keydown",
      () => {
        initAudio();
        if (audioCtx && audioCtx.state === "suspended") audioCtx.resume();
      },
      { once: true }
    );

    try {
      await loadWasm();
      await loadDefaultCartridge();
    } catch (err) {
      console.error(err);
      setStatus("Failed to load emulator: " + err.message);
      setOverlay("Load error — see console");
      return;
    }

    requestAnimationFrame(tick);
  }

  main();
})();
