import { str, type Key, type Lang } from '../shared/i18n.ts';

export const $ = <T extends HTMLElement = HTMLElement>(s: string, r: ParentNode = document): T | null => r.querySelector<T>(s);
export const $$ = <T extends HTMLElement = HTMLElement>(s: string, r: ParentNode = document): T[] => [...r.querySelectorAll<T>(s)];

export const reducedMotion = (): boolean => matchMedia('(prefers-reduced-motion: reduce)').matches;

export const lang = (): Lang => (document.documentElement.dataset.l === 'zh' ? 'zh' : 'en');
export const tr = (key: Key, vars?: Record<string, string | number>): string => str(key, lang(), vars);

/** Run `fn` now and again whenever the language changes. */
export const onLang = (fn: () => void): void => {
  fn();
  document.addEventListener('langchange', fn);
};

/** Intl formatter for the active language (UTC, because every date on the site is UTC). */
export const dateFmt = (o: Intl.DateTimeFormatOptions): Intl.DateTimeFormat => new Intl.DateTimeFormat(lang() === 'zh' ? 'zh-Hans' : 'en', { timeZone: 'UTC', ...o });
