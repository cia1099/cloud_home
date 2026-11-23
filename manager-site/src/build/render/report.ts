import { extname } from 'node:path';
import { ic, isIcon, type IconName } from '../../shared/icons.ts';
import type { Bi } from '../../shared/i18n.ts';
import type { Assets } from '../assets.ts';
import type { Report, Section } from '../reports.ts';
import { testTotals, type Site } from '../site.ts';
import { count } from '../spec.ts';
import { asArray, asBi, asNumber, asObject, asString, oneOf, optBi } from '../util.ts';
import {
  archDiagram, areaBars, burndown, callout, chip, compare, decisionCard, gallery, kpiGrid, otaDiagram, player, timelineList, toneChip, waffle,
  type ArchData, type ArchModule, type Kpi, type OtaStep, type Shot, type WalkStep,
} from './components.ts';
import { al, bi, D, esc, plainBi, rich, t, T } from './html.ts';

export interface Ctx {
  site: Site;
  assets: Assets;
  report: Report;
  root: string;
}

const icon = (v: unknown, where: string): IconName => {
  const s = asString(v, where);
  if (!isIcon(s)) throw new Error(`${where}: unknown icon "${s}"`);
  return s;
};

function shotOf(c: Ctx, o: Record<string, unknown>, where: string): Shot {
  const rel = asString(o.src, `${where}.src`);
  const { w, h } = c.assets.size(c.report.id, rel);
  const meta = [o.build, o.theme, o.language].filter((v): v is string => typeof v === 'string').join(' · ');
  return {
    src: `${c.root}${c.assets.media(c.report.id, rel)}`,
    w,
    h,
    alt: asBi(o.alt, `${where}.alt`),
    title: asBi(o.title ?? o.alt, `${where}.title`),
    meta: meta || undefined,
    wide: o.wide === true,
  };
}

export const shotsOf = (c: Ctx, s: Section): Shot[] =>
  asArray(s.data.shots, `${c.report.id} ${s.id}: shots`).map((x, i) => shotOf(c, asObject(x, `${c.report.id} ${s.id}.shots[${i}]`), `${c.report.id} ${s.id}.shots[${i}]`));

export function stepsOf(c: Ctx, s: Section): WalkStep[] {
  return asArray(s.data.steps, `${c.report.id} ${s.id}: steps`).map((x, i) => {
    const w = `${c.report.id} ${s.id}.steps[${i}]`;
    const o = asObject(x, w);
    const m = asObject(o.media, `${w}.media`);
    const rel = asString(m.src, `${w}.media.src`);
    const url = `${c.root}${c.assets.media(c.report.id, rel)}`;
    const ext = extname(rel).toLowerCase();
    const title = asBi(o.title, `${w}.title`);
    if (ext === '.mp4' || ext === '.webm') {
      const poster = m.poster === undefined ? undefined : `${c.root}${c.assets.media(c.report.id, asString(m.poster, `${w}.media.poster`))}`;
      return { title, caption: asBi(o.caption, `${w}.caption`), media: { kind: 'video' as const, src: url, mime: ext === '.mp4' ? 'video/mp4' : 'video/webm', poster } };
    }
    const { w: iw, h: ih } = c.assets.size(c.report.id, rel);
    return { title, caption: asBi(o.caption, `${w}.caption`), media: { kind: 'image' as const, src: url, w: iw, h: ih, alt: optBi(m.alt, `${w}.media.alt`) ?? title } };
  });
}

export function renderCompare(c: Ctx, s: Section): string {
  const w = `${c.report.id} ${s.id}`;
  const d = s.data;
  const mk = (k: 'before' | 'after'): Shot => shotOf(c, { src: d[k], alt: d.alt }, `${w}.${k}`);
  return compare({ before: mk('before'), after: mk('after'), beforeLabel: asBi(d.before_label, `${w}.before_label`), afterLabel: asBi(d.after_label, `${w}.after_label`), label: asBi(d.alt, `${w}.alt`) });
}

