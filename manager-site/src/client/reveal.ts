import { $$, reducedMotion } from './dom.ts';

/** Scroll reveal and count-up numbers; with reduced motion everything shows its final state at once. */
export function initReveal(): void {
  const rm = reducedMotion();
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        e.target.classList.add('in');
        io.unobserve(e.target);
      }
    },
    { threshold: 0.15, rootMargin: '0px 0px -6% 0px' },
  );
  for (const el of $$('.rv, .io')) (rm ? el.classList.add('in') : io.observe(el));

  const counters = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        counters.unobserve(e.target);
        const el = e.target as HTMLElement;
        const to = Number(el.dataset.count);
        const t0 = performance.now();
        const step = (t: number): void => {
          const k = Math.min((t - t0) / 1100, 1);
          el.textContent = String(Math.round(to * (1 - Math.pow(1 - k, 4))));
          if (k < 1) requestAnimationFrame(step);
        };
        requestAnimationFrame(step);
      }
    },
    { threshold: 0.5 },
  );
  if (rm) return;
  for (const el of $$('[data-count]')) {
    el.textContent = '0';
    counters.observe(el);
  }
}
