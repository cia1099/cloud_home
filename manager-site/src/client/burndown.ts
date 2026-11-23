import { $, $$, dateFmt, onLang, tr } from './dom.ts';

interface Data {
  points: { date: string; open: number }[];
  target: string | null;
}

const DAY = 864e5;
const day = (iso: string): number => Date.parse(`${iso}T12:00:00Z`) / DAY;

/** The chart is drawn at the container's pixel width so axis labels stay at their real size on a phone. */
function draw(host: HTMLElement, d: Data): void {
  const W = Math.max(280, Math.round(host.clientWidth || 640));
  const Hh = W < 420 ? 220 : 250;
  const [l, r, t, b] = [30, 12, 12, 30];
  const first = day(d.points[0]?.date ?? '');
  const lastPoint = d.points.at(-1);
  const last = day(lastPoint?.date ?? '');
  const target = d.target && day(d.target) > last ? day(d.target) : null;
  const end = target ?? last;
  const span = Math.max(end - first, 1);
  const max = Math.max(10, Math.ceil(Math.max(...d.points.map((p) => p.open)) / 10) * 10);
  const x = (v: number): number => l + ((v - first) / span) * (W - l - r);
  const y = (v: number): number => t + (1 - v / max) * (Hh - t - b);
  const pts = d.points.map((p) => [x(day(p.date)), y(p.open)] as const);
  const line = pts.map((p, i) => `${i ? 'L' : 'M'}${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join(' ');
  const fmt = dateFmt({ month: 'short', day: 'numeric' });
  const ticks = [0, 1, 2, 3].map((k) => {
    const v = first + (span * k) / 3;
    return `<text x="${x(v)}" y="${Hh - 8}" text-anchor="${k === 0 ? 'start' : k === 3 ? 'end' : 'middle'}">${fmt.format(new Date(v * DAY))}</text>`;
  });
  const grid = [0, 1, 2, 3, 4].map((k) => {
    const v = (max * k) / 4;
    return `<line class="grid-l" x1="${l}" x2="${W - r}" y1="${y(v)}" y2="${y(v)}"/><text x="${l - 8}" y="${y(v) + 4}" text-anchor="end">${Math.round(v)}</text>`;
  });
  const ideal = target && lastPoint ? `<line class="ideal" x1="${x(first)}" y1="${y(d.points[0]?.open ?? 0)}" x2="${x(target)}" y2="${y(0)}"/>` : '';
  host.innerHTML = `<svg class="chart" viewBox="0 0 ${W} ${Hh}" role="group" aria-label="${tr('chart.burndown')}"><defs><linearGradient id="bdg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--accent-fill)" stop-opacity=".28"/><stop offset="1" stop-color="var(--accent-fill)" stop-opacity="0"/></linearGradient></defs>${grid.join('')}${ticks.join('')}${ideal}<path class="area" d="${line} L${pts.at(-1)?.[0] ?? 0} ${y(0)} L${pts[0]?.[0] ?? 0} ${y(0)}Z"/><path class="line" d="${line}"/>${pts.map((p, i) => `<circle class="pt" cx="${p[0]}" cy="${p[1]}" r="5" tabindex="0" data-i="${i}" role="img" aria-label="${d.points[i]?.date}: ${d.points[i]?.open}"/>`).join('')}</svg><div class="tip" aria-live="polite"></div>`;
  const line$ = $('.line', host) as unknown as SVGPathElement | null;
  line$?.style.setProperty('--len', String(line$.getTotalLength()));
  const tip = $('.tip', host);
  const show = (i: number): void => {
    const p = d.points[i];
    if (tip && p) tip.innerHTML = `<b>${fmt.format(new Date(day(p.date) * DAY))}</b> · ${tr('chart.open_n', { n: p.open })}`;
  };
  show(d.points.length - 1);
  for (const c of $$('.pt', host)) for (const ev of ['pointerenter', 'focus']) c.addEventListener(ev, () => show(Number(c.dataset.i)));
  if (host.closest('.in')) host.classList.add('in');
}

export function initBurndown(): void {
  for (const host of $$('[data-burn]')) {
    const data = JSON.parse(host.dataset.burn ?? '{}') as Data;
    let width = 0;
    const render = (): void => {
      width = host.clientWidth;
      draw(host, data);
    };
    onLang(render);
    new ResizeObserver(() => {
      if (Math.abs(host.clientWidth - width) > 8) render();
    }).observe(host);
    new IntersectionObserver((es, o) => {
      if (es.some((e) => e.isIntersecting)) {
        host.classList.add('in');
        o.disconnect();
      }
    }).observe(host);
  }
}
