// 把原版纯黑图标重着色为「墨蓝」，产出一张源 PNG，供 `tauri icon` 重新生成全套图标。
// 纯 Node 内置依赖 (zlib)，无需第三方库。
// 用法: node scripts/recolor-icon.mjs <输入png> <输出png>
import { readFileSync, writeFileSync } from "node:fs";
import { inflateSync, deflateSync } from "node:zlib";

const INK = [0x14, 0x30, 0x4d]; // 墨蓝 #14304d

// ── CRC32 (PNG chunk 校验) ──
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function readChunks(buf) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (!buf.subarray(0, 8).equals(sig)) throw new Error("not a PNG");
  const chunks = [];
  let off = 8;
  while (off < buf.length) {
    const len = buf.readUInt32BE(off);
    const type = buf.toString("ascii", off + 4, off + 8);
    const data = buf.subarray(off + 8, off + 8 + len);
    chunks.push({ type, data });
    off += 12 + len;
  }
  return chunks;
}

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

// 解码为 RGBA8 像素 (Uint8Array, w*h*4)。支持 colorType 0/2/3/6, bitDepth 8。
function decode(buf) {
  const chunks = readChunks(buf);
  const ihdr = chunks.find((c) => c.type === "IHDR").data;
  const width = ihdr.readUInt32BE(0);
  const height = ihdr.readUInt32BE(4);
  const bitDepth = ihdr[8];
  const colorType = ihdr[9];
  if (bitDepth !== 8) throw new Error("only bitDepth 8 supported, got " + bitDepth);

  const plte = chunks.find((c) => c.type === "PLTE")?.data;
  const trns = chunks.find((c) => c.type === "tRNS")?.data;

  const idat = Buffer.concat(chunks.filter((c) => c.type === "IDAT").map((c) => c.data));
  const raw = inflateSync(idat);

  const channels = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[colorType];
  if (!channels) throw new Error("unsupported colorType " + colorType);
  const bpp = channels;
  const stride = width * bpp;

  // 去滤波 → 原始样本
  const samples = Buffer.alloc(height * stride);
  let pos = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[pos++];
    const line = raw.subarray(pos, pos + stride);
    pos += stride;
    const out = samples.subarray(y * stride, y * stride + stride);
    const prev = y > 0 ? samples.subarray((y - 1) * stride, (y - 1) * stride + stride) : null;
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? out[x - bpp] : 0;
      const b = prev ? prev[x] : 0;
      const c = prev && x >= bpp ? prev[x - bpp] : 0;
      let v = line[x];
      if (filter === 1) v = (v + a) & 0xff;
      else if (filter === 2) v = (v + b) & 0xff;
      else if (filter === 3) v = (v + ((a + b) >> 1)) & 0xff;
      else if (filter === 4) v = (v + paeth(a, b, c)) & 0xff;
      out[x] = v;
    }
  }

  // → RGBA
  const rgba = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) {
    let r, g, b, al;
    if (colorType === 6) { r = samples[i * 4]; g = samples[i * 4 + 1]; b = samples[i * 4 + 2]; al = samples[i * 4 + 3]; }
    else if (colorType === 2) { r = samples[i * 3]; g = samples[i * 3 + 1]; b = samples[i * 3 + 2]; al = 255; }
    else if (colorType === 0) { r = g = b = samples[i]; al = 255; }
    else { // palette
      const idx = samples[i];
      r = plte[idx * 3]; g = plte[idx * 3 + 1]; b = plte[idx * 3 + 2];
      al = trns && idx < trns.length ? trns[idx] : 255;
    }
    rgba[i * 4] = r; rgba[i * 4 + 1] = g; rgba[i * 4 + 2] = b; rgba[i * 4 + 3] = al;
  }
  return { width, height, rgba };
}

// 黑→墨蓝重着色: 按暗度在「白底」与「墨蓝」之间线性混合, 保留抗锯齿边缘与 alpha。
function recolor(rgba) {
  for (let i = 0; i < rgba.length; i += 4) {
    const r = rgba[i], g = rgba[i + 1], b = rgba[i + 2];
    const lum = 0.299 * r + 0.587 * g + 0.114 * b;
    const f = (255 - lum) / 255; // 0=白, 1=黑
    rgba[i] = Math.round(255 * (1 - f) + INK[0] * f);
    rgba[i + 1] = Math.round(255 * (1 - f) + INK[1] * f);
    rgba[i + 2] = Math.round(255 * (1 - f) + INK[2] * f);
    // alpha 不变
  }
}

// 编码 RGBA8 → PNG (filter 0)
function encode(width, height, rgba) {
  const stride = width * 4;
  const raw = Buffer.alloc(height * (stride + 1));
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0; // filter none
    Buffer.from(rgba.buffer, y * stride, stride).copy(raw, y * (stride + 1) + 1);
  }
  const idat = deflateSync(raw, { level: 9 });

  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length, 0);
    const typeBuf = Buffer.from(type, "ascii");
    const body = Buffer.concat([typeBuf, data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body), 0);
    return Buffer.concat([len, body, crc]);
  };

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;  // bitDepth
  ihdr[9] = 6;  // colorType RGBA
  ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;

  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
}

const [, , inPath, outPath] = process.argv;
const { width, height, rgba } = decode(readFileSync(inPath));
recolor(rgba);
writeFileSync(outPath, encode(width, height, rgba));
console.log(`recolored ${inPath} (${width}x${height}) → ${outPath} [ink #14304d]`);
