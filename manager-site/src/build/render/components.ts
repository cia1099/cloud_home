import { ic, statusGlyph, type IconName } from '../../shared/icons.ts';
import { acceptRequirementUrl, changeRequirementUrl, COMMIT_URL, decisionUrl, removeRequirementUrl } from '../../shared/issue.ts';
import { fill, S, type Bi, type Status } from '../../shared/i18n.ts';
import type { Approval } from '../approvals.ts';
import type { Report } from '../reports.ts';
import { planned, type Area, type Counts, type Requirement } from '../spec.ts';
import type { EventType, Site, SiteEvent } from '../site.ts';
import { al, alBi, bi, D, esc, fmt, plainBi, t, T } from './html.ts';

/** Segments in the order they fill a ring or bar, with the chart colour token each one uses. */
export const SEGS: [Status, string][] = [
  ['done', 'c-done'],
  ['review', 'c-review'],
  ['doing', 'c-doing'],
  ['blocked', 'c-blocked'],
  ['todo', 'c-todo'],
];

export const chip = (s: Status): string => `<span class="chip" data-s="${s}">${statusGlyph(s)}${t(`status.${s}`)}</span>`;
export const toneChip = (tone: string, icon: IconName, text: Bi): string => `<span class="chip ${tone}">${ic(icon)}${bi(text)}</span>`;

function legend(c: Counts): string {
  return `<div class="legend">${SEGS.map(([s, col]) => `<span><i style="background:var(--${col})${s === 'todo' ? ';box-shadow:inset 0 0 0 1px var(--line-strong)' : ''}"></i>${t(`status.${s}`)} <b class="num">${c[s]}</b></span>`).join('')}</div>`;
}

/** Overall progress ring: one circle per status, start at 12 o'clock (design/system.md 9.3). */
export function ring(c: Counts): string {
  const n = planned(c);
  const pct = n ? Math.round((c.done / n) * 100) : 0;
  let off = 0;
  const segs = SEGS.slice(0, 4)
    .map(([s, col], i) => {
      const len = n ? (c[s] / n) * 100 : 0;
      const el = `<circle class="seg-c" cx="50" cy="50" r="45" pathLength="100" stroke="var(--${col})" stroke-width="7" style="--i:${i};--da:${Math.max(len - (len > 0 ? 1.2 : 0), 0)} 100;stroke-dashoffset:${-off};stroke-linecap:butt"/>`;
      off += len;
      return el;
    })
    .join('');
  return `<div class="rv io"><div class="ring-wrap"><svg class="ring" viewBox="0 0 100 100" role="img" ${al('chart.ring', { pct })}><circle class="trk" cx="50" cy="50" r="45" stroke-width="7"/>${segs}</svg><div class="ring-c"><div class="big num"><span data-count="${pct}">${pct}</span><small>%</small></div><div class="cap">${T('of requirements done', '的需求已完成')}</div></div></div>${legend(c)}</div>`;
}

export interface Kpi {
  icon?: IconName;
  label: Bi;
  /** a number counts up; text is shown as is, or per language when it is {en, zh} */
  value: number | string | Bi;
  of?: number;
  sub?: Bi;
}

export function kpi(k: Kpi, i: number): string {
  const v = typeof k.value === 'number' ? `<span data-count="${k.value}">${k.value}</span>` : `<span class="k-text">${typeof k.value === 'string' ? esc(k.value) : plainBi(k.value)}</span>`;
  return `<article class="card kpi rv" style="--rd:${i}"><div class="k-l">${k.icon ? `<span class="k-ic">${ic(k.icon)}</span>` : ''}<span>${plainBi(k.label)}</span></div><div class="k-v">${v}${k.of === undefined ? '' : `<small>${T('of', '共')} ${k.of}</small>`}</div>${k.sub ? `<div class="k-s">${plainBi(k.sub)}</div>` : ''}</article>`;
}

export const kpiGrid = (items: Kpi[], min = 200): string => `<div class="grid g4" style="grid-template-columns:repeat(auto-fit,minmax(min(${min}px,100%),1fr))">${items.map(kpi).join('')}</div>`;

