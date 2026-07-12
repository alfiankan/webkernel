const fs = require("fs");
const path = require("path");

async function main() {
  const wasmPath = path.join(__dirname, "../../emulator/target/wasm32-unknown-unknown/release/gbcore.wasm");
  const romPath = path.join(__dirname, "selftest.gb");

  const wasmBytes = fs.readFileSync(wasmPath);
  const romBytes = fs.readFileSync(romPath);

  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  const exp = instance.exports;
  const mem = new Uint8Array(exp.memory.buffer);

  const ptr = exp.alloc_rom_buffer(romBytes.length);
  new Uint8Array(exp.memory.buffer, ptr, romBytes.length).set(romBytes);
  exp.load_rom(romBytes.length);

  for (let i = 0; i < 5; i++) {
    exp.run_frame();
  }

  const fbPtr = exp.framebuffer_ptr();
  const fbLen = exp.framebuffer_len();
  const fb = new Uint8Array(exp.memory.buffer, fbPtr, fbLen);

  const px = (x, y) => {
    const idx = (y * 160 + x) * 4;
    return [fb[idx], fb[idx + 1], fb[idx + 2], fb[idx + 3]];
  };

  console.log("pixel(0,0) =", px(0, 0));
  console.log("pixel(50,50) =", px(50, 50));

  const expected = [0x08, 0x18, 0x20, 255];
  const got = px(0, 0);
  const ok = got.every((v, i) => v === expected[i]);
  console.log(ok ? "SELFTEST PASS" : "SELFTEST FAIL");
  process.exit(ok ? 0 : 1);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
