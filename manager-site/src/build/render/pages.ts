import { pathToFileURL } from 'node:url';
import { join } from 'node:path';
import { ic } from '../../shared/icons.ts';
import { newRequirementUrl } from '../../shared/issue.ts';
import { STATUSES, ROLES, type Role } from '../../shared/i18n.ts';
import type { Assets } from '../assets.ts';
import { burndown, chip, areaBars, checks, decisionCard, emptyFrame, gallery, historyItem, kpiGrid, player, reqRow, reportCard, ring, timelineList, waffle } from './components.ts';
import { al, D, esc, plainBi, t, T } from './html.ts';
import type { Page } from './layout.ts';
import { bespokeCtx, renderCompare, renderStandard, shotsOf, stepsOf, type Ctx } from './report.ts';
import { testTotals, type Site } from '../site.ts';
import { count, planned } from '../spec.ts';
import { asNumber, REPORTS_DIR } from '../util.ts';
import type { Report } from '../reports.ts';

export interface Rendered {
  page: Page;
  main: string;
}

const AVATAR: Record<Role, [string, string]> = {
  manager: ['M', '#5e5ce6'],
  backend: ['B', '#0a84ff'],
  frontend: ['F', '#30b0c7'],
  designer: ['D', '#bf5af2'],
  tester: ['T', '#34c759'],
};

const head = (title: string, lede: string, extra = ''): string =>
  `<header style="padding-block:56px 28px;max-width:720px"><h1 class="display">${title}</h1><p class="lede" style="margin-top:12px">${lede}</p>${extra}</header>`;

function pendingBanner(site: Site): string {
  const n = site.pending.length;
  if (!n) {
    return `<div class="wrap"><aside class="pend clear" ${al('decision.pending_aria')}><span class="dot" aria-hidden="true"></span><div class="pend-t"><b>${t('decision.none')}</b> · <span>${T('It will appear here when something needs you.', '有需要时会出现在这里。')}</span></div><a class="btn" href="decisions/">${t('nav.decisions')}</a></div></div>`;
  }
  const first = site.pending[0];
  return `<div class="wrap"><aside class="pend" ${al('decision.pending_aria')}><span class="dot" aria-hidden="true"></span><div class="pend-t"><b>${T(`${n} decision${n === 1 ? ' is' : 's are'} waiting for you`, `${n} 个决策在等你`)}</b>${first ? ` · <span>${T('Next:', '下一个：')} ${plainBi(first.title)}</span>` : ''}</div><a class="btn" href="decisions/">${t('action.review')}</a></aside></div>`;
}

function roleCards(site: Site): string {
  return ROLES.map((r, i) => {
    const s = site.status.roles[r];
    const [letter, color] = AVATAR[r];
    const state = s ? (s.working ? t('role.working') : t('role.idle')) : t('role.idle');
    return `<article class="card role rv" style="--rd:${i}"><div class="role-h"><span class="avatar" style="--av:${color}" aria-hidden="true">${letter}</span><div><h3>${t(`role.${r}`)}</h3><span class="state${s?.working ? ' on' : ''}"><i></i>${state}</span></div></div><div class="task">${s ? plainBi(s.task) : t('role.no_info')}</div>${s?.at ? `<div class="when">${D(s.at)}</div>` : ''}</article>`;
  }).join('');
}