/** Progress by area, computed from SPEC.md statuses. */
export function areaBars(areas: Area[], countFn: (r: Requirement[]) => Counts, built: Record<string, number>): string {
  const rows = areas.map((a) => {
    const c = countFn(a.reqs);
    const n = planned(c);
    const b = built[a.key] ?? 0;
    const pre = b ? `<span class="pre-built">${T(`${b} built before the plan`, `计划前已建成 ${b} 项`)}</span> · ` : '';
    const segs = SEGS.map(([s, col], i) => (c[s] ? `<i style="--w:${(c[s] / n) * 100}%;--c:var(--${col});--i:${i}"></i>` : '')).join('');
    const aria = { en: fill(S['chart.bar'][0], { name: a.name.en, done: c.done, n }), zh: fill(S['chart.bar'][1], { name: a.name.zh, done: c.done, n }) };
    return `<div class="bar-row"><span class="b-n">${plainBi(a.name)}</span><span class="b-v">${pre}${t('chart.done_of', { d: c.done, n })}</span><div class="bar" role="img" ${alBi(aria)}>${segs}</div></div>`;
  });
  return `<div class="bars io">${rows.join('')}</div>`;
}

export function burndown(site: Site): string {
  const pts = site.burndown.points;
  const last = pts.at(-1);
  if (pts.length < 2) {
    const d = last ? fmt(last.date) : null;
    const line = last && d ? T(`One data point so far: ${last.open} requirements open on ${d.en}. The chart appears once there are two days of history.`, `目前只有一个数据点：${d.zh}有 ${last.open} 项需求未完成。有两天的历史后才会画出曲线。`) : T('No burn-down data has been recorded yet.', '还没有记录任何燃尽数据。');
    return `<div class="notice"><b>${T('Not enough history yet', '历史数据还不够')}</b><p class="sub">${line}</p></div>`;
  }
  const data = esc(JSON.stringify({ points: pts, target: site.burndown.target ?? null }));
  const rows = pts.map((p) => `<tr><td>${D(p.date)}</td><td>${p.open}</td></tr>`).join('');
  return `<div class="burn" data-burn="${data}"></div><details class="tbl"><summary>${t('action.view_table')}</summary><table><tr><th>${t('chart.day')}</th><th>${t('chart.open')}</th></tr>${rows}</table></details>`;
}

export function waffle(passed: number, failed: number, skipped: number): string {
  const n = passed + failed + skipped;
  const cells = Array.from({ length: n }, (_, i) => `<i${i < passed ? '' : i < passed + failed ? ' class="fail"' : ' class="skip"'} style="--n:${i}"></i>`).join('');
  return `<div class="waffle${n > 100 ? ' dense' : ''}" role="img" ${al('chart.waffle', { p: passed, f: failed, s: skipped })}>${cells}</div>`;
}

export function checks(items: { name: Bi; state: 'ok' | 'warn' | 'neutral'; detail: Bi }[]): string {
  const icon: Record<string, IconName> = { ok: 'check', warn: 'warn', neutral: 'more' };
  return `<ul class="checks">${items.map((c) => `<li><span class="dot chip ${c.state}">${ic(icon[c.state] ?? 'more')}</span><span>${plainBi(c.name)}</span><span class="r">${plainBi(c.detail)}</span></li>`).join('')}</ul>`;
}

export function callout(tone: 'risk' | 'block' | 'info', title: Bi, text: Bi, i = 0): string {
  const icon: IconName = tone === 'block' ? 'lock' : tone === 'risk' ? 'warn' : 'info';
  return `<div class="callout ${tone} rv" style="--rd:${i}">${ic(icon)}<div><h4>${bi(title)}</h4><p>${bi(text)}</p></div></div>`;
}

export function emptyFrame(title: Bi, text: Bi, extra = ''): string {
  return `<div class="emptyframe"><div class="eframe" aria-hidden="true"><i></i><i></i><i></i></div><div><h2 class="title">${plainBi(title)}</h2><p class="lede" style="margin:10px auto 0;text-align:center">${plainBi(text)}</p></div>${extra}</div>`;
}

// ---------- report cards ----------

