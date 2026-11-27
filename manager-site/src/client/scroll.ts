import { $, $$, reducedMotion } from './dom.ts';

function spy(links: HTMLElement[], targets: HTMLElement[]): void {
  if (!links.length) return;
  const byId = new Map(links.map((a) => [(a.getAttribute('href') ?? '').slice(1), a]));
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        for (const a of links) a.classList.remove('on');
        byId.get(e.target.id)?.classList.add('on');
      }
    },
    { rootMargin: '-30% 0px -60% 0px' },
  );
  for (const t of targets) io.observe(t);
}

/** Section highlighting for the report contents list, and the helpers of bespoke milestone pages. */
export function initScroll(): void {
  spy($$('.toc a'), $$('.rsec[id]'));
  spy($$('.ms-nav a'), $$('.ms-sec[id]'));

  const steps = $$('.wstep');
  if (steps.length) {
    const screens = $$('.screen');
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (!e.isIntersecting) continue;
          for (const s of steps) s.classList.toggle('on', s === e.target);
          for (const s of screens) s.classList.toggle('on', s.dataset.i === (e.target as HTMLElement).dataset.i);
        }
      },
      { rootMargin: '-45% 0px -45% 0px' },
    );
    for (const s of steps) io.observe(s);
    steps[0]?.classList.add('on');
  }

  const device = $('.ms-hero .device');
  if (device && !reducedMotion()) {
    const tilt = (): void => {
      const k = Math.min(scrollY / 420, 1);
      device.style.setProperty('--rx', `${(8 * (1 - k)).toFixed(2)}deg`);
      device.style.setProperty('--ty', `${(-24 * k).toFixed(1)}px`);
    };
    addEventListener('scroll', tilt, { passive: true });
    tilt();
  }
}