export function overview(site: Site): Rendered {
  const c = site.counts;
  const n = planned(c);
  const t0 = site.tests ? testTotals(site.tests.tests) : null;
  const tag = site.latestTag;
  const st = site.status;
  const headline = st.headline ? plainBi(st.headline) : T(`${c.done} of ${n} planned requirements are done.`, `${n} 项计划内需求已完成 ${c.done} 项。`);
  const kpis = kpiGrid(
    [
      { icon: 'check', label: { en: 'Requirements done', zh: '已完成需求' }, value: c.done, of: n, sub: c.blocked ? { en: `${c.blocked} waiting on the owner`, zh: `${c.blocked} 项在等你` } : undefined },
      { icon: 'swap', label: { en: 'Active or in review', zh: '进行与审核中' }, value: c.doing + c.review, sub: c.doing + c.review ? undefined : { en: 'Nothing is in progress right now', zh: '目前没有进行中的任务' } },
      {
        icon: 'shield',
        label: { en: 'Tests passing', zh: '测试通过' },
        value: t0 ? t0.passed : '—',
        of: t0 ? t0.passed + t0.failed + t0.skipped : undefined,
        sub: t0 ? { en: `${t0.failed} failing`, zh: `${t0.failed} 失败` } : { en: 'No test results recorded yet', zh: '还没有测试结果记录' },
      },
      { icon: 'tag', label: { en: 'Latest build', zh: '最新版本' }, value: tag ?? '—', sub: tag ? (tag.includes('-') ? { en: 'Pre-release, not a stable release yet', zh: '预发布版，尚未稳定发布' } : { en: 'Stable release', zh: '稳定版' }) : { en: 'No release tag found', zh: '没有找到发布标签' } },
    ],
    200,
  );
  const last = site.latest;
  const main = `<main id="main">
${pendingBanner(site)}
<section class="wrap hero"><div>${st.chip ? `<span class="chip ${st.chip.tone}">${ic('check')}${plainBi(st.chip.text)}</span>` : ''}<h1 class="display" style="margin:10px 0 16px">${headline}</h1>${st.lede ? `<p class="lede">${plainBi(st.lede)}</p>` : ''}<div class="meta">${st.updated ? `<span>${T('Updated', '更新于')} ${D(st.updated)}</span>` : ''}${last ? `<a href="latest/">${t('action.latest')}</a>` : ''}</div></div>${ring(c)}</section>
<section class="wrap" ${al('ui.key_numbers')}>${kpis}</section>
<section class="wrap sec"><div class="sec-h"><h2 class="title">${T('Where things stand', '进度一览')}</h2><a href="requirements/">${t('action.all_reqs')}</a></div>
<div class="grid g2"><article class="card rv"><h3 class="h3" style="margin-bottom:20px">${T('Progress by area', '按领域看进度')}</h3>${areaBars(site.spec.areas, count, st.builtBeforePlan)}</article>
<article class="card rv" style="--rd:1"><h3 class="h3">${t('chart.burndown')}</h3><p class="sub" style="margin:4px 0 12px">${site.burndown.target ? T('Open requirements over time, against the ideal pace to the target date.', '仍未完成的需求数量，对比到目标日期的理想进度。') : T('Open requirements per day, from plan/burndown.csv.', '每天仍未完成的需求数量，来自 plan/burndown.csv。')}</p>${burndown(site)}</article></div></section>
<section class="wrap sec"><div class="sec-h"><h2 class="title">${T('What each role is doing', '每个角色在做什么')}</h2></div><div class="grid g3 roles" style="grid-template-columns:repeat(auto-fit,minmax(min(200px,100%),1fr))">${roleCards(site)}</div></section>
<section class="wrap sec"><div class="sec-h"><h2 class="title">${T('Build and test health', '构建与测试状况')}</h2>${site.tests ? `<span class="small">${T('From report', '来自报告')} <a href="reports/${site.tests.report.id}/">${D(site.tests.report.date)}</a></span>` : ''}</div>
<div class="grid g2"><article class="card rv"><h3 class="h3">${T('Test results', '测试结果')}</h3>${
    t0
      ? `<p class="num" style="margin:6px 0 18px;font-size:2.1rem;font-weight:700;letter-spacing:-.03em">${t0.passed} <span class="sub" style="font-size:1rem;font-weight:500">${T(`passed · ${t0.failed} failed · ${t0.skipped} skipped`, `项通过 · ${t0.failed} 失败 · ${t0.skipped} 跳过`)}</span></p>${waffle(t0.passed, t0.failed, t0.skipped)}`
      : `<p class="sub">${T('No test results have been recorded yet.', '还没有记录任何测试结果。')}</p>`
  }</article>
<article class="card rv" style="--rd:1"><h3 class="h3">${T('Checks', '检查项')}</h3>${st.checks.length ? checks(st.checks) : `<p class="sub">${T('No checks are recorded in plan/STATUS.md yet.', 'plan/STATUS.md 里还没有记录检查项。')}</p>`}</article></div></section>
<section class="wrap sec"><div class="sec-h"><h2 class="title">${T('Latest report', '最新报告')}</h2><a href="reports/">${t('action.all_reports')}</a></div>${last ? reportCard(last, 'reports/', true) : emptyFrame({ en: 'No report has been published yet.', zh: '还没有发布任何报告。' }, { en: 'The first report will appear here.', zh: '第一份报告会出现在这里。' })}</section>
</main>`;
  return { page: { path: '/', nav: 'overview', title: { en: 'Overview', zh: '概览' } }, main };
}