function archOf(d: Record<string, unknown>, c: Ctx, w: string): ArchData {
  const done = new Map(c.site.spec.reqs.map((r) => [r.id, r.status === 'done']));
  const mods = (v: unknown, ww: string): ArchModule[] =>
    asArray(v, ww).map((m, i) => {
      const o = asObject(m, `${ww}[${i}]`);
      const requires = asArray(o.requires ?? [], `${ww}[${i}].requires`).map((r) => asString(r, `${ww}[${i}].requires`));
      for (const r of requires) if (!done.has(r)) throw new Error(`${ww}[${i}]: requirement ${r} is not in SPEC.md`);
      return { icon: icon(o.icon, `${ww}[${i}].icon`), title: asBi(o.title, `${ww}[${i}].title`), text: asBi(o.text, `${ww}[${i}].text`), status: requires.every((r) => done.get(r)) ? 'built' : 'planned' };
    });
  const client = asObject(d.client, `${w}.client`);
  const binary = asObject(d.binary, `${w}.binary`);
  const drive = asObject(d.drive, `${w}.drive`);
  const cloud = asObject(d.cloud, `${w}.cloud`);
  const links = asArray(d.links, `${w}.links`).map((l, i) => asBi(l, `${w}.links[${i}]`));
  const [l1, l2, l3] = links;
  if (!l1 || !l2 || !l3) throw new Error(`${w}.links: three link labels are required`);
  return {
    client: { icon: icon(client.icon, `${w}.client.icon`), title: asBi(client.title, `${w}.client.title`), text: asBi(client.text, `${w}.client.text`) },
    binary: { name: asString(binary.name, `${w}.binary.name`), subtitle: asBi(binary.subtitle, `${w}.binary.subtitle`), modules: mods(binary.modules, `${w}.binary.modules`) },
    drive: { title: asBi(drive.title, `${w}.drive.title`), text: asBi(drive.text, `${w}.drive.text`), modules: mods(drive.modules, `${w}.drive.modules`) },
    cloud: { icon: icon(cloud.icon, `${w}.cloud.icon`), title: asBi(cloud.title, `${w}.cloud.title`), text: asBi(cloud.text, `${w}.cloud.text`) },
    links: [l1, l2, l3],
  };
}

function testsBlock(c: Ctx, note: Bi | undefined): string {
  const tests = c.report.tests;
  if (!tests) throw new Error(`reports/${c.report.id}: a "tests" section needs reports/${c.report.id}/tests.json`);
  const { passed, failed, skipped } = testTotals(tests);
  const suites = tests.suites
    .map((x) => `<li><span class="dot chip ${x.failed ? 'warn' : 'ok'}">${ic(x.failed ? 'warn' : 'check')}</span><span>${esc(x.name)}</span><span class="r">${T(`${x.passed} passed · ${x.failed} failed · ${x.skipped} skipped`, `${x.passed} 通过 · ${x.failed} 失败 · ${x.skipped} 跳过`)}</span></li>`)
    .join('');
  return `<article class="card rv io tests-card"><div>${waffle(passed, failed, skipped)}</div><div><p class="num big-n">${passed}</p><p class="sub" style="margin:6px 0 0">${T('tests passed', '项测试通过')}</p><p class="small" style="margin:10px 0 0">${T(`${failed} failed · ${skipped} skipped`, `${failed} 失败 · ${skipped} 跳过`)}</p></div></article><ul class="checks card" style="margin-top:16px;padding-block:8px">${suites}</ul>${note ? `<p class="small" style="margin-top:12px">${bi(note)}</p>` : ''}`;
}

