import { ic } from '../shared/icons.ts';
import { $, $$, onLang, reducedMotion, tr } from './dom.ts';

/** One shared viewer for every gallery on the page; the page behind it is inert while it is open. */
export function initLightbox(): void {
  if (!$('[data-lb]')) return;
  const lb = document.createElement('div');
  lb.className = 'lb';
  lb.setAttribute('role', 'dialog');
  lb.setAttribute('aria-modal', 'true');
  lb.hidden = true;
  lb.innerHTML = `<div class="lb-bar"><span class="lb-n num"></span><button class="icon-btn lb-x" type="button">${ic('xmark')}</button></div><div class="lb-stage"><button class="icon-btn lb-nav p" type="button">${ic('chevL')}</button><div class="lb-art"></div><button class="icon-btn lb-nav n" type="button">${ic('chev')}</button></div><div class="lb-cap" aria-live="polite"></div>`;
  document.body.appendChild(lb);
  const q = <T extends HTMLElement>(s: string): T => $<T>(s, lb) as T;
  let shots: HTMLElement[] = [];
  let i = 0;
  let opener: HTMLElement | null = null;
  let start: [number, number] | null = null;
  const inertTargets = (): HTMLElement[] => [...document.body.children].filter((el): el is HTMLElement => el instanceof HTMLElement && el !== lb);
  const show = (n: number): void => {
    i = (n + shots.length) % shots.length;
    const s = shots[i];
    const img = s?.querySelector('img');
    if (!s || !img) return;
    const big = new Image();
    big.src = img.currentSrc || img.src;
    big.width = img.width;
    big.height = img.height;
    big.alt = img.alt;
    q('.lb-art').replaceChildren(big);
    q('.lb-cap').innerHTML = $('.cap', s)?.innerHTML ?? '';
    q('.lb-n').textContent = `${i + 1} / ${shots.length}`;
  };
  const open = (group: HTMLElement[], n: number): void => {
    opener = document.activeElement as HTMLElement | null;
    shots = group;
    show(n);
    lb.hidden = false;
    requestAnimationFrame(() => lb.classList.add('open'));
    document.body.classList.add('noscroll');
    for (const el of inertTargets()) el.inert = true;
    q('.lb-x').focus();
  };
  const close = (): void => {
    lb.classList.remove('open');
    document.body.classList.remove('noscroll');
    for (const el of inertTargets()) el.inert = false;
    setTimeout(() => {
      lb.hidden = true;
    }, reducedMotion() ? 0 : 350);
    opener?.focus();
  };
  onLang(() => {
    lb.setAttribute('aria-label', tr('ui.viewer'));
    q('.lb-x').setAttribute('aria-label', tr('ui.close'));
    q('.lb-nav.p').setAttribute('aria-label', tr('ui.prev'));
    q('.lb-nav.n').setAttribute('aria-label', tr('ui.next'));
    if (!lb.hidden) show(i);
  });
  for (const g of $$('[data-gallery]')) {
    const group = $$('[data-lb]', g);
    group.forEach((s, n) => s.addEventListener('click', () => open(group, n)));
  }
  q('.lb-x').addEventListener('click', close);
  q('.lb-nav.p').addEventListener('click', () => show(i - 1));
  q('.lb-nav.n').addEventListener('click', () => show(i + 1));
  lb.addEventListener('click', (e) => {
    const t = e.target as HTMLElement;
    if (t === lb || t.classList.contains('lb-stage')) close();
  });
  lb.addEventListener('pointerdown', (e) => {
    if (e.pointerType === 'touch') start = [e.clientX, e.clientY];
  });
  lb.addEventListener('pointerup', (e) => {
    if (!start) return;
    const dx = e.clientX - start[0];
    const dy = e.clientY - start[1];
    start = null;
    if (Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy)) show(i + (dx < 0 ? 1 : -1));
    else if (dy > 90) close();
  });
  document.addEventListener('keydown', (e) => {
    if (lb.hidden) return;
    if (e.key === 'Escape') close();
    if (e.key === 'ArrowLeft') show(i - 1);
    if (e.key === 'ArrowRight') show(i + 1);
    if (e.key === 'Tab') {
      e.preventDefault();
      const f = $$('button', lb);
      const a = f.indexOf(document.activeElement as HTMLElement);
      f[(a + (e.shiftKey ? -1 : 1) + f.length) % f.length]?.focus();
    }
  });
}
