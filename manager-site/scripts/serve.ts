/**
 * Static server that behaves like the real one: python3 -m http.server, no rewrites, no SPA fallback.
 * It also answers Range requests (the deployed server must, for video).
 *   bun scripts/serve.ts            serve report_web/ on :4173 (PORT, OUT_DIR override)
 *   bun scripts/serve.ts --watch    rebuild when the site source, reports/, SPEC.md or plan/ change
 */
import { existsSync, statSync, watch } from 'node:fs';
import { join, normalize, resolve } from 'node:path';

const root = resolve(import.meta.dir, '..', process.env.OUT_DIR ?? 'report_web');
const port = Number(process.env.PORT ?? 4173);

function respond(req: Request): Response {
  const url = new URL(req.url);
  const rel = normalize(decodeURIComponent(url.pathname));
  let file = join(root, rel);
  if (!file.startsWith(root)) return new Response('forbidden', { status: 403 });
  if (existsSync(file) && statSync(file).isDirectory()) {
    if (!url.pathname.endsWith('/')) return new Response(null, { status: 301, headers: { location: `${url.pathname}/${url.search}` } });
    file = join(file, 'index.html');
  }
  if (!existsSync(file)) return new Response('404 not found', { status: 404 });
  const f = Bun.file(file);
  const range = /^bytes=(\d*)-(\d*)$/.exec(req.headers.get('range') ?? '');
  if (!range) return new Response(f, { headers: { 'accept-ranges': 'bytes' } });
  const size = f.size;
  const start = range[1] ? Number(range[1]) : Math.max(size - Number(range[2]), 0);
  const end = range[1] && range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
  if (start > end || start >= size) return new Response(null, { status: 416, headers: { 'content-range': `bytes */${size}` } });
  return new Response(f.slice(start, end + 1), { status: 206, headers: { 'content-range': `bytes ${start}-${end}/${size}`, 'accept-ranges': 'bytes' } });
}

Bun.serve({ port, fetch: respond });
console.log(`serving ${root} on http://localhost:${port}/`);

if (process.argv.includes('--watch')) {
  const site = resolve(import.meta.dir, '..');
  const repo = resolve(site, '..');
  let timer: ReturnType<typeof setTimeout> | undefined;
  let building = false;
  const rebuild = (): void => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      if (building) return;
      building = true;
      const p = Bun.spawn(['bun', 'run', 'build'], { cwd: site, stdout: 'inherit', stderr: 'inherit' });
      await p.exited;
      building = false;
    }, 300);
  };
  for (const p of [join(site, 'src'), join(site, 'content'), join(repo, 'reports'), join(repo, 'plan'), join(repo, 'SPEC.md')]) {
    if (existsSync(p)) watch(p, { recursive: true }, rebuild);
  }
  rebuild();
}