export function reportCard(r: Report, base: string, feature = false): string {
  const cover = `<div class="cover ${r.cover.style}"><span class="glyph">${ic(r.cover.icon)}</span></div>`;
  const meta = `<div class="row">${r.chip ? toneChip(r.chip.tone, r.chip.icon, r.chip.text) : ''}<span class="chip neutral">${t(`kind.${r.kind}`)}</span><span class="small">${D(r.date)}</span></div>`;
  const href = `${base}${r.id}/`;
  if (feature) {
    return `<a class="card feature rcard rv" href="${href}" style="color:inherit"><div class="f-body">${meta}<h3 class="display" style="font-size:clamp(1.75rem,3.6vw,2.5rem)">${plainBi(r.title)}</h3><p class="sub" style="margin:0">${plainBi(r.summary)}</p><span class="more">${t('action.read_report')}</span></div>${cover}</a>`;
  }
  return `<a class="card rcard rv" href="${href}">${cover}<div class="body">${meta}<h3>${plainBi(r.title)}</h3><p>${plainBi(r.summary)}</p><span class="more">${t('action.read')}</span></div></a>`;
}

// ---------- diagrams ----------

export interface ArchModule {
  icon: IconName;
  title: Bi;
  text: Bi;
  status: 'built' | 'planned';
}

export interface ArchData {
  client: { icon: IconName; title: Bi; text: Bi };
  binary: { name: string; subtitle: Bi; modules: ArchModule[] };
  drive: { title: Bi; text: Bi; modules: ArchModule[] };
  cloud: { icon: IconName; title: Bi; text: Bi };
  links: [Bi, Bi, Bi];
}

const mod = (m: ArchModule): string =>
  `<div class="mod ${m.status}"><span class="mod-i">${ic(m.icon)}</span><div><b>${plainBi(m.title)}</b><span>${plainBi(m.text)}</span></div><span class="mod-b">${t(m.status === 'built' ? 'diagram.built' : 'diagram.not_built')}</span></div>`;

export function archDiagram(a: ArchData): string {
  const link = (cls: string, b: Bi): string => `<div class="a-link ${cls}" aria-hidden="true"><span>${plainBi(b)}</span><i></i></div>`;
  return `<div class="arch" role="group" ${al('diagram.arch')}>
    <div class="a-node a-client"><span class="mod-i big">${ic(a.client.icon)}</span><b>${plainBi(a.client.title)}</b><span>${plainBi(a.client.text)}</span></div>
    ${link('l1', a.links[0])}
    <div class="a-node a-bin"><div class="bin-h"><code>${esc(a.binary.name)}</code><span>${plainBi(a.binary.subtitle)}</span></div>${a.binary.modules.map(mod).join('')}</div>
    ${link('l2', a.links[1])}
    <div class="a-node a-drive"><div class="bin-h"><b>${plainBi(a.drive.title)}</b><span>${plainBi(a.drive.text)}</span></div>${a.drive.modules.map(mod).join('')}</div>
    ${link('l3', a.links[2])}
    <div class="a-node a-cloud"><span class="mod-i big">${ic(a.cloud.icon)}</span><b>${plainBi(a.cloud.title)}</b><span>${plainBi(a.cloud.text)}</span></div></div>`;
}

export interface OtaStep {
  icon: IconName;
  title: Bi;
  text: Bi;
  fail?: Bi;
}

export function otaDiagram(steps: OtaStep[], rollback: Bi): string {
  const li = steps.map((s, i) => `<li style="--i:${i}"><span class="node">${ic(s.icon)}</span><div><b>${plainBi(s.title)}</b><span>${plainBi(s.text)}</span>${s.fail ? `<span class="failtag">${ic('undo')}${plainBi(s.fail)}</span>` : ''}</div></li>`).join('');
  return `<div class="io"><ol class="flow" style="--n:${steps.length}">${li}</ol></div><div class="rollback">${ic('warn')}<span>${bi(rollback)}</span></div>`;
}

// ---------- timeline ----------

const EVENT_ICON: Record<EventType, IconName> = { commit: 'commit', release: 'tag', report: 'reports', decision: 'decisions', spec: 'requirements' };

