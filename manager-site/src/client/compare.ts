import { $, $$, onLang, reducedMotion, tr } from './dom.ts';

/** Before/after: the native range input is the control, so arrow keys, Home/End, touch and screen readers work. */
export function initCompare(): void {
  for (const ba of $$('.ba')) {
    const input = $<HTMLInputElement>('input', ba);
    if (!input) continue;
    const set = (v: number): void => {
      ba.style.setProperty('--p', `${v}%`);
      input.setAttribute('aria-valuetext', tr('ui.compare_value', { b: Math.round(100 - v), a: Math.round(v) }));
    };
    input.addEventListener('input', () => set(Number(input.value)));
    onLang(() => set(Number(input.value)));
    if (reducedMotion()) continue;
    const touched = (): void => {
      ba.dataset.touched = '1';
    };
    input.addEventListener('pointerdown', touched);
    input.addEventListener('keydown', touched);
    new IntersectionObserver(
      (entries, o) => {
        if (!entries[0]?.isIntersecting) return;
        o.disconnect();
        const t0 = performance.now();
        const sweep = (t: number): void => {
          if (ba.dataset.touched) return;
          const k = Math.min((t - t0) / 1100, 1);
          const v = 92 - 42 * (1 - Math.pow(1 - k, 3));
          set(v);
          input.value = String(v);
          if (k < 1) requestAnimationFrame(sweep);
        };
        sweep(t0);
      },
      { threshold: 0.6 },
    ).observe(ba);
  }
}