/** One report section: the component named in report.json, composed from its data. */
export function renderSection(c: Ctx, s: Section): string {
  const w = `reports/${c.report.id} section "${s.id}"`;
  const d = s.data;
  switch (s.component) {
    case 'kpis':
      return kpiGrid(
        asArray(d.items, `${w}: items`).map((x, i): Kpi => {
          const o = asObject(x, `${w}.items[${i}]`);
          return { label: asBi(o.label, `${w}.items[${i}].label`), value: typeof o.value === 'number' || typeof o.value === 'string' ? o.value : asBi(o.value, `${w}.items[${i}].value`), of: o.of === undefined ? undefined : asNumber(o.of, `${w}.items[${i}].of`), sub: optBi(o.sub, `${w}.items[${i}].sub`) };
        }),
        150,
      );
    case 'text': {
      const paras = asArray(d.paragraphs ?? [], `${w}: paragraphs`).map((p, i) => `<p>${bi(asBi(p, `${w}.paragraphs[${i}]`))}</p>`);
      const bullets = asArray(d.bullets ?? [], `${w}: bullets`).map((p, i) => `<li>${bi(asBi(p, `${w}.bullets[${i}]`))}</li>`);
      return paras.join('') + (bullets.length ? `<ul>${bullets.join('')}</ul>` : '');
    }
    case 'diagram': {
      const kind = oneOf(d.kind, ['arch', 'ota'] as const, `${w}: kind`);
      const body =
        kind === 'arch'
          ? archDiagram(archOf(d, c, w))
          : otaDiagram(
              asArray(d.steps, `${w}: steps`).map((x, i): OtaStep => {
                const o = asObject(x, `${w}.steps[${i}]`);
                return { icon: icon(o.icon, `${w}.steps[${i}].icon`), title: asBi(o.title, `${w}.steps[${i}].title`), text: asBi(o.text, `${w}.steps[${i}].text`), fail: optBi(o.fail, `${w}.steps[${i}].fail`) };
              }),
              asBi(d.rollback, `${w}: rollback`),
            );
      return `<figure class="diagram rv" style="margin:20px 0 0">${body}${s.caption ? `<figcaption class="cap">${bi(s.caption)}</figcaption>` : ''}</figure>`;
    }
    case 'areas':
      return `<article class="card rv" style="margin-top:16px">${areaBars(c.site.spec.areas, count, c.site.status.builtBeforePlan)}<p class="small" style="margin:16px 0 0">${T('Computed from SPEC.md each time the site is built, so it always shows the current status, not the status on the day of this report.', '每次构建网站时都会根据 SPEC.md 重新计算，因此显示的是当前状态，而不是本报告发布当天的状态。')}</p></article>`;
    case 'burndown':
      return `<article class="card rv io" style="margin-top:16px">${burndown(c.site)}</article>`;
    case 'tests':
      return testsBlock(c, optBi(d.note, `${w}: note`));
    case 'timeline': {
      const limit = d.limit === undefined ? 8 : asNumber(d.limit, `${w}: limit`);
      const until = `${c.report.date}T${c.report.time}:59.999Z`;
      const events = c.site.events.filter((e) => (e.type === 'commit' || e.type === 'release') && e.at <= until).slice(0, limit);
      return `${timelineList(events, c.root)}<p class="small"><a href="${c.root}timeline/">${t('action.full_timeline')}</a></p>`;
    }
    case 'callouts':
      return `<div class="stack">${asArray(d.items, `${w}: items`)
        .map((x, i) => {
          const o = asObject(x, `${w}.items[${i}]`);
          return callout(oneOf(o.tone, ['risk', 'block', 'info'] as const, `${w}.items[${i}].tone`), asBi(o.title, `${w}.items[${i}].title`), asBi(o.text, `${w}.items[${i}].text`), i);
        })
        .join('')}</div>`;
    case 'gallery':
      return gallery(shotsOf(c, s));
    case 'compare':
      return renderCompare(c, s);
    case 'walkthrough':
      return player(stepsOf(c, s), d.duration_ms === undefined ? 3400 : asNumber(d.duration_ms, `${w}: duration_ms`));
    case 'decisions':
      return asArray(d.ids, `${w}: ids`)
        .map((id) => {
          const a = c.site.approvals.find((x) => x.id === id);
          if (!a) throw new Error(`${w}: decision ${String(id)} is not in plan/APPROVALS.md`);
          return a.state === 'pending' ? decisionCard(a) : `<p>${chip(a.state === 'approved' ? 'done' : 'blocked')} ${plainBi(a.title)}</p>`;
        })
        .join('');
    default:
      throw new Error(`${w}: unknown component "${s.component}"`);
  }
}

