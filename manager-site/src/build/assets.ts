import { copyFileSync, existsSync, mkdirSync, readFileSync, statSync } from 'node:fs';
import { basename, extname, join } from 'node:path';
import { imageSize } from './imagesize.ts';
import { OUT_DIR } from './util.ts';

const MAX_FILE = 10 * 1024 * 1024;
const MAX_REPORT = 25 * 1024 * 1024;

/**
 * Every file a report uses is copied to assets/media/<name>-<content hash>.<ext>, so Cloudflare can cache it for
 * hours without ever serving stale bytes. Limits: 10 MB per file, 25 MB per report (the build fails otherwise).
 */
export class Assets {
  private byPath = new Map<string, string>();
  private perReport = new Map<string, Map<string, number>>();

  private reportsDir: string;

  constructor(reportsDir: string) {
    this.reportsDir = reportsDir;
  }

  /** `rel` is relative to reports/<reportId>/; returns the site-root-relative URL, e.g. assets/media/shot-1a2b3c4d.webp. */
  media(reportId: string, rel: string): string {
    const abs = join(this.reportsDir, reportId, rel);
    if (!existsSync(abs)) throw new Error(`reports/${reportId}: media file "${rel}" does not exist`);
    const url = this.publish(abs, `reports/${reportId}/${rel}`);
    const files = this.perReport.get(reportId) ?? new Map<string, number>();
    files.set(url, statSync(abs).size);
    this.perReport.set(reportId, files);
    const total = [...files.values()].reduce((a, b) => a + b, 0);
    if (total > MAX_REPORT) throw new Error(`reports/${reportId}: media is ${(total / 1048576).toFixed(1)} MB, over the 25 MB per-report limit`);
    return url;
  }

  /** A file from anywhere in the repository (for example design/brand/): same hashing and size limit, no per-report budget. */
  file(abs: string): string {
    if (!existsSync(abs)) throw new Error(`${basename(abs)}: file does not exist`);
    return this.publish(abs, abs);
  }

  private publish(abs: string, label: string): string {
    const cached = this.byPath.get(abs);
    if (cached) return cached;
    const size = statSync(abs).size;
    if (size > MAX_FILE) throw new Error(`${label}: ${(size / 1048576).toFixed(1)} MB exceeds the 10 MB per-file limit`);
    const hash = new Bun.CryptoHasher('sha256').update(readFileSync(abs)).digest('hex').slice(0, 10);
    const ext = extname(abs);
    const slug = basename(abs, ext).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'file';
    const url = `assets/media/${slug}-${hash}${ext.toLowerCase()}`;
    mkdirSync(join(OUT_DIR, 'assets/media'), { recursive: true });
    copyFileSync(abs, join(OUT_DIR, url));
    this.byPath.set(abs, url);
    return url;
  }

  /** Pixel size for width/height attributes, so images never shift the layout. */
  size(reportId: string, rel: string): { w: number; h: number } {
    const dim = imageSize(join(this.reportsDir, reportId, rel));
    if (!dim) throw new Error(`reports/${reportId}/${rel}: cannot read the image size (use png, jpeg, webp or svg)`);
    return dim;
  }
}
