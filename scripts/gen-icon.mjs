// 生成 PixelForge AI 应用图标源图：16x16 像素画墨盒，放大到 512x512
// 仅依赖 Node 标准库（zlib），手工编码 PNG
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// 16x16 像素画：墨盒（. 透明  B 机身暗色  b 机身亮色  A 琥珀色标签  D 深色标签文字）
const GRID = [
  "................",
  "..BBBBBBBBBBBB..",
  ".BBBBBBBBBBBBBB.",
  ".BAaaaaaaaaaaAB.",
  ".BAaaaaaaaaaaAB.",
  ".BADDAAAAAADDAB.",
  ".BADAAAAAAADDAB.",
  ".BADDAAAAAADDAB.",
  ".BAaaaaaaaaaaAB.",
  ".BAaaAAaaAAaaAB.",
  ".BAaaAAaaAAaaAB.",
  ".BAaaaaaaaaaaAB.",
  ".BBBAABBBBBAABB.",
  ".BBBAABBBBBAABB.",
  "..BBBBBBBBBBBB..",
  "................",
];

const PALETTE = {
  ".": [0, 0, 0, 0], // 透明
  B: [40, 43, 50, 255], // 机身
  b: [58, 62, 72, 255], // 机身亮部
  A: [232, 163, 61, 255], // 琥珀标签
  D: [45, 32, 12, 255], // 标签深色
  G: [106, 190, 120, 255], // 辅助绿（像素点缀）
};

const SRC = 16;
const SCALE = 32; // 16 * 32 = 512
const OUT = SRC * SCALE;

const raw = Buffer.alloc(OUT * (OUT * 4 + 1));
for (let y = 0; y < OUT; y++) {
  const rowStart = y * (OUT * 4 + 1);
  raw[rowStart] = 0; // PNG filter: None
  const gy = Math.floor(y / SCALE);
  for (let x = 0; x < OUT; x++) {
    const gx = Math.floor(x / SCALE);
    const ch = GRID[gy]?.[gx] ?? ".";
    // 机身右侧与底部加 1 像素亮边，制造轻微立体感
    let color = PALETTE[ch] ?? PALETTE["."];
    if (ch === "B" && (gx === 13 || gy === 13)) color = PALETTE["b"];
    const o = rowStart + 1 + x * 4;
    raw[o] = color[0];
    raw[o + 1] = color[1];
    raw[o + 2] = color[2];
    raw[o + 3] = color[3];
  }
}

function crc32(buf) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c;
    }
  }
  let crc = -1;
  for (let i = 0; i < buf.length; i++) crc = (crc >>> 8) ^ table[(crc ^ buf[i]) & 0xff];
  return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(OUT, 0);
ihdr.writeUInt32BE(OUT, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const outPath = resolve(ROOT, "src-tauri/icons/source.png");
mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, png);
console.log(`written: ${outPath} (${OUT}x${OUT})`);
