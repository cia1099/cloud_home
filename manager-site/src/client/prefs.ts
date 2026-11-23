import { ic } from '../shared/icons.ts';
import type { Lang } from '../shared/i18n.ts';
import { $, $$, lang, tr } from './dom.ts';

type Theme = 'auto' | 'light' | 'dark';
const H = document.documentElement;
const dark = matchMedia('(prefers-color-scheme: dark)');

function store(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* storage can be blocked (private mode); the choice then lasts for this page only */
  }
}

function setLang(l: Lang, persist: boolean): void {
  H.dataset.l = l;
  H.lang = l === 'zh' ? 'zh-Hans' : 'en';
  if (persist) store('chL', l);
  for (const b of $$('[data-setl]')) b.setAttribute('aria-pressed', String(b.dataset.setl === l));
  for (const el of $$('[data-al-en]')) {
    const v = l === 'zh' ? el.dataset.alZh : el.dataset.alEn;
    el.setAttribute(el instanceof HTMLImageElement ? 'alt' : 'aria-label', v ?? '');
  }
  for (const el of $$<HTMLInputElement>('[data-ph-en]')) el.placeholder = (l === 'zh' ? el.dataset.phZh : el.dataset.phEn) ?? '';
  document.title = (l === 'zh' ? document.body.dataset.titleZh : document.body.dataset.titleEn) ?? document.title;
  paintTheme();
  document.dispatchEvent(new Event('langchange'));
}

function paintTheme(): void {
  const tp = (H.dataset.tp ?? 'auto') as Theme;
  const btn = $('#themeBtn');
  if (btn) {
    const mode = tr(`ui.${tp}`);
    btn.innerHTML = `${ic(tp === 'auto' ? 'auto' : tp === 'dark' ? 'moon' : 'sun')}<span class="vh">${tr('ui.appearance_now', { mode })}</span>`;
  }
  for (const b of $$('[data-sett]')) b.setAttribute('aria-pressed', String(b.dataset.sett === tp));
}

function setTheme(tp: Theme): void {
  H.dataset.tp = tp;
  H.dataset.theme = tp === 'auto' ? (dark.matches ? 'dark' : 'light') : tp;
  store('chT', tp);
  paintTheme();
}

/** Language and appearance switches; the head script has already applied the stored choice before first paint. */
export function initPrefs(): void {
  document.addEventListener('click', (e) => {
    const target = e.target as Element;
    const l = target.closest<HTMLElement>('[data-setl]');
    if (l) setLang(l.dataset.setl === 'zh' ? 'zh' : 'en', true);
    const t = target.closest<HTMLElement>('[data-sett]');
    if (t) setTheme(t.dataset.sett as Theme);
    if (target.closest('#themeBtn')) setTheme({ auto: 'light', light: 'dark', dark: 'auto' }[(H.dataset.tp ?? 'auto') as Theme] as Theme);
  });
  dark.addEventListener('change', () => {
    if (H.dataset.tp === 'auto') setTheme('auto');
  });
  setLang(lang(), false);
}
