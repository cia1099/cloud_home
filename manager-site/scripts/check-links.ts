/**
 * Checks a built site (default report_web/): every href/src/refresh target resolves to a real file, internal page links end
 * in "/", asset files carry a content hash, no file is over 10 MB. Exits 1 with a list of problems.
 *   bun scripts/check-links.ts [dir]
 */
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';

const root = resolve(process.argv[2] ?? join(import.meta.dir, '..', 'report_web'));
const problems: string[] = [];
const files = readdirSync(root, { withFileTypes: true, recursive: true }).filter((e) => e.isFile());
const rel = (p: string): string => p.slice(root.length + 1);
let links = 0;

for (const f of files) {
  const abs = join(f.parentPath, f.name);
  if (statSync(abs).size > 10 * 1024 * 1024) problems.push(`${rel(abs)}: larger than 10 MB`);
  if (rel(abs).startsWith('assets/') && !/-[A-Za-z0-9_-]{8,}\.[a-z0-9]+$/.test(f.name)) problems.push(`${rel(abs)}: asset name has no content hash`);
  if (extname(f.name) !== '.html') continue;
  const html = readFileSync(abs, 'utf8');
  const targets = [...html.matchAll(/\b(?:href|src|poster)="([^"]*)"/g)].map((m) => m[1] ?? '');
  targets.push(...[...html.matchAll(/http-equiv="refresh" content="[^"]*url=([^"]+)"/g)].map((m) => m[1] ?? ''));
  for (const raw of targets) {
    if (!raw || /^(https?:|data:|mailto:|#)/.test(raw)) continue;
    links++;
    const [path = ''] = raw.split(/[?#]/);
    const target = resolve(dirname(abs), path);
    if (!target.startsWith(root)) {
      problems.push(`${rel(abs)}: ${raw} leaves the site`);
      continue;
    }
    const isPage = path.endsWith('/') || path === '';
    if (!isPage && !extname(path)) problems.push(`${rel(abs)}: page link "${raw}" must end in "/"`);
    const file = isPage ? join(target, 'index.html') : target;
    if (!existsSync(file)) problems.push(`${rel(abs)}: ${raw} -> missing ${rel(file)}`);
  }
}

if (!existsSync(join(root, 'index.html'))) problems.push('index.html is missing');
console.log(`${files.length} files, ${links} internal links checked`);
if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}
console.log('all links resolve');