const hero = (r: Report): string =>
  `<header class="wrap rhero"><nav class="small" ${al('nav.breadcrumb')}><a href="../">${t('nav.reports')}</a> › ${D(r.date)}</nav><div class="row" style="margin-top:18px">${r.chip ? toneChip(r.chip.tone, r.chip.icon, r.chip.text) : ''}<span class="chip neutral">${t(`kind.${r.kind}`)}</span></div><h1 class="display">${plainBi(r.title)}</h1><p class="lede">${plainBi(r.summary)}</p></header>`;

function pager(site: Site, r: Report): string {
  const i = site.reports.findIndex((x) => x.id === r.id);
  const older = site.reports[i + 1];
  const newer = site.reports[i - 1];
  const link = (x: Report | undefined, cls: string, label: Bi): string =>
    x ? `<a class="${cls}" href="../${x.id}/"><small>${plainBi(label)}</small><b>${plainBi(x.title)}</b></a>` : `<span></span>`;
  return `<nav class="pager" ${al('nav.other_reports')}>${link(older, '', { en: 'Older report', zh: '更早的报告' })}${link(newer, 'nx', { en: 'Newer report', zh: '更新的报告' })}</nav>`;
}

/** The standard report template, composed from the sections of report.json. */
export function renderStandard(c: Ctx): string {
  const r = c.report;
  const toc = r.sections.filter((s) => s.title).map((s) => `<a href="#${esc(s.id)}">${plainBi(s.title as Bi)}</a>`).join('');
  const secs = r.sections
    .map((s) => `<section class="rsec" id="${esc(s.id)}">${s.title ? `<h2 class="title">${plainBi(s.title)}</h2>` : ''}${s.intro ? `<p class="sub">${bi(s.intro)}</p>` : ''}${renderSection(c, s)}${s.caption && s.component !== 'diagram' ? `<p class="cap">${bi(s.caption)}</p>` : ''}</section>`)
    .join('');
  return `<main id="main">${hero(r)}<div class="wrap rlayout"><nav class="toc" ${al('nav.on_page')}>${toc}</nav><div>${secs}${pager(c.site, r)}</div></div></main>`;
}

/** Everything a bespoke page module (reports/<id>/page.ts) may use. */
export interface BespokeCtx {
  site: Site;
  report: Report;
  /** URL of a file in reports/<id>/, hashed and copied to the output */
  asset: (rel: string) => string;
  imageSize: (rel: string) => { w: number; h: number };
  section: (s: Section) => string;
  root: string;
  kit: { T: typeof T; bi: typeof bi; plainBi: typeof plainBi; t: typeof t; rich: typeof rich; esc: typeof esc; D: typeof D; ic: typeof ic; kpiGrid: typeof kpiGrid; waffle: typeof waffle; pager: () => string };
}

export function bespokeCtx(c: Ctx): BespokeCtx {
  return {
    site: c.site,
    report: c.report,
    root: c.root,
    asset: (rel) => `${c.root}${c.assets.media(c.report.id, rel)}`,
    imageSize: (rel) => c.assets.size(c.report.id, rel),
    section: (s) => renderSection(c, s),
    kit: { T, bi, plainBi, t, rich, esc, D, ic, kpiGrid, waffle, pager: () => pager(c.site, c.report) },
  };
}
