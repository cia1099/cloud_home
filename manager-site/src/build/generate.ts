import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { extname, join } from 'node:path';
import { Assets } from './assets.ts';
import { decisions, latestRedirect, overview, product, reportPage, reportsIndex, requirements, timeline, type Rendered } from './render/pages.ts';
import { shell, type Bundle } from './render/layout.ts';
import { loadSite } from './site.ts';
import { OUT_DIR, readJson, REPORTS_DIR, warnings } from './util.ts';

/** Same patterns as scripts/deploy_report.sh, plus e-mail addresses: nothing here may ever be published. */
const FORBIDDEN =
  /SITE_TOKEN|JWT_SECRET|ADMIN_PASSWORD|MINISIGN_SECRET|CHAT_BOT_TOKEN|chat\.env|[0-9]{8,10}:[A-Za-z0-9_-]{35}|BEGIN ([A-Z]+ )?PRIVATE KEY|\/Users\/|cia1099@|\.ssh\/|manager\.env|[\w.+-]+@[\w-]+\.[a-z]{2,}/;

function readBundle(): Bundle {
  const manifest = readJson(join(OUT_DIR, '.vite/manifest.json')) as Record<string, { file: string; css?: string[]; isEntry?: boolean }>;
  const entry = Object.values(manifest).find((m) => m.isEntry);
  if (!entry) throw new Error('Vite manifest has no entry; run "vite build" first');
  return { js: entry.file, css: entry.css ?? [] };
}

function write(path: string, html: string): void {
  const file = join(OUT_DIR, path, 'index.html');
  mkdirSync(join(OUT_DIR, path), { recursive: true });
  writeFileSync(file, html);
}

function scanOutput(dir: string): void {
  const TEXT = new Set(['.html', '.css', '.js', '.json', '.svg', '.txt']);
  for (const e of readdirSync(dir, { withFileTypes: true, recursive: true })) {
    if (!e.isFile() || !TEXT.has(extname(e.name))) continue;
    const file = join(e.parentPath, e.name);
    const m = FORBIDDEN.exec(readFileSync(file, 'utf8'));
    if (m) throw new Error(`refusing to publish: ${file.slice(OUT_DIR.length + 1)} contains "${m[0]}"`);
  }
}

const bundle = readBundle();
const site = loadSite();
const assets = new Assets(REPORTS_DIR);
const render = (r: Rendered): string => shell(site, bundle, r.page, r.main);

const pages: Rendered[] = [overview(site), reportsIndex(site), timeline(site), requirements(site), product(site, assets), decisions(site)];
for (const r of site.reports) pages.push(await reportPage(site, assets, r));
for (const p of pages) write(p.page.path, render(p));
if (site.latest) write('/latest/', latestRedirect(site.latest));

rmSync(join(OUT_DIR, '.vite'), { recursive: true, force: true });
scanOutput(OUT_DIR);
for (const w of warnings) console.warn(`warning: ${w}`);
console.log(`built ${pages.length + (site.latest ? 1 : 0)} pages, ${site.reports.length} report(s), into ${OUT_DIR}`);