export function reportsIndex(site: Site): Rendered {
  const [first, ...rest] = site.reports;
  const body = first
    ? `${reportCard(first, '', true)}<div class="sec-h" style="margin-top:56px"><h2 class="title">${T('All reports', '全部报告')}</h2></div><div class="grid g3" style="grid-template-columns:repeat(auto-fill,minmax(min(300px,100%),1fr))">${[first, ...rest].map((r) => reportCard(r, '')).join('')}</div>`
    : emptyFrame({ en: 'No report has been published yet.', zh: '还没有发布任何报告。' }, { en: 'Reports appear here as they are written.', zh: '写好的报告会出现在这里。' });
  const main = `<main id="main" class="wrap">${head(T('Reports', '报告'), T('A record of every milestone and weekly update. Milestones get a page of their own.', '每次里程碑与每周进展的记录。里程碑报告有专属页面。'))}${body}</main>`;
  return { page: { path: '/reports/', nav: 'reports', title: { en: 'Reports', zh: '报告' } }, main };
}

export function latestRedirect(latest: Report): string {
  const to = `../reports/${latest.id}/`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<meta http-equiv="refresh" content="0; url=${to}">
<link rel="canonical" href="${to}">
<title>Latest report · Cloud Home Report</title>
<style>body{margin:0;min-height:100vh;display:grid;place-items:center;font:1.0625rem/1.5 -apple-system,BlinkMacSystemFont,"PingFang SC","Helvetica Neue",system-ui,sans-serif;background:#fbfbfd;color:#1d1d1f;text-align:center}a{color:#0066cc}@media(prefers-color-scheme:dark){body{background:#000;color:#f5f5f7}a{color:#2997ff}}</style>
<script>location.replace(${JSON.stringify(to)})</script>
</head>
<body>
<p><span lang="en">Opening the latest report…</span> <span lang="zh-Hans">正在打开最新报告…</span><br>
<a href="${to}"><span lang="en">Continue</span> · <span lang="zh-Hans">继续</span></a></p>
</body>
</html>
`;
}

export function timeline(site: Site): Rendered {
  const types = ['commit', 'release', 'report', 'decision', 'spec'] as const;
  const present = types.filter((ty) => site.events.some((e) => e.type === ty));
  const filters = `<div class="filters" id="tlFilters" role="group" ${al('ui.filter_events')} style="margin-bottom:8px"><button class="fchip" data-f="all" aria-pressed="true" type="button">${t('ui.all')}</button>${present.map((ty) => `<button class="fchip" data-f="${ty}" aria-pressed="false" type="button">${t(`event.${ty}`)}</button>`).join('')}</div>`;
  const body = site.events.length ? timelineList(site.events, '../', 'tl') : emptyFrame({ en: 'No events yet.', zh: '还没有任何事件。' }, { en: 'Commits, releases, reports and decisions will appear here.', zh: '提交、发布、报告和决策会出现在这里。' });
  const cut = site.commits.length >= 400 ? `<p class="small">${T('Only the newest 400 commits are listed.', '只列出最新的 400 个提交。')}</p>` : '';
  const main = `<main id="main" class="read">${head(T('Timeline', '时间线'), T('Everything that happened: commits, releases, reports and decisions, newest first.', '发生过的每件事：提交、发布、报告和决策，最新的在最上面。'))}${filters}${body}${cut}<p class="small" style="margin-top:24px">${T('Commit hashes link to the commit on GitHub. Only the subject line of each commit is shown, in English. Times are UTC.', '提交哈希会链接到 GitHub 上对应的提交。每个提交只显示英文标题行。时间为 UTC。')}</p></main>`;
  return { page: { path: '/timeline/', nav: 'timeline', title: { en: 'Timeline', zh: '时间线' } }, main };
}

export function requirements(site: Site): Rendered {
  const c = site.counts;
  const summary = [...(['done', 'review', 'doing', 'blocked', 'todo', 'dropped'] as const)].map((s) => `<span class="sc">${chip(s)}<b class="num">${c[s]}</b></span>`).join('');
  const filters = (['all', ...STATUSES] as const).map((s) => `<button class="fchip" data-f="${s}" aria-pressed="${s === 'all'}" type="button">${s === 'all' ? t('ui.all') : t(`status.${s}`)}<span class="n">${s === 'all' ? site.spec.reqs.length : c[s]}</span></button>`).join('');
  const areas = site.spec.areas
    .map((a) => {
      const ac = count(a.reqs);
      const an = planned(ac);
      return `<section class="area" data-a="${a.key}"><div class="area-h"><h2>${plainBi(a.name)}</h2><span class="small">${T(`${ac.done} of ${an} done`, `${ac.done}/${an} 已完成`)}</span><div class="mini"><i style="--w:${an ? (ac.done / an) * 100 : 0}%"></i></div></div><ul class="reqs">${a.reqs.map(reqRow).join('')}</ul></section>`;
    })
    .join('');
  const main = `<main id="main" class="wrap">
<header style="padding-block:56px 24px;display:flex;align-items:flex-end;justify-content:space-between;gap:24px;flex-wrap:wrap"><div style="max-width:640px"><h1 class="display">${t('nav.requirements')}</h1><p class="lede" style="margin-top:12px">${T('A read-only view of SPEC.md with live status. To change anything, use a button to open a pre-filled GitHub issue.', '来自 SPEC.md 的只读视图，状态实时更新。想改什么，用按钮提交一个预填好的 GitHub 议题。')}</p></div><a class="btn btn-sec btn-lg" href="${esc(newRequirementUrl())}" target="_blank" rel="noopener">${ic('plus')}${t('action.new_req')}</a></header>
<div class="card" style="padding:14px 18px;margin-bottom:20px"><div class="summary-chips" role="group" ${al('ui.status_totals')}>${summary}</div></div>
<div class="row filter-bar"><div class="filters" id="reqFilters" role="group" ${al('ui.filter_status')}>${filters}</div><label class="search"><span class="vh">${t('ui.search_req')}</span>${ic('search')}<input id="reqSearch" type="search" placeholder="R-012, upload…" data-ph-en="R-012, upload…" data-ph-zh="R-012、上传…" autocomplete="off"></label></div>
<p class="small m-only" style="margin:4px 0 0">${T('Tap a row to reveal Change and Remove.', '点一行即可展开，使用“修改”或“移除”。')}</p>
<div id="reqs">${areas}<div class="empty card" id="noRes" hidden><b>${T('No requirement matches.', '没有匹配的需求。')}</b><p class="sub">${T('Try another status or clear the search.', '换一个状态，或清除搜索。')}</p></div></div>
<p class="small" style="margin-top:20px">${T('“Change” and “Remove” open a pre-filled GitHub issue in a new tab, titled [CHG R-nnn] or [DEL R-nnn]. Nothing is sent until you press Submit new issue. Titles shown in Chinese come from content/requirements.zh.json.', '“修改”和“移除”会在新标签页打开预填好的 GitHub 议题，标题带 [CHG R-nnn] 或 [DEL R-nnn]；你点“Submit new issue”后才会发送。中文标题来自 content/requirements.zh.json。')}</p></main>`;
  return { page: { path: '/requirements/', nav: 'requirements', title: { en: 'Requirements', zh: '需求' } }, main };
}

export function decisions(site: Site): Rendered {
  const history = site.approvals.filter((a) => a.state !== 'pending');
  const steps = `<ol class="steps card" style="padding:22px 24px;margin:0 0 40px;list-style:none"><li><b>${T('Tap Approve or Deny', '点“批准”或“拒绝”')}</b><span>${T('A pre-filled GitHub issue opens in a new tab.', '会在新标签页打开一个预填好的 GitHub 议题。')}</span></li><li><b>${T('Press Submit new issue', '按“Submit new issue”')}</b><span>${T('The title is ready; add a note if you like.', '标题已写好，内容无需修改；想补充可以加一句备注。')}</span></li><li><b>${T('The manager picks it up', '经理在下个周期处理')}</b><span>${T('It replies on the issue, records the answer and carries on.', '回复议题、记录结果并继续工作。')}</span></li></ol>`;
  const pend = site.pending.length
    ? `<div class="sec-h"><h2 class="title">${T('Waiting for your answer', '等待回答')}</h2><span class="chip warn">${t('ui.pending_n', { n: site.pending.length })}</span></div>${site.pending.map(decisionCard).join('')}`
    : `<div class="sec-h"><h2 class="title">${T('Waiting for your answer', '等待回答')}</h2></div><div class="card allclear"><span class="k-ic ok-ic">${ic('check')}</span><h3 class="h3">${t('decision.caught_up')}</h3><p class="sub" style="margin:6px 0 0">${T('The team does not need anything from you right now. When it does, the request appears here and at the top of the Overview.', '团队目前不需要你的任何回答。需要时，请求会出现在这里，并显示在概览页顶部。')}</p></div>`;
  const hist = history.length
    ? `<div class="card" style="padding:8px 24px"><ul class="hist">${history.map(historyItem).join('')}</ul></div>`
    : `<div class="card allclear"><p class="sub" style="margin:0">${T('No decisions have been made yet.', '还没有做过决策。')}</p></div>`;
  const main = `<main id="main" class="wrap">${head(t('nav.decisions'), T('Things the team needs your yes or no on before going further. Answering takes two taps, from a phone too.', '团队需要你点头之后才能继续的事。回答只需两步，手机上也能完成。'))}${steps}<section>${pend}</section><section><div class="sec-h" style="margin-top:64px"><h2 class="title">${T('History', '历史')}</h2></div>${hist}</section></main>`;
  return { page: { path: '/decisions/', nav: 'decisions', title: { en: 'Decisions', zh: '决策' } }, main };
}

export function product(site: Site, assets: Assets): Rendered {
  let walks = '';
  let shots = '';
  let compares = '';
  for (const r of site.reports) {
    const ctx: Ctx = { site, assets, report: r, root: '../' };
    const label = `<span class="small">${plainBi(r.title)} · ${D(r.date)}</span>`;
    for (const s of r.sections) {
      if (s.component === 'walkthrough') walks += `<section style="margin-bottom:56px"><div class="sec-h"><h2 class="title">${s.title ? plainBi(s.title) : T('Walkthrough', '动画演示')}</h2>${label}</div>${player(stepsOf(ctx, s), s.data.duration_ms === undefined ? 3400 : asNumber(s.data.duration_ms, `${r.id} ${s.id}: duration_ms`))}</section>`;
      if (s.component === 'gallery') shots += `<section style="margin-bottom:56px"><div class="sec-h"><h2 class="title">${s.title ? plainBi(s.title) : T('Screenshots', '截图')}</h2>${label}</div>${gallery(shotsOf(ctx, s))}</section>`;
      if (s.component === 'compare') compares += `<section style="margin-bottom:56px"><div class="sec-h"><h2 class="title">${s.title ? plainBi(s.title) : T('Before and after', '前后对比')}</h2>${label}</div>${renderCompare(ctx, s)}</section>`;
    }
  }
  const body =
    walks + shots + compares ||
    `<section aria-labelledby="empty-h">${emptyFrame(
      { en: 'The web app is not built yet.', zh: '网页应用还没有开发。' },
      { en: 'Screenshots and walkthroughs will appear here.', zh: '截图和演示会出现在这里。' },
      `<div class="preview-list"><div><b>${T('Screenshots', '截图')}</b><span>${T('From the running app, in light and dark and three languages.', '来自真实运行的界面，浅色与深色、三种语言。')}</span></div><div><b>${T('Walkthroughs', '动画演示')}</b><span>${T('Short clips of signing in, uploading and previewing.', '几秒钟的短片，演示登录、上传、预览。')}</span></div><div><b>${T('Before and after', '前后对比')}</b><span>${T('Drag to compare each redesign.', '每次改版，可拖动对比。')}</span></div></div><a class="btn btn-sec" href="../requirements/">${T('See the web app requirements ›', '查看网页应用的需求 ›')}</a>`,
    )}</section>`;
  const main = `<main id="main" class="wrap">${head(t('nav.product'), T('Screenshots and animated walkthroughs of the real Cloud Home app, latest and history.', '真实 Cloud Home 界面的截图与动画演示，最新的和历史版本。'))}${body}</main>`;
  return { page: { path: '/product/', nav: 'product', title: { en: 'Product', zh: '产品' } }, main };
}

/** A report page: the bespoke module when report.json names one, otherwise the standard template. */
export async function reportPage(site: Site, assets: Assets, r: Report): Promise<Rendered> {
  const path = `/reports/${r.id}/`;
  const ctx: Ctx = { site, assets, report: r, root: '../../' };
  const base = { path, nav: 'reports' as const, title: r.title };
  if (!r.page) return { page: base, main: renderStandard(ctx) };
  const mod = (await import(pathToFileURL(join(REPORTS_DIR, r.id, r.page)).href)) as { default?: (c: ReturnType<typeof bespokeCtx>) => string };
  if (typeof mod.default !== 'function') throw new Error(`reports/${r.id}/${r.page}: it must export a default function (ctx) => string`);
  const main = mod.default(bespokeCtx(ctx));
  if (!main.includes('id="main"')) throw new Error(`reports/${r.id}/${r.page}: the returned HTML must contain <main id="main">`);
  return { page: { ...base, css: r.pageCss ? `${assets.media(r.id, r.pageCss)}` : undefined }, main };
}
