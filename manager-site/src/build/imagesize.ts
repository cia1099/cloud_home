import { readFileSync } from 'node:fs';
import { extname } from 'node:path';

export interface Dim {
  w: number;
  h: number;
}

function png(b: Buffer): Dim | null {
  return b.length > 24 && b.toString('latin1', 1, 4) === 'PNG' ? { w: b.readUInt32BE(16), h: b.readUInt32BE(20) } : null;
}

function jpeg(b: Buffer): Dim | null {
  if (b[0] !== 0xff || b[1] !== 0xd8) return null;
  let i = 2;
  while (i + 9 < b.length) {
    if (b[i] !== 0xff) return null;
    const marker = b[i + 1] ?? 0;
    if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) return { h: b.readUInt16BE(i + 5), w: b.readUInt16BE(i + 7) };
    i += 2 + b.readUInt16BE(i + 2);
  }
  return null;
}

function webp(b: Buffer): Dim | null {
  if (b.toString('latin1', 0, 4) !== 'RIFF' || b.toString('latin1', 8, 12) !== 'WEBP') return null;
  const kind = b.toString('latin1', 12, 16);
  if (kind === 'VP8X') return { w: 1 + b.readUIntLE(24, 3), h: 1 + b.readUIntLE(27, 3) };
  if (kind === 'VP8L') {
    const v = b.readUInt32LE(21);
    return { w: 1 + (v & 0x3fff), h: 1 + ((v >> 14) & 0x3fff) };
  }
  if (kind === 'VP8 ') return { w: b.readUInt16LE(26) & 0x3fff, h: b.readUInt16LE(28) & 0x3fff };
  return null;
}

function svg(text: string): Dim | null {
  const vb = /viewBox="[\d.\-]+[ ,]+[\d.\-]+[ ,]+([\d.]+)[ ,]+([\d.]+)"/.exec(text);
  return vb ? { w: Math.round(Number(vb[1])), h: Math.round(Number(vb[2])) } : null;
}

/** Reads width and height from the file header (png, jpeg, webp) or the viewBox (svg); no dependencies. */
export function imageSize(path: string): Dim | null {
  const b = readFileSync(path);
  switch (extname(path).toLowerCase()) {
    case '.png':
      return png(b);
    case '.jpg':
    case '.jpeg':
      return jpeg(b);
    case '.webp':
      return webp(b);
    case '.svg':
      return svg(b.toString('utf8', 0, 2000));
    default:
      return null;
  }
}
