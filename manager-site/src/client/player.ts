import { ic } from '../shared/icons.ts';
import { $, $$, onLang, reducedMotion, tr } from './dom.ts';

/**
 * Walkthrough player: scenes advance on a timer, every step is also a button. Autoplays only while at least 60 % visible,
 * never with reduced motion, and a user choice (pause, step) stops autoplay for good.
 */
export function initPlayers(): void {
  const rm = reducedMotion();
  for (const pl of $$('.player')) {
    const scenes = $$('.p-scene', pl);
    const steps = $$('.p-steps button', pl);
    const play = $('.p-play', pl);
    const cap = $('.p-cap', pl);
    const time = $('.p-time', pl);
    const track = $('.p-track i', pl);
    if (!play || !cap || !time || !track) continue;
    const N = scenes.length;
    const duration = Number(pl.dataset.duration) || 3400;
    let i = 0;
    let playing = false;
    let timer = 0;
    let user = false;

    const label = (): void => {
      const atEnd = i === N - 1 && !playing;
      play.setAttribute('aria-label', tr(playing ? 'ui.pause' : atEnd ? 'ui.replay' : 'ui.play'));
      play.setAttribute('aria-pressed', String(playing));
      play.innerHTML = ic(playing ? 'pause' : atEnd ? 'refresh' : 'play');
    };
    const go = (n: number): void => {
      i = (n + N) % N;
      scenes.forEach((s, k) => {
        s.classList.toggle('on', k === i);
        const v = $<HTMLVideoElement>('video', s);
        if (v) {
          if (k === i && playing) void v.play();
          else v.pause();
        }
      });
      steps.forEach((b, k) => b.setAttribute('aria-current', String(k === i)));
      cap.innerHTML = $('.s-cap', steps[i] as HTMLElement)?.innerHTML ?? '';
      time.textContent = `${i + 1} / ${N}`;
      track.style.animation = 'none';
      track.style.transform = `scaleX(${(i + (rm ? 1 : 0)) / N})`;
      void track.offsetWidth;
      if (playing && !rm) {
        track.style.transform = '';
        track.style.animation = `pfill ${duration}ms linear forwards`;
        track.style.setProperty('--from', String(i / N));
        track.style.setProperty('--to', String((i + 1) / N));
      }
      label();
    };
    const next = (): void => {
      clearTimeout(timer);
      if (!playing) return;
      timer = window.setTimeout(() => {
        if (i === N - 1) {
          playing = false;
          label();
          return;
        }
        go(i + 1);
        next();
      }, duration);
    };
    const start = (fromUser: boolean): void => {
      if (fromUser) user = true;
      if (!playing && i === N - 1) go(0);
      playing = true;
      go(i);
      next();
    };
    const stop = (fromUser: boolean): void => {
      if (fromUser) user = true;
      playing = false;
      clearTimeout(timer);
      track.style.animation = 'none';
      track.style.transform = `scaleX(${(i + 1) / N})`;
      $<HTMLVideoElement>('video', scenes[i] as HTMLElement)?.pause();
      label();
    };
    play.addEventListener('click', () => (playing ? stop(true) : start(true)));
    steps.forEach((b, k) =>
      b.addEventListener('click', () => {
        user = true;
        go(k);
        if (playing) next();
      }),
    );
    pl.addEventListener('keydown', (e) => {
      if (!(e.target as Element).closest('.p-steps') || (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft')) return;
      e.preventDefault();
      const to = (i + (e.key === 'ArrowRight' ? 1 : -1) + N) % N;
      steps[to]?.focus();
      user = true;
      go(to);
    });
    onLang(() => {
      label();
      go(i);
    });
    stop(false);
    if (rm) continue;
    new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          if (!user && !playing) start(false);
        } else if (playing) stop(false);
      },
      { threshold: 0.6 },
    ).observe(pl);
    document.addEventListener('visibilitychange', () => {
      if (document.hidden && playing) stop(false);
    });
  }
}
