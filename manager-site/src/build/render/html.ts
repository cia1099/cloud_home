import { fill, S, type Bi, type Key } from '../../shared/i18n.ts';

export const esc = (s: string): string => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** Two sibling spans; CSS shows the one matching html[data-l]. `en`/`zh` are already HTML. */
export const T = (en: string, zh: string): string => `<span lang="en">${en}</span><span lang="zh-Hans">${zh}</span>`;

/** Plain text from data files: HTML-escaped, with `code` and **bold** as the only markup. */
export function rich(s: string): string {
  return esc(s).replace(/`([^`]+)`/g, '<span class="mono">$1</span>').replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>');
}

export const bi = (b: Bi): string => T(rich(b.en), rich(b.zh));
export const plainBi = (b: Bi): string => T(esc(b.en), esc(b.zh));

export const t = (key: Key, vars?: Record<string, string | number>): string => T(esc(fill(S[key][0], vars)), esc(fill(S[key][1], vars)));

/** aria-label that follows the language switch (the browser code swaps data-al-en / data-al-zh). */
export const al = (key: Key, vars?: Record<string, string | number>): string => {
  const [en, zh] = [fill(S[key][0], vars), fill(S[key][1], vars)];
  return `aria-label="${esc(en)}" data-al-en="${esc(en)}" data-al-zh="${esc(zh)}"`;
};

export const alBi = (b: Bi): string => `aria-label="${esc(b.en)}" data-al-en="${esc(b.en)}" data-al-zh="${esc(b.zh)}"`;

const dateFmt = (loc: string, o: Intl.DateTimeFormatOptions): Intl.DateTimeFormat => new Intl.DateTimeFormat(loc, { timeZone: 'UTC', ...o });

const DEFAULT_DATE: Intl.DateTimeFormatOptions = { day: 'numeric', month: 'short', year: 'numeric' };
const DATE_TIME: Intl.DateTimeFormatOptions = { ...DEFAULT_DATE, hour: '2-digit', minute: '2-digit', hourCycle: 'h23' };

/** Plain-text date per language, formatted by Intl, always in UTC. */
export function fmt(iso: string, o: Intl.DateTimeFormatOptions = DEFAULT_DATE): Bi {
  const d = new Date(iso.length === 10 ? `${iso}T12:00:00Z` : iso);
  return { en: dateFmt('en', o).format(d), zh: dateFmt('zh-Hans', o).format(d) };
}

export function D(iso: string, o?: Intl.DateTimeFormatOptions): string {
  const f = fmt(iso, o);
  return T(f.en, f.zh);
}

export const DT = (iso: string): string => D(iso, DATE_TIME);
export const fmtDateTime = (iso: string): Bi => fmt(iso, DATE_TIME);

export const slug = (s: string): string => s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