export function eventItem(e: SiteEvent, root: string, month: string): { html: string; month: string } {
  const m = e.at.slice(0, 7);
  const head = m !== month ? `<li class="tl-month" aria-hidden="true">${D(e.at, { month: 'long', year: 'numeric' })}</li>` : '';
  const hash = e.hash ? `<a class="hash" href="${COMMIT_URL}${e.hash}" target="_blank" rel="noopener" ${al('ui.open_commit')}>${ic('commit')}${e.hash}</a>` : '';
  const tag = e.tag ? `<span class="chip neutral">${ic('tag')}${esc(e.tag)}</span>` : '';
  let ref = '';
  if (e.ref) {
    if (e.type === 'decision') ref = `<a class="hash" href="${root}decisions/">${esc(e.ref)}</a>`;
    else if (e.type === 'report') ref = `<a class="hash" href="${root}reports/${e.ref}/">${ic('reports')}${t('event.open_report')}</a>`;
  }
  const html = `${head}<li class="ev" data-t="${e.type}"><span class="pin">${ic(EVENT_ICON[e.type])}</span><div class="ev-card"><div class="ev-t">${e.type === 'report' || e.type === 'decision' ? plainBi(e.title) : `<span lang="en">${esc(e.title.en)}</span>`}</div><div class="ev-m"><span>${t(`event.${e.type}`)}</span><span aria-hidden="true">·</span><time datetime="${e.at}">${D(e.at, { month: 'short', day: 'numeric' })}</time>${hash}${tag}${ref}</div></div></li>`;
  return { html, month: m };
}

export function timelineList(events: SiteEvent[], root: string, id?: string): string {
  let month = '';
  const items = events.map((e) => {
    const r = eventItem(e, root, month);
    month = r.month;
    return r.html;
  });
  return `<ol class="tl"${id ? ` id="${id}"` : ''}>${items.join('')}</ol>`;
}

// ---------- requirements ----------

const ownerLabel = (o: Requirement['owner']): string => (o ? t(`role.${o}`) : '');

export function reqRow(r: Requirement): string {
  const q = `${r.id} ${r.title.en} ${r.title.zh}`.toLowerCase();
  const ch = esc(changeRequirementUrl(r.id, r.title.en));
  const de = esc(removeRequirementUrl(r.id, r.title.en));
  const ok = r.status === 'review' ? `<a class="btn btn-ghost acc" href="${esc(acceptRequirementUrl(r.id, r.title.en))}" target="_blank" rel="noopener">${ic('check')}${t('action.accept')}</a>` : '';
  return `<li class="req" data-s="${r.status}" data-q="${esc(q)}"><span class="id">${r.id}</span><span class="t">${plainBi(r.title)}${r.blocked ? `<span class="why">${plainBi(r.blocked)}</span>` : ''}</span><span class="pri" data-p="${r.pri}" title="${esc(S[`priority.${r.pri}` as const][0])}">${r.pri}</span><span class="ch">${chip(r.status)}</span><span class="own">${ownerLabel(r.owner)}</span><span class="acts">${ok}<a class="btn btn-ghost" href="${ch}" target="_blank" rel="noopener">${ic('pencil')}${t('action.change')}</a><a class="btn btn-ghost del" href="${de}" target="_blank" rel="noopener">${ic('xmark')}${t('action.remove')}</a></span></li>`;
}

// ---------- decisions ----------

/** Decisions that have a page of their own (relative to decisions/): the explanation the owner reads before answering. */
export const DETAIL_PAGE: Record<string, string> = { 'A-004': 'brand/', 'A-002': 'admins/', 'A-005': 'guides/' };

/** `root` is the relative prefix from the rendered page to the site root ("../", "../../"). `detail: false` hides the link on the detail page itself. */
export function decisionCard(a: Approval, root: string, detail = true): string {
  const f = a.fields;
  const rows: [string, Bi | undefined][] = [
    [T('What', '内容'), f.what],
    [T('Why', '原因'), f.why],
    [T('Risk', '风险'), f.risk],
    [T('If approved', '批准后'), f.ifApproved],
    [T('If denied', '拒绝后'), f.ifDenied],
  ];
  const dl = rows.filter((r): r is [string, Bi] => r[1] !== undefined).map(([k, v]) => `<dt>${k}</dt><dd>${plainBi(v)}</dd>`).join('');
  const page = detail ? DETAIL_PAGE[a.id] : undefined;
  const more = page ? `<p style="margin:18px 0 0"><a href="${root}decisions/${page}">${T('Read the full explanation ›', '阅读完整说明 ›')}</a></p>` : '';
  return `<article class="card dcard rv" id="${a.id}"><div class="dh"><span class="hash" style="height:24px">${a.id}</span>${a.requested ? `<span class="small">${T('Requested by the manager,', '经理提出于')} ${D(a.requested)}</span>` : ''}</div><h3>${plainBi(a.title)}</h3>${dl ? `<dl class="dl">${dl}</dl>` : ''}${more}<div class="dact">${answerButtons(a)}<p class="note">${T('Opens a pre-filled issue on GitHub. Nothing is sent until you press Submit there.', '在 GitHub 上打开预填好的议题。你按下提交之前不会发送任何内容。')}</p></div></article>`;
}

