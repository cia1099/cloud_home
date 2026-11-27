import { ic, LOGO, type IconName } from '../../shared/icons.ts';
import { askUrl } from '../../shared/issue.ts';
import type { Site } from '../site.ts';
import { al, esc, fmtDateTime, t, T } from './html.ts';

export type NavKey = 'overview' | 'reports' | 'timeline' | 'requirements' | 'product' | 'decisions';

export interface Page {
  /** URL path with leading and trailing slash, e.g. /reports/ */
  path: string;
  nav: NavKey;
  title: { en: string; zh: string };
  /** extra stylesheet (hashed) of a bespoke report page */
  css?: string;
}

export interface Bundle {
  js: string;
  css: string[];
}

/** Relative prefix from a page to the site root: "/" -> "", "/reports/" -> "../", "/reports/x/" -> "../../". */
export const rootOf = (path: string): string => '../'.repeat(path.split('/').filter(Boolean).length);

const NAV: [NavKey, string][] = [
  ['overview', ''],
  ['reports', 'reports/'],
  ['timeline', 'timeline/'],
  ['requirements', 'requirements/'],
  ['product', 'product/'],
  ['decisions', 'decisions/'],
];
const TABS: NavKey[] = ['overview', 'reports', 'requirements', 'decisions'];

/** Runs in <head> before first paint: language, appearance, no flash. */
const BOOT = `(function(){var d=document.documentElement,s={};try{s=localStorage}catch(e){}var l=s.chL||(/^zh/i.test(navigator.language)?'zh':'en');if(l!=='zh')l='en';d.dataset.l=l;d.lang=l==='zh'?'zh-Hans':'en';var t=s.chT||'auto';d.dataset.tp=t;d.dataset.theme=t==='auto'?(matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light'):t;d.classList.add('js')})()`;

const FAVICON = `data:image/svg+xml,${encodeURIComponent(LOGO.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" '))}`;

export function shell(site: Site, bundle: Bundle, p: Page, main: string): string {
  const root = rootOf(p.path);
  const href = (rel: string): string => `${root}${rel}`;
  const label = (k: NavKey): string => t(`nav.${k}`);
  const pend = site.pending.length;
  const badge = (aria: boolean): string => (pend ? `<span class="badge"${aria ? ` ${al('decision.badge', { n: pend })}` : ''}>${pend}</span>` : '');
  const cur = (k: NavKey): string => (p.nav === k ? ' aria-current="page"' : '');
  const ask = esc(askUrl(p.path));
  const tabIcon = (k: NavKey): IconName => k;
  const seg = `<div class="seg" role="group" ${al('ui.language_both')}><button data-setl="en" lang="en" ${al('ui.english')} aria-pressed="true">EN</button><button data-setl="zh" lang="zh-Hans" ${al('ui.chinese')} aria-pressed="false">中文</button></div>`;
  const css = [...bundle.css, ...(p.css ? [p.css] : [])];
  const built = fmtDateTime(site.builtAt.toISOString());
  const at = site.head ? { en: ` at commit <span class="mono">${site.head}</span>`, zh: `在提交 <span class="mono">${site.head}</span> ` } : { en: '', zh: '' };
  const src = '<span class="mono">reports/</span>, <span class="mono">SPEC.md</span>';
  return `<!doctype html>
<html lang="en" data-l="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="color-scheme" content="light dark">
<title>${esc(p.title.en)} · Cloud Home Report</title>
<link rel="icon" href="${FAVICON}">
${css.map((c) => `<link rel="stylesheet" href="${href(c)}">`).join('\n')}
<script>${BOOT}</script>
<script type="module" src="${href(bundle.js)}"></script>
</head>
<body data-title-en="${esc(p.title.en)} · Cloud Home Report" data-title-zh="${esc(p.title.zh)} · Cloud Home 报告">
<a class="skip" href="#main">${t('action.skip')}</a>
<header class="hdr"><div class="hdr-in">
  <a class="brand" href="${href('')}">${LOGO}<span>Cloud Home<span class="brand-s">${t('site.report')}</span></span></a>
  <nav class="nav" ${al('nav.main')}>${NAV.map(([k, r]) => `<a href="${href(r)}"${cur(k)}>${label(k)}${k === 'decisions' ? badge(true) : ''}</a>`).join('')}</nav>
  <div class="hdr-r">
    <a class="btn btn-sec ask" href="${ask}" target="_blank" rel="noopener" ${al('action.ask')}>${ic('ask')}<span class="ask-t">${t('action.ask')}</span></a>
    ${seg}
    <button class="icon-btn" id="themeBtn" type="button"></button>
  </div></div></header>
${main}
<footer class="ftr"><div class="wrap">
  <div class="links">${NAV.map(([k, r]) => `<a href="${href(r)}">${label(k)}</a>`).join('')}${site.latest ? `<a href="${href('latest/')}">${T('Latest report', '最新报告')}</a>` : ''}</div>
  <div>${T(`Generated from ${src} and <span class="mono">plan/</span>${at.en} · built ${built.en} UTC`, `由 ${src} 与 <span class="mono">plan/</span> ${at.zh}生成 · 构建于 ${built.zh}（UTC）`)}</div>
  <div>${T('This site is public and read-only. Request buttons open a pre-filled GitHub issue; nothing is sent until you press Submit there.', '本站公开且只读。请求按钮会打开预填好的 GitHub 议题，只有在那里点击提交后才会发送。')}</div>
</div></footer>
<nav class="tabbar" ${al('nav.main')}>${TABS.map((k) => `<a href="${href(NAV.find((n) => n[0] === k)?.[1] ?? '')}"${cur(k)}>${ic(tabIcon(k))}<span>${label(k)}</span>${k === 'decisions' ? badge(false) : ''}</a>`).join('')}<button id="moreBtn" type="button" aria-haspopup="dialog">${ic('more')}<span>${t('nav.more')}</span></button></nav>
<div class="sheet" id="sheet" role="dialog" aria-modal="true" ${al('nav.more')}><div class="sheet-p"><div class="grab"></div>
  <a class="item" href="${href('timeline/')}">${ic('timeline')}${label('timeline')}</a>
  <a class="item" href="${href('product/')}">${ic('product')}${label('product')}</a>
  <a class="item" href="${ask}" target="_blank" rel="noopener">${ic('ask')}${t('action.ask')}</a>
  <div class="setrow"><span>${t('ui.language')}</span>${seg}</div>
  <div class="setrow"><span>${t('ui.appearance')}</span><div class="seg"><button data-sett="auto" type="button">${t('ui.auto')}</button><button data-sett="light" type="button">${t('ui.light')}</button><button data-sett="dark" type="button">${t('ui.dark')}</button></div></div>
</div></div>
</body></html>`;
}
