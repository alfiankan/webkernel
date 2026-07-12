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

  let exports = null;
  let memory = null;
  let romLoaded = false;
  let paused = false;
  let muted = false;
  let audioCtx = null;
  let nextAudioTime = 0;
  let gainNode = null;

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
    if (!romLoaded || paused) {
      lastTime = now;
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

  function setupKeyboard() {
    const held = new Set();
    window.addEventListener("keydown", (e) => {
      const name = KEY_MAP[e.code];
      if (!name) return;
      e.preventDefault();
      if (held.has(name)) return;
      held.add(name);
      pressButton(name);
    });
    window.addEventListener("keyup", (e) => {
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