function answerButtons(a: Approval): string {
  const link = (cls: string, answer: 'approve' | 'deny', icon: IconName, label: string): string =>
    `<a class="btn ${cls} btn-lg" href="${esc(decisionUrl(a.id, a.title.en, answer))}" target="_blank" rel="noopener">${ic(icon)}${label}</a>`;
  return link('btn-pri', 'approve', 'check', t('action.approve')) + link('btn-neutral', 'deny', 'xmark', t('action.deny'));
}

export function historyItem(a: Approval): string {
  const ok = a.state === 'approved';
  const note = a.fields.note ? ` · ${plainBi(a.fields.note)}` : '';
  return `<li><span class="hash">${a.id}</span><div><b>${plainBi(a.title)}</b><div class="small">${a.requested ? D(a.requested) : ''}${note}</div></div><span class="chip ${ok ? 'ok' : ''}" ${ok ? '' : 'data-s="blocked"'}>${ic(ok ? 'check' : 'xmark')}${ok ? T('Approved', '已批准') : T('Denied', '已拒绝')}</span></li>`;
}

// ---------- media: gallery, compare, walkthrough ----------

export interface Shot {
  src: string;
  w: number;
  h: number;
  alt: Bi;
  title: Bi;
  meta?: string;
  wide?: boolean;
}

export const img = (src: string, w: number, h: number, alt: Bi): string =>
  `<img src="${src}" width="${w}" height="${h}" loading="lazy" decoding="async" alt="${esc(alt.en)}" data-al-en="${esc(alt.en)}" data-al-zh="${esc(alt.zh)}" data-alt="1">`;

export function gallery(shots: Shot[]): string {
  return `<div class="gal" data-gallery>${shots
    .map((s) => `<button class="shot${s.wide ? ' wide' : ''}" data-lb type="button"><div class="art">${img(s.src, s.w, s.h, s.alt)}</div><div class="cap"><span>${plainBi(s.title)}</span>${s.meta ? `<small>${esc(s.meta)}</small>` : ''}</div></button>`)
    .join('')}</div>`;
}

export function compare(c: { before: Shot; after: Shot; beforeLabel: Bi; afterLabel: Bi; label: Bi }): string {
  return `<div class="ba io"><div class="layer before"><div class="art">${img(c.before.src, c.before.w, c.before.h, c.before.alt)}</div></div><div class="layer after"><div class="art">${img(c.after.src, c.after.w, c.after.h, c.after.alt)}</div></div><input type="range" min="0" max="100" step="2" value="50" ${alBi(c.label)}><div class="handle" aria-hidden="true"></div><span class="lab l" aria-hidden="true">${plainBi(c.beforeLabel)}</span><span class="lab r" aria-hidden="true">${plainBi(c.afterLabel)}</span></div><p class="cap">${t('ui.compare_hint')}</p>`;
}

export interface WalkStep {
  title: Bi;
  caption: Bi;
  media: { kind: 'image'; src: string; w: number; h: number; alt: Bi } | { kind: 'video'; src: string; mime: string; poster?: string };
}

export function player(steps: WalkStep[], durationMs: number): string {
  const scenes = steps
    .map((s, i) => {
      const m = s.media;
      const inner = m.kind === 'image' ? img(m.src, m.w, m.h, m.alt) : `<video muted playsinline loop preload="metadata"${m.poster ? ` poster="${m.poster}"` : ''}><source src="${m.src}" type="${m.mime}"></video>`;
      return `<div class="p-scene${i === 0 ? ' on' : ''}">${inner}</div>`;
    })
    .join('');
  const list = steps
    .map((s, i) => `<li><button type="button"${i === 0 ? ' aria-current="true"' : ''}><span class="n">${i + 1}</span><span>${plainBi(s.title)}</span><span class="s-cap">${plainBi(s.caption)}</span></button></li>`)
    .join('');
  return `<div class="player" role="group" ${al('ui.player')} data-duration="${durationMs}"><div class="p-stage">${scenes}</div><div class="p-bar"><button class="p-play" type="button"></button><div class="p-track" aria-hidden="true"><i></i></div><span class="p-time num" aria-hidden="true">1 / ${steps.length}</span></div><p class="p-cap" aria-live="polite"></p><ol class="p-steps">${list}</ol></div>`;
}
