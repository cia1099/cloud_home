import type { Status } from './i18n.ts';

/** Original line icons on a 24px grid (design/system.md section 7). No Apple glyphs, no SF Symbols. */
const P = {
  overview: '<rect x="3.5" y="3.5" width="7" height="7" rx="2"/><rect x="13.5" y="3.5" width="7" height="7" rx="2"/><rect x="3.5" y="13.5" width="7" height="7" rx="2"/><rect x="13.5" y="13.5" width="7" height="7" rx="2"/>',
  reports: '<path d="M6 3.5h8l4 4V20a.5.5 0 0 1-.5.5h-11A.5.5 0 0 1 6 20z"/><path d="M14 3.5V8h4M9 12.5h6M9 16h6"/>',
  timeline: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
  requirements: '<path d="M9 6.5h11M9 12h11M9 17.5h11"/><circle cx="4.5" cy="6.5" r=".9"/><circle cx="4.5" cy="12" r=".9"/><circle cx="4.5" cy="17.5" r=".9"/>',
  product: '<rect x="3.5" y="5" width="17" height="14" rx="3"/><circle cx="9" cy="10" r="1.6"/><path d="M4 17l5-4.5 3.5 3L15 13l5 4.5"/>',
  decisions: '<circle cx="12" cy="12" r="8.5"/><path d="M8.2 12.3l2.6 2.6 5-5.4"/>',
  ask: '<circle cx="12" cy="12" r="8.5"/><path d="M9.6 9.6a2.5 2.5 0 1 1 3.6 2.2c-.8.5-1.2 1-1.2 1.9M12 16.6v.1"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  pencil: '<path d="M4 20l1-4L16.5 4.5a2 2 0 0 1 3 3L8 19z"/>',
  xmark: '<path d="M6 6l12 12M18 6L6 18"/>',
  more: '<circle cx="5.5" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="18.5" cy="12" r="1.3"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6L7 7M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4"/>',
  moon: '<path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z"/>',
  auto: '<circle cx="12" cy="12" r="8.5"/><path d="M12 3.5v17"/><path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="currentColor"/>',
  chev: '<path d="M9 5l7 7-7 7"/>',
  chevL: '<path d="M15 5l-7 7 7 7"/>',
  play: '<path d="M8 5.5v13l11-6.5z" fill="currentColor"/>',
  pause: '<path d="M8 5.5v13M16 5.5v13"/>',
  commit: '<circle cx="12" cy="12" r="3.5"/><path d="M3 12h5.5M15.5 12H21"/>',
  warn: '<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17v.1"/>',
  tag: '<path d="M4 4h8l8 8-8 8-8-8z"/><circle cx="8.5" cy="8.5" r="1"/>',
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8v.1"/>',
  search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4 4"/>',
  cloud: '<path d="M7.5 18h9a4 4 0 0 0 .7-7.9A5.5 5.5 0 0 0 6.8 11 3.5 3.5 0 0 0 7.5 18z"/>',
  drive: '<rect x="3.5" y="7" width="17" height="10" rx="3"/><path d="M7 12h5"/><circle cx="16.5" cy="12" r=".9"/>',
  db: '<ellipse cx="12" cy="6.5" rx="7" ry="3"/><path d="M5 6.5v11c0 1.7 3.1 3 7 3s7-1.3 7-3v-11M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3"/>',
  window: '<rect x="3.5" y="5" width="17" height="14" rx="3"/><path d="M3.5 9.5h17"/>',
  server: '<rect x="4" y="4" width="16" height="6.5" rx="2.5"/><rect x="4" y="13.5" width="16" height="6.5" rx="2.5"/><path d="M8 7.2v.1M8 16.8v.1"/>',
  undo: '<path d="M9 7L4.5 11.5 9 16M5 11.5h8a6 6 0 0 1 6 6"/>',
  refresh: '<path d="M19 12a7 7 0 1 1-2.2-5.1M19 4.5v4.2h-4.2"/>',
  phone: '<rect x="7" y="3" width="10" height="18" rx="3"/><path d="M11 18h2"/>',
  shield: '<path d="M12 3.5l7 2.5v5.5c0 4.2-2.9 7.6-7 9-4.1-1.4-7-4.8-7-9V6z"/><path d="M9 12l2.2 2.2L15.2 10"/>',
  down: '<path d="M12 4v11M7 11l5 5 5-5M5 20h14"/>',
  swap: '<path d="M4 8h13l-3-3M20 16H7l3 3"/>',
  power: '<path d="M12 3.5v8M7 6.5a7.5 7.5 0 1 0 10 0"/>',
  heart: '<path d="M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.4 4.3 4.3 0 0 1 19.5 10c0 5.4-7.5 10-7.5 10z"/>',
  lock: '<rect x="5" y="10.5" width="14" height="9.5" rx="2.5"/><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"/>',
} as const;

export type IconName = keyof typeof P;
export const isIcon = (n: string): n is IconName => n in P;

export const ic = (name: IconName, cls = ''): string =>
  `<svg class="ic ${cls}" viewBox="0 0 24 24" aria-hidden="true">${P[name]}</svg>`;

const SG: Record<Status, string> = {
  todo: '<circle cx="6" cy="6" r="4.2"/>',
  doing: '<circle cx="6" cy="6" r="4.2"/><path d="M6 1.8a4.2 4.2 0 0 1 0 8.4z" fill="currentColor"/>',
  review: '<circle cx="5.2" cy="5.2" r="3"/><path d="M7.6 7.6l2.9 2.9"/>',
  done: '<circle cx="6" cy="6" r="5" fill="currentColor" stroke="none"/><path d="M3.8 6.2l1.6 1.6 2.9-3.3" style="stroke:var(--st-bg)"/>',
  blocked: '<circle cx="6" cy="6" r="4.2"/><path d="M4.7 4v4M7.3 4v4"/>',
  dropped: '<circle cx="6" cy="6" r="4.2"/><path d="M3.8 6h4.4"/>',
};

export const statusGlyph = (s: Status): string => `<svg viewBox="0 0 12 12" aria-hidden="true">${SG[s]}</svg>`;

export const LOGO =
  '<svg viewBox="0 0 28 28" aria-hidden="true"><defs><linearGradient id="lg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#42a5ff"/><stop offset="1" stop-color="#0a5fd8"/></linearGradient></defs><rect width="28" height="28" rx="7" fill="url(#lg)"/><path d="M9.4 20h9.2a3.6 3.6 0 0 0 .5-7.2A5.4 5.4 0 0 0 8.7 13.7 3.2 3.2 0 0 0 9.4 20z" fill="#fff"/></svg>';
