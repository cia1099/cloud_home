/* Mockup behaviour for the Cloud Home report site. Illustrative: the real site (manager-site/) renders the same
   markup from reports/<id>/report.json and SPEC.md at build time. Sample data below is NOT real project status. */
(() => {
  'use strict';
  const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const B = document.body, H = document.documentElement;
  const root = B.dataset.root || './', page = B.dataset.page, path = B.dataset.path || '/';
  const RM = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const SAMPLE_PAGE = B.hasAttribute('data-sample'); /* templates and placeholder-art pages: always sample */
  const STATE = B.hasAttribute('data-real') ? 'real' : H.dataset.state, REAL = STATE === 'real' && !SAMPLE_PAGE;
  const T = (en, zh) => `<span lang="en">${en}</span><span lang="zh-Hans">${zh}</span>`;
  const esc = (s) => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
  const ISSUE = 'https://github.com/cia1099/cloud_home/issues/new';
  const issue = (t, b) => `${ISSUE}?title=${encodeURIComponent(t)}&body=${encodeURIComponent(b)}`;
  const D = (iso, o) => { const d = new Date(iso + 'T12:00:00Z'); o = { timeZone: 'UTC', ...o };
    return T(new Intl.DateTimeFormat('en', o).format(d), new Intl.DateTimeFormat('zh-Hans', o).format(d)); };

  /* ---------- icons (original, 24 grid, 1.75 stroke) ---------- */
  const P = {
    overview: '<rect x="3.5" y="3.5" width="7" height="7" rx="2"/><rect x="13.5" y="3.5" width="7" height="7" rx="2"/><rect x="3.5" y="13.5" width="7" height="7" rx="2"/><rect x="13.5" y="13.5" width="7" height="7" rx="2"/>',
    reports: '<path d="M6 3.5h8l4 4V20a.5.5 0 0 1-.5.5h-11A.5.5 0 0 1 6 20z"/><path d="M14 3.5V8h4M9 12.5h6M9 16h6"/>',
    timeline: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
    requirements: '<path d="M9 6.5h11M9 12h11M9 17.5h11"/><circle cx="4.5" cy="6.5" r=".9"/><circle cx="4.5" cy="12" r=".9"/><circle cx="4.5" cy="17.5" r=".9"/>',
    product: '<rect x="3.5" y="5" width="17" height="14" rx="3"/><circle cx="9" cy="10" r="1.6"/><path d="M4 17l5-4.5 3.5 3L15 13l5 4.5"/>',
    decisions: '<circle cx="12" cy="12" r="8.5"/><path d="M8.2 12.3l2.6 2.6 5-5.4"/>',
    ask: '<circle cx="12" cy="12" r="8.5"/><path d="M9.6 9.6a2.5 2.5 0 1 1 3.6 2.2c-.8.5-1.2 1-1.2 1.9M12 16.6v.1"/>',
    plus: '<path d="M12 5v14M5 12h14"/>', pencil: '<path d="M4 20l1-4L16.5 4.5a2 2 0 0 1 3 3L8 19z"/>', xmark: '<path d="M6 6l12 12M18 6L6 18"/>',
    more: '<circle cx="5.5" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="18.5" cy="12" r="1.3"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6L7 7M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4"/>',
    moon: '<path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z"/>', auto: '<circle cx="12" cy="12" r="8.5"/><path d="M12 3.5v17" /><path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="currentColor"/>',
    chev: '<path d="M9 5l7 7-7 7"/>', chevL: '<path d="M15 5l-7 7 7 7"/>', up: '<path d="M8 16L16 8M9 8h7v7"/>', play: '<path d="M8 5.5v13l11-6.5z" fill="currentColor"/>',
    commit: '<circle cx="12" cy="12" r="3.5"/><path d="M3 12h5.5M15.5 12H21"/>', warn: '<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17v.1"/>',
    tag: '<path d="M4 4h8l8 8-8 8-8-8z"/><circle cx="8.5" cy="8.5" r="1"/>', check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>', info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8v.1"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4 4"/>', cloud: '<path d="M7.5 18h9a4 4 0 0 0 .7-7.9A5.5 5.5 0 0 0 6.8 11 3.5 3.5 0 0 0 7.5 18z"/>',
    drive: '<rect x="3.5" y="7" width="17" height="10" rx="3"/><path d="M7 12h5"/><circle cx="16.5" cy="12" r=".9"/>', db: '<ellipse cx="12" cy="6.5" rx="7" ry="3"/><path d="M5 6.5v11c0 1.7 3.1 3 7 3s7-1.3 7-3v-11M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3"/>',
    window: '<rect x="3.5" y="5" width="17" height="14" rx="3"/><path d="M3.5 9.5h17"/>', server: '<rect x="4" y="4" width="16" height="6.5" rx="2.5"/><rect x="4" y="13.5" width="16" height="6.5" rx="2.5"/><path d="M8 7.2v.1M8 16.8v.1"/>', undo: '<path d="M9 7L4.5 11.5 9 16M5 11.5h8a6 6 0 0 1 6 6"/>', refresh: '<path d="M19 12a7 7 0 1 1-2.2-5.1M19 4.5v4.2h-4.2"/>', phone: '<rect x="7" y="3" width="10" height="18" rx="3"/><path d="M11 18h2"/>', pause: '<path d="M8 5.5v13M16 5.5v13"/>', sliders: '<path d="M4 8h9M17 8h3M4 16h3M11 16h9"/><circle cx="15" cy="8" r="2"/><circle cx="9" cy="16" r="2"/>',
    shield: '<path d="M12 3.5l7 2.5v5.5c0 4.2-2.9 7.6-7 9-4.1-1.4-7-4.8-7-9V6z"/><path d="M9 12l2.2 2.2L15.2 10"/>', down: '<path d="M12 4v11M7 11l5 5 5-5M5 20h14"/>', swap: '<path d="M4 8h13l-3-3M20 16H7l3 3"/>', power: '<path d="M12 3.5v8M7 6.5a7.5 7.5 0 1 0 10 0"/>', heart: '<path d="M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.4 4.3 4.3 0 0 1 19.5 10c0 5.4-7.5 10-7.5 10z"/>', lock: '<rect x="5" y="10.5" width="14" height="9.5" rx="2.5"/><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"/>'
  };
  const ic = (n, c = '') => `<svg class="ic ${c}" viewBox="0 0 24 24" aria-hidden="true">${P[n]}</svg>`;
  window.CHI = ic;
  const SG = {
    todo: '<circle cx="6" cy="6" r="4.2"/>', doing: '<circle cx="6" cy="6" r="4.2"/><path d="M6 1.8a4.2 4.2 0 0 1 0 8.4z" fill="currentColor"/>',
    review: '<circle cx="5.2" cy="5.2" r="3"/><path d="M7.6 7.6l2.9 2.9"/>', done: '<circle cx="6" cy="6" r="5" fill="currentColor" stroke="none"/><path d="M3.8 6.2l1.6 1.6 2.9-3.3" style="stroke:var(--st-bg)"/>',
    blocked: '<circle cx="6" cy="6" r="4.2"/><path d="M4.7 4v4M7.3 4v4"/>', dropped: '<circle cx="6" cy="6" r="4.2"/><path d="M3.8 6h4.4"/>'
  };
  const SL = { todo: ['To do', '待办'], doing: ['In progress', '进行中'], review: ['In review', '审核中'], done: ['Done', '已完成'], blocked: ['Blocked', '受阻'], dropped: ['Dropped', '已取消'] };
  const chip = (s) => `<span class="chip" data-s="${s}"><svg viewBox="0 0 12 12" aria-hidden="true">${SG[s]}</svg>${T(...SL[s])}</span>`;

  /* ---------- sample data ---------- */
  const BUILT = { backend: 10 }; /* STATUS.md: capabilities shipped before the plan began (core API, auth, files, trash, shares, search, SSE, usage, admin, OTA updater) */
  const AREAS = { site: ['Report site', '报告网站'], web: ['Web app', '网页应用'], backend: ['Backend', '后端'], private: ['Private access', '私有访问'], quality: ['Quality', '质量'], release: ['Release', '发布'] };
  const OWN = { manager: ['Manager', '经理'], backend: ['Backend engineer', '后端工程师'], frontend: ['Frontend engineer', '前端工程师'], designer: ['Designer', '设计师'], tester: ['Tester', '测试工程师'] };
  const R = (id, pri, s, a, en, zh, o, why) => ({ id, pri, s, a, en, zh, o, why });
  const REQS_SAMPLE = [
    R('R-001', 'P0', 'review', 'site', 'Manager report site', '经理报告网站', 'frontend'),
    R('R-002', 'P0', 'review', 'web', 'Design system and screens', '设计系统与界面', 'designer'),
    R('R-003', 'P0', 'done', 'web', 'Web app foundation', 'Web 应用基础架构', 'frontend'),
    R('R-004', 'P0', 'done', 'web', 'Visual direction: Apple web style', '视觉方向：苹果网页风格', 'designer'),
    R('R-005', 'P0', 'doing', 'web', 'Internationalization: English, 简体中文, 繁體中文', '国际化：英文、简体中文、繁体中文', 'frontend'),
    R('R-006', 'P0', 'review', 'web', 'Sign in, register and sign out', '登录、注册与退出', 'frontend'),
    R('R-007', 'P0', 'doing', 'web', 'File browser: grid, list, breadcrumb, selection', '文件浏览器：网格、列表、面包屑、多选', 'frontend'),
    R('R-008', 'P0', 'doing', 'web', 'Upload with per-file progress and cancel', '带逐文件进度和取消的上传', 'frontend'),
    R('R-009', 'P1', 'todo', 'web', 'Preview: image gallery, audio and video', '预览：图片库、音频与视频', 'frontend'),
    R('R-010', 'P1', 'todo', 'web', 'Batch and single-file download', '批量与单文件下载', 'frontend'),
    R('R-011', 'P1', 'todo', 'web', 'Trash, sharing and storage usage', '回收站、分享与存储用量', 'frontend'),
    R('R-012', 'P1', 'todo', 'web', 'Account settings', '账户设置', 'frontend'),
    R('R-013', 'P1', 'todo', 'web', 'Admin update page', '管理员更新页面', 'frontend'),
    R('R-027', 'P1', 'todo', 'web', 'The UI follows updates without a broken screen', '界面随更新同步，不出现损坏画面', 'frontend'),
    R('R-062', 'P0', 'todo', 'web', 'Approval states in the web app', '网页端的审批状态', 'designer'),
    R('R-020', 'P0', 'doing', 'backend', 'OTA admin API', 'OTA 管理接口', 'backend'),
    R('R-021', 'P0', 'todo', 'backend', 'Device packaging and installer', '设备打包与安装脚本', 'backend'),
    R('R-022', 'P1', 'blocked', 'backend', 'Real one-time-code delivery', '真实的一次性验证码发送', 'backend', ['Needs an email or SMS provider choice from the owner', '需要主人选择邮件或短信服务商']),
    R('R-023', 'P2', 'dropped', 'backend', 'Google / Apple sign-in', '谷歌 / 苹果登录', 'backend'),
    R('R-024', 'P2', 'todo', 'backend', 'Registration ownership verification', '注册身份归属验证', 'backend'),
    R('R-025', 'P1', 'todo', 'backend', 'Close API gaps reported by design and frontend', '补齐设计与前端报告的接口缺口', 'backend'),
    R('R-026', 'P1', 'todo', 'backend', 'One binary serves the API and the UI', '单一程序同时提供接口与界面', 'backend'),
    R('R-060', 'P0', 'done', 'private', 'Approval states and registration modes', '审批状态与注册模式', 'backend'),
    R('R-061', 'P0', 'done', 'private', 'Private admin endpoints for approvals', '审批用的私有管理接口', 'backend'),
    R('R-063', 'P0', 'review', 'private', 'Chat core with pairing and Approve / Reject buttons', '聊天核心：配对与批准 / 拒绝按钮', 'backend'),
    R('R-064', 'P1', 'todo', 'private', 'Admin operations in chat', '在聊天中进行管理操作', 'backend'),
    R('R-065', 'P2', 'todo', 'private', 'Chat alerts', '聊天提醒', 'backend'),
    R('R-068', 'P2', 'blocked', 'private', 'Additional chat providers', '更多聊天服务商', 'backend', ['Needs a decision on how a webhook reaches the device', '需要决定 Webhook 如何到达设备']),
    R('R-030', 'P0', 'doing', 'quality', 'Backend integration test suite', '后端集成测试套件', 'tester'),
    R('R-031', 'P0', 'review', 'quality', 'OTA end-to-end script', 'OTA 端到端脚本', 'tester'),
    R('R-032', 'P1', 'todo', 'quality', 'UI end-to-end tests in three languages', '三种语言的界面端到端测试', 'tester'),
    R('R-066', 'P0', 'todo', 'quality', 'Approval and chat test suite', '审批与聊天测试套件', 'tester'),
    R('R-070', 'P1', 'todo', 'quality', 'Idle-code sweep', '清理闲置代码', 'backend'),
    R('R-040', 'P1', 'todo', 'release', 'Cut stable v0.3.1', '发布稳定版 v0.3.1', 'manager'),
    R('R-050', 'P0', 'done', 'release', 'Owner-request intake via GitHub issues', '通过 GitHub 议题接收主人的请求', 'manager')
  ];
  /* Real state (24 Nov 2026, SPEC.md): every requirement is todo except the three that wait for the owner. */
  const BLOCKED_REAL = { 'R-022': 0, 'R-023': ['Needs developer-account credentials from the owner', '需要主人提供开发者账号凭据'], 'R-068': 0 };
  const REQS = REAL ? REQS_SAMPLE.map((r) => ({ ...r, s: r.id in BLOCKED_REAL ? 'blocked' : 'todo', why: BLOCKED_REAL[r.id] || r.why })) : REQS_SAMPLE;
  const BURN = REAL ? [35] : [35, 35, 35, 35, 34, 34, 33, 33, 32, 32, 31, 31, 31, 30, 30]; // open requirements per day
  const BURN_START = REAL ? '2026-11-24' : '2026-11-19', BURN_END = '2026-12-31', TESTS = REAL ? 63 : 212;
  const EVENTS_REAL = [
    { d: '2026-11-24', t: 'release', en: 'Pre-release v0.3.1-rc.2 published', zh: '预发布版 v0.3.1-rc.2 已发布', tag: 'v0.3.1-rc.2' },
    { d: '2026-11-24', t: 'report', en: 'Report published: Backend foundation and the self-updating box', zh: '报告已发布：后端基础与会自我更新的盒子', ref: '2026-11-24-backend-foundation' },
    { d: '2026-11-24', t: 'commit', en: 'fix: let the headless manager wait for background subagents', zh: '修复：让无界面经理等待后台子代理', h: '09f1d2f' },
    { d: '2026-11-24', t: 'commit', en: 'docs: add SPEC, approval/chat design, report-site deploy rules and manager settings', zh: '文档：加入 SPEC、审批/聊天设计、报告站部署规则与经理设置', h: '37017fb' },
    { d: '2026-11-23', t: 'commit', en: 'add agents', zh: '加入各角色代理', h: 'a0a5461' },
    { d: '2026-11-19', t: 'commit', en: 'fix(ci): use macos-latest runner for the Apple Silicon build', zh: '修复 CI：Apple Silicon 构建改用 macos-latest', h: 'ce58fb8' },
    { d: '2026-11-18', t: 'release', en: 'Pre-release v0.3.1-rc.1', zh: '预发布版 v0.3.1-rc.1', h: 'e94badd', tag: 'v0.3.1-rc.1' },
    { d: '2026-11-18', t: 'commit', en: 'feat: add signed online self-update with automatic rollback', zh: '新功能：带签名的在线自更新与自动回滚', h: '3dec8fd' },
    { d: '2026-09-21', t: 'commit', en: 'feat: add admin endpoint to purge an account\'s trash instantly', zh: '新功能：管理员可立即清空某账户的回收站', h: 'b2395b6' },
    { d: '2026-09-21', t: 'commit', en: 'feat: move credentials into identities table with editable profile', zh: '新功能：凭据迁入身份表，资料可编辑', h: 'b3cea10' },
    { d: '2026-09-15', t: 'commit', en: 'feat: auto-suffix duplicate names on upload and folder-create', zh: '新功能：上传与新建文件夹时自动为重名加后缀', h: '905b488' },
    { d: '2026-09-15', t: 'commit', en: 'feat: add structured logging and audit trail', zh: '新功能：结构化日志与审计记录', h: '1b695f2' },
    { d: '2026-09-14', t: 'release', en: 'Release v0.2.0', zh: '发布 v0.2.0', h: '42c824c', tag: 'v0.2.0' },
    { d: '2026-09-14', t: 'commit', en: 'feat: support in-browser audio/video playback via /files/{id}/raw', zh: '新功能：支持通过 /files/{id}/raw 在浏览器播放音视频', h: '2a993c4' }
  ];
  const EVENTS_SAMPLE = [
    { d: '2026-12-03', t: 'report', en: 'Milestone report published: The first working UI', zh: '里程碑报告已发布：第一个能用的界面', ref: 'm1-first-ui' },
    { d: '2026-12-03', t: 'status', en: 'R-003 done: web app foundation merged', zh: 'R-003 已完成：网页应用基础架构已合并', ref: 'R-003', h: '9c41d2e' },
    { d: '2026-12-02', t: 'decision', en: 'Decision A-003 requested: tag and publish v0.3.1', zh: '已发起决策 A-003：打标签并发布 v0.3.1', ref: 'A-003' },
    { d: '2026-12-01', t: 'status', en: 'R-060 and R-061 done: approval states and admin endpoints', zh: 'R-060、R-061 已完成：审批状态与管理接口', ref: 'R-060', h: '51be07a' },
    { d: '2026-11-24', t: 'commit', en: 'docs: add SPEC, approval/chat design, report-site deploy rules and manager settings', zh: '文档：加入 SPEC、审批/聊天设计、报告站部署规则与经理设置', h: '37017fb' },
    { d: '2026-11-24', t: 'report', en: 'Report published: Backend foundation and the self-updating box', zh: '报告已发布：后端基础与会自我更新的盒子', ref: '2026-11-24-backend-foundation' },
    { d: '2026-11-23', t: 'commit', en: 'add agents', zh: '加入各角色代理', h: 'a0a5461' },
    { d: '2026-11-18', t: 'commit', en: 'fix(ci): use macos-latest runner for the Apple Silicon build', zh: '修复 CI：Apple Silicon 构建改用 macos-latest', h: 'ce58fb8' },
    { d: '2026-11-18', t: 'release', en: 'Pre-release v0.3.1-rc.1 prepared', zh: '预发布版 v0.3.1-rc.1 已准备', h: 'e94badd', tag: 'v0.3.1-rc.1' },
    { d: '2026-11-17', t: 'commit', en: 'feat: add signed online self-update with automatic rollback', zh: '新功能：带签名的在线自更新与自动回滚', h: '3dec8fd' }
  ];
  const ET = { commit: ['Commit', '提交', 'commit'], release: ['Release', '发布', 'tag'], report: ['Report', '报告', 'reports'], decision: ['Decision', '决策', 'decisions'], status: ['Status', '状态', 'check'] };
  const EVENTS = REAL ? EVENTS_REAL : EVENTS_SAMPLE;
  const PENDING = REAL ? 0 : 2;
  const count = (rs) => rs.reduce((m, r) => (m[r.s]++, m), { todo: 0, doing: 0, review: 0, done: 0, blocked: 0, dropped: 0 });
  const C = count(REQS), TOTAL = REQS.length, ACTIVE = TOTAL - C.dropped;
  const SEGS = [['done', 'c-done'], ['review', 'c-review'], ['doing', 'c-doing'], ['blocked', 'c-blocked'], ['todo', 'c-todo']];

  /* ---------- language, theme ---------- */
  const setL = (l) => { H.dataset.l = l; H.lang = l === 'zh' ? 'zh-Hans' : 'en'; try { localStorage.chL = l; } catch (e) {}
    $$('[data-setl]').forEach((b) => b.setAttribute('aria-pressed', b.dataset.setl === l));
    $$('[data-al-en]').forEach((el) => el.setAttribute('aria-label', l === 'zh' ? el.dataset.alZh : el.dataset.alEn));
    $$('[data-ph-en]').forEach((el) => { el.placeholder = l === 'zh' ? el.dataset.phZh : el.dataset.phEn; });
    if (l === 'zh' && B.dataset.titleZh) document.title = B.dataset.titleZh; else document.title = B.dataset.titleEn || document.title;
    document.dispatchEvent(new Event('langchange')); };
  B.dataset.titleEn = document.title;
  const setT = (t) => { H.dataset.tp = t; H.dataset.theme = t === 'auto' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : t; try { localStorage.chT = t; } catch (e) {};
    const tb = $('#themeBtn'); if (tb) tb.innerHTML = ic(t === 'auto' ? 'auto' : t === 'dark' ? 'moon' : 'sun') + `<span class="vh">${T('Appearance: ' + t, '外观：' + { auto: '自动', light: '浅色', dark: '深色' }[t])}</span>`;
    $$('[data-sett]').forEach((b) => b.setAttribute('aria-pressed', b.dataset.sett === t)); };

  /* ---------- bilingual content wrap ---------- */
  $$('[data-zh]').forEach((el) => { el.innerHTML = T(el.innerHTML, el.dataset.zh); });

  $$('[data-ic]').forEach((el) => el.insertAdjacentHTML('afterbegin', ic(el.dataset.ic)));

  /* ---------- chrome ---------- */
  const NAV = [['overview', '', 'Overview', '概览'], ['reports', 'reports/', 'Reports', '报告'], ['timeline', 'timeline/', 'Timeline', '时间线'], ['requirements', 'requirements/', 'Requirements', '需求'], ['product', 'product/', 'Product', '产品'], ['decisions', 'decisions/', 'Decisions', '决策']];
  const href = (p) => root + p; /* every URL is a real folder with index.html; links end in "/" */
  const cur = (k) => (page === k ? ' aria-current="page"' : '');
  const askHref = issue('[Q] ', `**Question**\n\n\n\n---\nAbout page: ${path}`);
  const logo = '<svg viewBox="0 0 28 28" aria-hidden="true"><defs><linearGradient id="lg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#42a5ff"/><stop offset="1" stop-color="#0a5fd8"/></linearGradient></defs><rect width="28" height="28" rx="7" fill="url(#lg)"/><path d="M9.4 20h9.2a3.6 3.6 0 0 0 .5-7.2A5.4 5.4 0 0 0 8.7 13.7 3.2 3.2 0 0 0 9.4 20z" fill="#fff"/></svg>';
  const hdr = `<a class="skip" href="#main">${T('Skip to content', '跳到正文')}</a>
  <header class="hdr"><div class="hdr-in">
    <a class="brand" href="${href('')}">${logo}<span>Cloud Home<span class="brand-s">${T('Report', '报告')}</span></span></a>
    <nav class="nav" aria-label="${'Main'}">${NAV.map(([k, p, en, zh]) => `<a href="${href(p)}"${cur(k)}>${T(en, zh)}${k === 'decisions' && PENDING ? `<span class="badge" aria-label="${PENDING} pending">${PENDING}</span>` : ''}</a>`).join('')}</nav>
    <div class="hdr-r">
      <a class="btn btn-sec ask" href="${esc(askHref)}" target="_blank" rel="noopener">${ic('ask')}<span class="ask-t">${T('Ask a question', '提问')}</span></a>
      <div class="seg" role="group" aria-label="Language / 语言"><button data-setl="en" lang="en" aria-label="English" aria-pressed="true">EN</button><button data-setl="zh" lang="zh-Hans" aria-label="简体中文" aria-pressed="false">中文</button></div>
      <button class="icon-btn" id="themeBtn"></button>
    </div></div></header>
  ${REAL && !SAMPLE_PAGE ? '' : `<div class="samplebar" role="note"><span class="sample">${T('Sample', '示例')}</span><span>${SAMPLE_PAGE ? T('This page is a template filled with made-up numbers and placeholder art.', '本页是用虚构数字和占位图填充的模板。') : T('Populated demo with made-up numbers.', '用虚构数字填充的演示。')}</span>${SAMPLE_PAGE ? '' : `<a href="?state=real">${T('Show the real state', '查看真实状态')}</a>`}</div>`}`;
  const tabs = [['overview', '', 'overview', 'Overview', '概览'], ['reports', 'reports/', 'reports', 'Reports', '报告'], ['requirements', 'requirements/', 'requirements', 'Requirements', '需求'], ['decisions', 'decisions/', 'decisions', 'Decisions', '决策']];
  const tab = `<nav class="tabbar" aria-label="Main">${tabs.map(([k, p, i, en, zh]) => `<a href="${href(p)}"${cur(k)}>${ic(i)}<span>${T(en, zh)}</span>${k === 'decisions' && PENDING ? `<span class="badge">${PENDING}</span>` : ''}</a>`).join('')}<button id="moreBtn" aria-haspopup="dialog">${ic('more')}<span>${T('More', '更多')}</span></button></nav>
  <div class="sheet" id="sheet" role="dialog" aria-modal="true" aria-label="More"><div class="sheet-p"><div class="grab"></div>
    <a class="item" href="${href('timeline/')}">${ic('timeline')}${T('Timeline', '时间线')}</a>
    <a class="item" href="${href('product/')}">${ic('product')}${T('Product', '产品')}</a>
    <a class="item" href="${esc(askHref)}" target="_blank" rel="noopener">${ic('ask')}${T('Ask a question', '提问')}</a>
    <div class="setrow"><span>${T('Language', '语言')}</span><div class="seg" role="group" aria-label="Language / 语言"><button data-setl="en" lang="en" aria-label="English">EN</button><button data-setl="zh" lang="zh-Hans" aria-label="简体中文">中文</button></div></div>
    <div class="setrow"><span>${T('Appearance', '外观')}</span><div class="seg"><button data-sett="auto">${T('Auto', '自动')}</button><button data-sett="light">${T('Light', '浅色')}</button><button data-sett="dark">${T('Dark', '深色')}</button></div></div>
  </div></div>`;
  const ftr = `<footer class="ftr"><div class="wrap">
    <div class="links">${NAV.map(([, p, en, zh]) => `<a href="${href(p)}">${T(en, zh)}</a>`).join('')}<a href="${href('latest/')}">${T('Latest report', '最新报告')}</a></div>
    <div>${REAL ? T('Generated from <span class="mono">reports/</span>, <span class="mono">SPEC.md</span> and <span class="mono">plan/</span> at commit <span class="mono">09f1d2f</span> · built 24 Nov 2026', '由 <span class="mono">reports/</span>、<span class="mono">SPEC.md</span> 与 <span class="mono">plan/</span> 在提交 <span class="mono">09f1d2f</span> 生成 · 构建于 2026 年 11 月 24 日') : T('Sample data: commit <span class="mono">37017fb</span> · built 3 Dec 2026, 14:05 are made up', '示例数据：提交 <span class="mono">37017fb</span> · 构建于 2026 年 12 月 3 日 14:05 均为虚构')}</div>
    <div>${T('This site is public and read-only. Request buttons open a pre-filled GitHub issue; nothing is sent until you press Submit there.', '本站公开且只读。请求按钮会打开预填好的 GitHub 议题，只有在那里点击提交后才会发送。')}</div>
    <div class="row" style="gap:10px"><span class="sample">${T('Design mockup', '设计原型')}</span>${REAL ? `<a href="?state=sample">${T('Preview the populated state (sample data)', '预览填充后的状态（示例数据）')}</a>` : `<a href="?state=real">${T('Back to the real state', '回到真实状态')}</a>`}</div></div></footer>`;
  B.insertAdjacentHTML('afterbegin', hdr);
  B.insertAdjacentHTML('beforeend', ftr + tab);
  const sheet = $('#sheet');
  $('#moreBtn').onclick = () => { sheet.classList.add('open'); $('.item', sheet).focus(); };
  sheet.onclick = (e) => { if (e.target === sheet) sheet.classList.remove('open'); };
  document.addEventListener('click', (e) => {
    const l = e.target.closest('[data-setl]'); if (l) setL(l.dataset.setl);
    const t = e.target.closest('[data-sett]'); if (t) setT(t.dataset.sett);
    if (e.target.closest('#themeBtn')) setT({ auto: 'light', light: 'dark', dark: 'auto' }[H.dataset.tp]);
  });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') sheet.classList.remove('open'); });
  setL(H.dataset.l); setT(H.dataset.tp);

  /* request links */
  $$('[data-issue-title]').forEach((a) => { a.href = issue(a.dataset.issueTitle, (a.dataset.issueBody || '').replace(/\\n/g, '\n')); a.target = '_blank'; a.rel = 'noopener'; });

  /* ---------- charts ---------- */
  const legend = () => `<div class="legend">${SEGS.map(([s, c]) => `<span><i style="background:var(--${c})"></i>${T(...SL[s])} <b class="num">${C[s]}</b></span>`).join('')}</div>`;
  const ringEl = $('#ring');
  if (ringEl) {
    let off = 0;
    const segs = SEGS.slice(0, 4).map(([s, c], i) => { const len = (C[s] / ACTIVE) * 100; const el = `<circle class="seg-c" cx="50" cy="50" r="45" pathLength="100" stroke="var(--${c})" stroke-width="7" style="--i:${i};--da:${Math.max(len - 1.2, 0)} 100;stroke-dashoffset:${-off};stroke-linecap:butt"><title>${SL[s][0]} ${C[s]}</title></circle>`; off += len; return el; }).join('');
    const pct = Math.round((C.done / ACTIVE) * 100);
    ringEl.innerHTML = `<div class="ring-wrap"><svg class="ring" viewBox="0 0 100 100" role="img" aria-label="${pct}% of requirements done"><circle class="trk" cx="50" cy="50" r="45" stroke-width="7"/>${segs}</svg><div class="ring-c"><div class="big num"><span data-count="${pct}">${pct}</span><small>%</small></div><div class="cap">${T('of requirements done', '的需求已完成')}</div></div></div>${legend()}`;
  }
  const kp = { done: C.done, active: C.doing + C.review, total: TOTAL };
  $$('[data-kpi]').forEach((el) => { const k = el.dataset.kpi; if (kp[k] != null) el.innerHTML = `<span data-count="${kp[k]}">${kp[k]}</span>`; });
  $$('[data-total]').forEach((el) => { el.textContent = TOTAL; });
  $$('[data-tests]').forEach((el) => { if (el.classList.contains('waffle')) { el.dataset.pass = TESTS; return; } el.dataset.count = TESTS; el.textContent = TESTS; });
  const areasEl = $('#areas');
  if (areasEl) {
    areasEl.innerHTML = Object.keys(AREAS).map((a) => { const rs = REQS.filter((r) => r.a === a), c = count(rs), n = rs.length - c.dropped;
      const built = areasEl.dataset.built ? BUILT[a] : 0;
      return `<div class="bar-row"><span class="b-n">${T(...AREAS[a])}</span><span class="b-v">${built ? `<span class="pre-built">${T(`${built} built before the plan`, `计划前已建成 ${built} 项`)}</span> · ` : ''}${T(`${c.done} of ${n} planned done`, `计划内 ${c.done}/${n} 已完成`)}</span><div class="bar" role="img" aria-label="${AREAS[a][0]}: ${c.done}/${n}">${SEGS.map(([s, col], i) => c[s] ? `<i style="--w:${(c[s] / n) * 100}%;--c:var(--${col});--i:${i}"></i>` : '').join('')}</div></div>`; }).join('');
  }
  const burnEl = $('#burn');
  const dayISO = (i) => new Date(Date.parse(BURN_START + 'T12:00:00Z') + i * 864e5).toISOString().slice(0, 10);
  const DAYS = Math.round((Date.parse(BURN_END) - Date.parse(BURN_START)) / 864e5);
  const drawBurn = () => {
    /* the SVG is drawn at the container's pixel width so its 12 px labels stay 12 px on a phone */
    const W = Math.max(280, Math.round(burnEl.clientWidth || 640)), Hh = W < 420 ? 220 : 250, l = 30, r = 10, t = 12, b = 30, days = DAYS, max = 40, x = (d) => l + (d / days) * (W - l - r), y = (v) => t + (1 - v / max) * (Hh - t - b);
    const pts = BURN.map((v, i) => [x(i), y(v)]), d = pts.map((p, i) => (i ? 'L' : 'M') + p[0].toFixed(1) + ' ' + p[1].toFixed(1)).join(' ');
    const ticks = [0, 1, 2, 3].map((k) => { const dd = Math.round((days * k) / 3); return [dd, D(dayISO(dd), { month: 'short', day: 'numeric' })]; });
    const lab = (X, Y, html, a = 'middle') => `<foreignObject x="${a === 'start' ? X : a === 'end' ? X - 80 : X - 40}" y="${Y - 14}" width="80" height="20"><div xmlns="http://www.w3.org/1999/xhtml" style="text-align:${a === 'middle' ? 'center' : a === 'start' ? 'left' : 'right'};font:12px var(--font);color:var(--text-2)">${html}</div></foreignObject>`;
    burnEl.innerHTML = `<svg class="chart" viewBox="0 0 ${W} ${Hh}" role="group" aria-label="Burn-down"><defs><linearGradient id="bdg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--accent-fill)" stop-opacity=".28"/><stop offset="1" stop-color="var(--accent-fill)" stop-opacity="0"/></linearGradient></defs>
      ${[0, 10, 20, 30, 40].map((v) => `<line class="grid-l" x1="${l}" x2="${W - r}" y1="${y(v)}" y2="${y(v)}"/><text x="${l - 8}" y="${y(v) + 4}" text-anchor="end">${v}</text>`).join('')}
      ${ticks.map(([dd, h], k) => lab(x(dd), Hh - 6, h, k === 0 ? 'start' : k === 3 ? 'end' : 'middle')).join('')}
      <line class="ideal" x1="${x(0)}" y1="${y(BURN[0])}" x2="${x(days)}" y2="${y(0)}"/>
      ${BURN.length > 1 ? `<path class="area" d="${d} L${pts.at(-1)[0]} ${y(0)} L${pts[0][0]} ${y(0)}Z"/><path class="line" id="burnLine" d="${d}"/>` : ''}
      ${pts.map((p, i) => `<circle class="pt" cx="${p[0]}" cy="${p[1]}" r="5" tabindex="0" data-i="${i}" role="img" aria-label="${dayISO(i)}: ${BURN[i]}"/>`).join('')}</svg>
      <div class="tip" id="burnTip" aria-live="polite"></div>
      <details class="tbl"><summary>${T('View as table', '以表格查看')}</summary><table><tr><th>${T('Day', '日期')}</th><th>${T('Open', '未完成')}</th></tr>${BURN.map((v, i) => `<tr><td>${D(dayISO(i), { month: 'short', day: 'numeric' })}</td><td>${v}</td></tr>`).join('')}</table></details>`;
    const line = $('#burnLine'); if (line) line.style.setProperty('--len', line.getTotalLength());
    const tip = $('#burnTip'), show = (i) => { tip.innerHTML = `<b>${D(dayISO(i), { month: 'short', day: 'numeric' })}</b> · ${T(BURN[i] + ' requirements still open', '仍有 ' + BURN[i] + ' 项需求未完成')}${BURN.length === 1 ? ' · ' + T('first day of the plan', '计划的第一天') : ''}`; };
    show(BURN.length - 1);
    $$('.pt', burnEl).forEach((p) => { ['pointerenter', 'focus'].forEach((ev) => p.addEventListener(ev, () => show(+p.dataset.i))); });
    if (burnEl.closest('.in')) burnEl.classList.add('in');
  };
  if (burnEl) { drawBurn(); let lw = burnEl.clientWidth; new ResizeObserver(() => { if (Math.abs(burnEl.clientWidth - lw) > 8) { lw = burnEl.clientWidth; drawBurn(); } }).observe(burnEl); }
  $$('.waffle').forEach((wf) => { const pass = +wf.dataset.pass, fail = +wf.dataset.fail || 0, skip = +wf.dataset.skip || 0, n = pass + fail + skip;
    wf.setAttribute('role', 'img'); wf.setAttribute('aria-label', H.dataset.l === 'zh' ? `${pass} 项通过，${fail} 项失败，${skip} 项跳过` : `${pass} tests passed, ${fail} failed, ${skip} skipped`);
    wf.style.gridTemplateColumns = `repeat(auto-fill, minmax(${n > 100 ? 12 : 18}px, 1fr))`; if (n > 100) wf.style.gap = '4px'; wf.style.maxWidth = n > 100 ? 'none' : '';
    wf.innerHTML = Array.from({ length: n }, (_, i) => `<i class="${i < pass ? '' : i < pass + fail ? 'fail' : 'skip'}" style="--n:${i}"></i>`).join(''); });
  const ROLES_REAL = [
    ['M', 'Manager', '经理', '#5e5ce6', 0, 'Idle · waiting for the first cycle. Next: R-001 and R-002 in parallel', '空闲 · 等待第一个周期。下一步：并行启动 R-001 与 R-002', 'No tasks assigned yet', '尚未分配任务'],
    ['B', 'Backend engineer', '后端工程师', '#0a84ff', 0, 'Idle · next up R-060 approval states, then R-020 OTA admin API', '空闲 · 下一项 R-060 审批状态，随后是 R-020 OTA 管理接口', 'No tasks assigned yet', '尚未分配任务'],
    ['F', 'Frontend engineer', '前端工程师', '#30b0c7', 0, 'Idle · starts R-001 report site once the design is delivered', '空闲 · 设计交付后开始 R-001 报告网站', 'No tasks assigned yet', '尚未分配任务'],
    ['D', 'Designer', '设计师', '#bf5af2', 0, 'Idle · next up R-002 design system and screens', '空闲 · 下一项 R-002 设计系统与界面', 'No tasks assigned yet', '尚未分配任务'],
    ['T', 'Tester', '测试工程师', '#34c759', 0, 'Idle · next up R-030 integration tests and R-031 OTA end-to-end script', '空闲 · 下一项 R-030 集成测试与 R-031 OTA 端到端脚本', 'No tasks assigned yet', '尚未分配任务']];
  $$('.roles').forEach((el) => { const RL = REAL ? ROLES_REAL : [
    ['M', 'Manager', '经理', '#5e5ce6', 1, 'Verifying R-020 and drafting the weekly report', '正在验证 R-020 并起草周报', '12 min ago', '12 分钟前'],
    ['B', 'Backend engineer', '后端工程师', '#0a84ff', 1, 'R-020 · OTA admin API: install endpoint and tests', 'R-020 · OTA 管理接口：安装端点与测试', '3 min ago', '3 分钟前'],
    ['F', 'Frontend engineer', '前端工程师', '#30b0c7', 1, 'R-001 · Report site: requirements page and filters', 'R-001 · 报告网站：需求页与筛选', '28 min ago', '28 分钟前'],
    ['D', 'Designer', '设计师', '#bf5af2', 0, 'Idle · last delivered R-001 mockups', '空闲 · 最近交付 R-001 原型', '2 h ago', '2 小时前'],
    ['T', 'Tester', '测试工程师', '#34c759', 1, 'R-030 · account-isolation integration tests', 'R-030 · 账户隔离集成测试', '41 min ago', '41 分钟前']];
    el.innerHTML = RL.map((r, i) => `<article class="card role rv" style="--rd:${i}"><div class="role-h"><span class="avatar" style="--av:${r[3]}" aria-hidden="true">${r[0]}</span><div><h3>${T(r[1], r[2])}</h3><span class="state ${r[4] ? 'on' : ''}"><i></i>${r[4] ? T('Working', '工作中') : T('Idle', '空闲')}</span></div></div><div class="task">${T(r[5], r[6])}</div><div class="when">${T(r[7], r[8])}</div></article>`).join(''); });

  /* requirements page */
  const reqEl = $('#reqs');
  if (reqEl) {
    const rowH = (r) => { const tt = `[CHG ${r.id}] ${r.en}`, dt = `[DEL ${r.id}] ${r.en}`;
      const ch = issue(tt, `**Change request for ${r.id}: ${r.en}**\n\nWhat should change?\n\n\n`), de = issue(dt, `**Remove ${r.id}: ${r.en}**\n\nReason (optional):\n\n`);
      return `<li class="req" data-s="${r.s}" data-a="${r.a}" data-q="${(r.id + ' ' + r.en + ' ' + r.zh).toLowerCase()}"><span class="id">${r.id}</span><span class="t">${T(r.en, r.zh)}${r.why ? `<span class="why">${T(...r.why)}</span>` : ''}</span><span class="pri" data-p="${r.pri}" title="${{ P0: 'must', P1: 'should', P2: 'nice' }[r.pri]}">${r.pri}</span><span class="ch">${chip(r.s)}</span><span class="own">${T(...OWN[r.o])}</span>
      <span class="acts"><a class="btn btn-ghost" href="${esc(ch)}" target="_blank" rel="noopener">${ic('pencil')}${T('Change', '修改')}</a><a class="btn btn-ghost del" href="${esc(de)}" target="_blank" rel="noopener">${ic('xmark')}${T('Remove', '移除')}</a></span></li>`; };
    reqEl.innerHTML = Object.keys(AREAS).map((a) => { const rs = REQS.filter((r) => r.a === a), c = count(rs), n = rs.length - c.dropped;
      return `<section class="area" data-a="${a}"><div class="area-h"><h2>${T(...AREAS[a])}</h2><span class="small">${T(`${c.done} of ${n} done`, `${c.done}/${n} 已完成`)}</span><div class="mini"><i style="--w:${n ? (c.done / n) * 100 : 0}%"></i></div></div><ul class="reqs">${rs.map(rowH).join('')}</ul></section>`; }).join('') + `<div class="empty card" id="noRes" hidden><b>${T('No requirement matches.', '没有匹配的需求。')}</b><p class="sub">${T('Try another status or clear the search.', '换一个状态，或清除搜索。')}</p></div>`;
    const fl = $('#reqFilters'), st = ['all', ...Object.keys(SL)];
    fl.innerHTML = st.map((s) => `<button class="fchip" data-f="${s}" aria-pressed="${s === 'all'}">${s === 'all' ? T('All', '全部') : T(...SL[s])}<span class="n">${s === 'all' ? TOTAL : C[s]}</span></button>`).join('');
    let f = 'all', q = '';
    const apply = () => { let n = 0; $$('.req', reqEl).forEach((li) => { const ok = (f === 'all' || li.dataset.s === f) && (!q || li.dataset.q.includes(q)); li.classList.toggle('hide', !ok); n += ok; });
      $$('.area', reqEl).forEach((s) => { s.hidden = !$('.req:not(.hide)', s); }); $('#noRes').hidden = n > 0; };
    fl.onclick = (e) => { const b = e.target.closest('[data-f]'); if (!b) return; f = b.dataset.f; $$('[data-f]', fl).forEach((x) => x.setAttribute('aria-pressed', x === b)); apply(); };
    $('#reqSearch').oninput = (e) => { q = e.target.value.trim().toLowerCase(); apply(); };
    reqEl.addEventListener('click', (e) => { if (matchMedia('(max-width:860px)').matches && !e.target.closest('a')) e.target.closest('.req')?.classList.toggle('open'); });
    $('#newReq').href = issue('[ADD] ', '**New requirement**\n\nWhat do you want?\n\n\nPriority (P0 must / P1 should / P2 nice): P1\n\nHow will we know it works?\n\n');
    const sm = $('#reqSummary'); if (sm) sm.innerHTML = SEGS.concat([['dropped']]).map(([s]) => `<span class="sc">${chip(s)}<b class="num">${C[s]}</b></span>`).join('');
  }

  /* timeline page */
  const tlEl = $('#tl');
  if (tlEl) {
    let last = '';
    const evH = (e) => { const m = e.d.slice(0, 7), head = m !== last ? (last = m, `<li class="tl-month" aria-hidden="true">${D(e.d, { month: 'long', year: 'numeric' })}</li>`) : '';
      const hash = e.h ? `<a class="hash" href="https://github.com/cia1099/cloud_home/commit/${e.h}" target="_blank" rel="noopener" title="Open commit">${ic('commit')}${e.h}</a>` : '';
      const tag = e.tag ? `<span class="chip neutral">${ic('tag')}${e.tag}</span>` : '', ref = e.ref ? (/^R-/.test(e.ref) ? `<a class="hash" href="${href('requirements/')}">${e.ref}</a>` : /^A-/.test(e.ref) ? `<a class="hash" href="${href('decisions/')}">${e.ref}</a>` : `<a class="hash" href="${root}reports/${e.ref}/">${ic('reports')}${T('Open report', '打开报告')}</a>`) : '';
      return head + `<li class="ev" data-t="${e.t}"><span class="pin">${ic(ET[e.t][2])}</span><div class="ev-card rv"><div class="ev-t">${T(e.en, e.zh)}</div><div class="ev-m"><span>${T(ET[e.t][0], ET[e.t][1])}</span><span aria-hidden="true">·</span><time datetime="${e.d}">${D(e.d, { month: 'short', day: 'numeric' })}</time>${hash}${tag}${ref}</div></div></li>`; };
    const draw = (t) => { last = ''; tlEl.innerHTML = EVENTS.filter((e) => t === 'all' || e.t === t).map(evH).join(''); wireIO(); };
    const tf = $('#tlFilters');
    if (tf) { tf.innerHTML = ['all', ...Object.keys(ET)].map((t) => `<button class="fchip" data-f="${t}" aria-pressed="${t === 'all'}">${t === 'all' ? T('All', '全部') : T(ET[t][0], ET[t][1])}</button>`).join('');
      tf.onclick = (e) => { const b = e.target.closest('[data-f]'); if (!b) return; $$('[data-f]', tf).forEach((x) => x.setAttribute('aria-pressed', x === b)); draw(b.dataset.f); }; }
    draw('all');
  }

  /* ---------- gallery / lightbox / slider / walkthrough player ---------- */
  const shots = $$('.shot[data-lb]');
  if (shots.length) {
    const lb = document.createElement('div'); lb.className = 'lb'; lb.setAttribute('role', 'dialog'); lb.setAttribute('aria-modal', 'true'); lb.setAttribute('aria-label', 'Image viewer'); lb.hidden = true;
    lb.innerHTML = `<div class="lb-bar"><span class="lb-n num" id="lbN"></span><button class="icon-btn" id="lbX" data-al-en="Close" data-al-zh="关闭">${ic('xmark')}</button></div><div class="lb-stage"><button class="icon-btn lb-nav p" data-al-en="Previous image" data-al-zh="上一张">${ic('chevL')}</button><div class="lb-art" id="lbArt"></div><button class="icon-btn lb-nav n" data-al-en="Next image" data-al-zh="下一张">${ic('chev')}</button></div><div class="lb-cap" id="lbCap" aria-live="polite"></div>`;
    B.appendChild(lb); setL(H.dataset.l); let i = 0, opener = null, x0 = null;
    const show = (n) => { i = (n + shots.length) % shots.length; const s = shots[i]; $('#lbArt').innerHTML = s.querySelector('.art').outerHTML; $('#lbCap').innerHTML = s.querySelector('.cap').innerHTML; $('#lbN').textContent = `${i + 1} / ${shots.length}`; };
    const open = (n) => { opener = document.activeElement; show(n); lb.hidden = false; requestAnimationFrame(() => lb.classList.add('open')); B.classList.add('noscroll'); $('main').inert = true; $('#lbX').focus(); };
    const close = () => { lb.classList.remove('open'); B.classList.remove('noscroll'); $('main').inert = false; setTimeout(() => { lb.hidden = true; }, RM ? 0 : 350); opener?.focus(); };
    shots.forEach((s, n) => s.addEventListener('click', () => open(n)));
    $('#lbX').onclick = close; $('.lb-nav.p', lb).onclick = () => show(i - 1); $('.lb-nav.n', lb).onclick = () => show(i + 1);
    lb.onclick = (e) => { if (e.target === lb || e.target.classList.contains('lb-stage')) close(); };
    /* touch: swipe left/right to move, swipe down to close */
    lb.addEventListener('pointerdown', (e) => { if (e.pointerType === 'touch') x0 = [e.clientX, e.clientY]; });
    lb.addEventListener('pointerup', (e) => { if (!x0) return; const dx = e.clientX - x0[0], dy = e.clientY - x0[1]; x0 = null; if (Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy)) show(i + (dx < 0 ? 1 : -1)); else if (dy > 90) close(); });
    document.addEventListener('keydown', (e) => { if (lb.hidden) return; if (e.key === 'Escape') close(); if (e.key === 'ArrowLeft') show(i - 1); if (e.key === 'ArrowRight') show(i + 1);
      if (e.key === 'Tab') { const f = $$('button', lb); const a = f.indexOf(document.activeElement); e.preventDefault(); f[(a + (e.shiftKey ? -1 : 1) + f.length) % f.length].focus(); } });
  }
  /* before / after: the native range input is the control, so arrow keys, Home/End, touch drag and screen readers work */
  $$('.ba').forEach((ba) => { const inp = $('input', ba), set = (v) => { ba.style.setProperty('--p', v + '%'); inp.setAttribute('aria-valuetext', H.dataset.l === 'zh' ? `之前 ${Math.round(100 - v)}%，之后 ${Math.round(v)}%` : `Before ${Math.round(100 - v)}%, after ${Math.round(v)}%`); }; inp.oninput = () => set(+inp.value); set(+inp.value);
    if (RM) return;
    new IntersectionObserver((en, o) => { if (!en[0].isIntersecting) return; o.disconnect(); const t0 = performance.now(); (function f(t) { if (ba.dataset.touched) return; const k = Math.min((t - t0) / 1100, 1), v = 92 - 42 * (1 - Math.pow(1 - k, 3)); set(v); inp.value = v; if (k < 1) requestAnimationFrame(f); })(t0); }, { threshold: .6 }).observe(ba);
    inp.addEventListener('pointerdown', () => { ba.dataset.touched = 1; }); inp.addEventListener('keydown', () => { ba.dataset.touched = 1; }); });
  /* walkthrough player: scenes advance on a timer, every step is also a button; reduced motion = starts paused, no autoplay */
  $$('.player').forEach((pl) => { const sc = $$('.p-scene', pl), st = $$('.p-steps button', pl), btn = $('.p-play', pl), cap = $('.p-cap', pl), tm = $('.p-time', pl), tr = $('.p-track i', pl), N = sc.length, DUR = 3400;
    let i = 0, playing = false, timer = 0, user = false;
    const label = () => { const zh = H.dataset.l === 'zh'; btn.setAttribute('aria-label', playing ? (zh ? '暂停' : 'Pause') : i === N - 1 ? (zh ? '重播' : 'Replay') : (zh ? '播放' : 'Play')); btn.setAttribute('aria-pressed', playing); btn.innerHTML = ic(playing ? 'pause' : i === N - 1 && !playing ? 'refresh' : 'play'); };
    const go = (n) => { i = (n + N) % N; sc.forEach((s, k) => s.classList.toggle('on', k === i)); st.forEach((b, k) => b.setAttribute('aria-current', k === i)); cap.innerHTML = st[i].querySelector('.s-cap').innerHTML; tm.textContent = `${i + 1} / ${N}`; tr.style.animation = 'none'; tr.style.transform = `scaleX(${(i + (RM ? 1 : 0)) / N})`; void tr.offsetWidth;
      if (playing && !RM) { tr.style.transform = ''; tr.style.animation = `pfill ${DUR}ms linear forwards`; tr.style.setProperty('--from', i / N); tr.style.setProperty('--to', (i + 1) / N); } label(); };
    const next = () => { clearTimeout(timer); if (!playing) return; timer = setTimeout(() => { if (i === N - 1) { playing = false; label(); return; } go(i + 1); next(); }, DUR); };
    const play = (fromUser) => { if (fromUser) user = true; if (!playing && i === N - 1) go(0); playing = true; go(i); next(); }, pause = (fromUser) => { if (fromUser) user = true; playing = false; clearTimeout(timer); tr.style.animation = 'none'; tr.style.transform = `scaleX(${(i + 1) / N})`; label(); };
    btn.onclick = () => (playing ? pause(true) : play(true)); st.forEach((b, k) => { b.onclick = () => { user = true; go(k); if (playing) next(); }; });
    pl.addEventListener('keydown', (e) => { if (e.key === 'ArrowRight' && e.target.closest('.p-steps')) { e.preventDefault(); st[(i + 1) % N].focus(); go(i + 1); } if (e.key === 'ArrowLeft' && e.target.closest('.p-steps')) { e.preventDefault(); st[(i - 1 + N) % N].focus(); go(i - 1); } });
    document.addEventListener('langchange', label); go(0); pause(false);
    if (!RM) new IntersectionObserver((en) => { if (en[0].isIntersecting) { if (!user && !playing) play(false); } else if (playing) pause(false); }, { threshold: .6 }).observe(pl);
    document.addEventListener('visibilitychange', () => { if (document.hidden && playing) pause(false); }); });
  /* product page: populated-state preview toggle */
  const pop = $('#popToggle');
  if (pop) { const set = (on) => { B.classList.toggle('pop', on); pop.setAttribute('aria-pressed', on); }; pop.onclick = () => set(!B.classList.contains('pop')); if (new URLSearchParams(location.search).get('pop') === '1') set(true); }

  /* ---------- diagram components (HTML/CSS, data-driven; text stays selectable and translatable) ---------- */
  const ARCH = {
    client: { icon: 'phone', t: ['Browsers and phones', '浏览器与手机'], d: ['Family members on the home network', '家庭网络里的家人'] },
    binary: { name: 'cloud_home', t: ['One program, one version', '一个程序，一个版本'], mods: [
      { icon: 'window', t: ['Web app, embedded', '网页应用（内嵌）'], d: ['Served from the same address as the API', '与接口同一地址提供'], status: 'planned', badge: ['Not built yet', '尚未开发'] },
      { icon: 'server', t: ['API  /api/v1', '接口 /api/v1'], d: ['Accounts, files, trash, sharing, search, live events', '账户、文件、回收站、分享、搜索、实时事件'], status: 'built', badge: ['Built', '已完成'] },
      { icon: 'refresh', t: ['Updater', '更新器'], d: ['Checks, verifies and installs signed releases', '检查、验证并安装带签名的版本'], status: 'built', badge: ['Built', '已完成'] }] },
    drive: { t: ['External drive', '外接硬盘'], d: ['Travels with the data', '数据在哪，数据库就在哪'], mods: [
      { icon: 'db', t: ['SQLite database', 'SQLite 数据库'], d: ['Accounts, folders, shares', '账户、文件夹、分享'], status: 'built' },
      { icon: 'drive', t: ['Family files', '家人的文件'], d: ['One private folder per account', '每个账户一个私有文件夹'], status: 'built' }] },
    cloud: { icon: 'cloud', t: ['GitHub releases', 'GitHub 发布页'], d: ['Signed update packages', '带签名的更新包'] },
    links: [['pages and calls', '页面与请求'], ['reads and writes', '读写'], ['fetches updates', '获取更新']]
  };
  const OTA = [
    { icon: 'cloud', t: ['Check', '检查'], d: ['Looks for a newer release on GitHub', '在 GitHub 上查找新版本'] },
    { icon: 'down', t: ['Download', '下载'], d: ['Fetches the package for this machine', '取回适合本机的安装包'] },
    { icon: 'shield', t: ['Verify', '验证'], d: ['Checks the minisign signature with the built-in key', '用内置公钥检查 minisign 签名'], fail: ['Bad signature: nothing is installed', '签名不符：不会安装任何东西'] },
    { icon: 'check', t: ['Self-test', '自检'], d: ['The new program must pass its own checks first, web app included', '新程序先通过自检，网页应用也在其中'], fail: ['Fails: the update is refused', '未通过：拒绝更新'] },
    { icon: 'swap', t: ['Switch', '切换'], d: ['The current link moves to the new version; the old one stays', '“当前版本”指向新版本，旧版本保留'] },
    { icon: 'power', t: ['Restart', '重启'], d: ['Transfers in progress finish first', '正在进行的传输先完成'] },
    { icon: 'heart', t: ['Health check', '健康检查'], d: ['Must report healthy in time', '必须按时报告健康'], fail: ['Not healthy: rolls back automatically', '不健康：自动回滚'] },
    { icon: 'undo', t: ['Done or rolled back', '完成或回滚'], d: ['Healthy keeps the update. Otherwise the old version and the database copy from before come back', '健康则保留新版本；否则旧版本与更新前的数据库副本自动恢复'] }
  ];
  const mod = (m) => `<div class="mod ${m.status || ''}"><span class="mod-i">${ic(m.icon)}</span><div><b>${T(...m.t)}</b><span>${T(...m.d)}</span></div>${m.badge ? `<span class="mod-b">${T(...m.badge)}</span>` : ''}</div>`;
  $$('[data-diagram=arch]').forEach((el) => { const A = ARCH;
    el.innerHTML = `<div class="arch" role="group" aria-label="Architecture">
      <div class="a-node a-client"><span class="mod-i big">${ic(A.client.icon)}</span><b>${T(...A.client.t)}</b><span>${T(...A.client.d)}</span></div>
      <div class="a-link l1" aria-hidden="true"><span>${T(...A.links[0])}</span><i></i></div>
      <div class="a-node a-bin"><div class="bin-h"><code>${A.binary.name}</code><span>${T(...A.binary.t)}</span></div>${A.binary.mods.map(mod).join('')}</div>
      <div class="a-link l2" aria-hidden="true"><span>${T(...A.links[1])}</span><i></i></div>
      <div class="a-node a-drive"><div class="bin-h"><b>${T(...A.drive.t)}</b><span>${T(...A.drive.d)}</span></div>${A.drive.mods.map(mod).join('')}</div>
      <div class="a-link l3" aria-hidden="true"><span>${T(...A.links[2])}</span><i></i></div>
      <div class="a-node a-cloud"><span class="mod-i big">${ic(A.cloud.icon)}</span><b>${T(...A.cloud.t)}</b><span>${T(...A.cloud.d)}</span></div></div>`; });
  $$('[data-diagram=ota]').forEach((el) => {
    el.innerHTML = `<ol class="flow" style="--n:${OTA.length}">${OTA.map((s, i) => `<li style="--i:${i}"><span class="node">${ic(s.icon)}</span><div><b>${T(...s.t)}</b><span>${T(...s.d)}</span>${s.fail ? `<span class="failtag">${ic('undo')}${T(...s.fail)}</span>` : ''}</div></li>`).join('')}</ol>`; });

  /* ---------- scroll behaviours ---------- */
  function wireIO() {
    const io = new IntersectionObserver((es) => es.forEach((e) => { if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); } }), { threshold: .15, rootMargin: '0px 0px -6% 0px' });
    $$('.rv:not(.in), .io:not(.in)').forEach((el) => (RM ? el.classList.add('in') : io.observe(el)));
    const cio = new IntersectionObserver((es) => es.forEach((e) => { if (!e.isIntersecting) return; cio.unobserve(e.target); const el = e.target, to = +el.dataset.count; if (RM) { el.textContent = to; return; }
      const t0 = performance.now(), fmt = el.dataset.fmt; (function f(t) { const k = Math.min((t - t0) / 1100, 1); el.textContent = Math.round(to * (1 - Math.pow(1 - k, 4))); if (k < 1) requestAnimationFrame(f); })(t0); }), { threshold: .5 });
    $$('[data-count]').forEach((el) => { if (!RM) el.textContent = '0'; cio.observe(el); });
  }
  wireIO();
  const spy = (links, targets, cls = 'on') => { if (!links.length) return; const m = new Map(links.map((a) => [a.getAttribute('href').slice(1), a]));
    const io = new IntersectionObserver((es) => es.forEach((e) => { if (e.isIntersecting) { links.forEach((a) => a.classList.remove(cls)); m.get(e.target.id)?.classList.add(cls); } }), { rootMargin: '-30% 0px -60% 0px' }); targets.forEach((t) => io.observe(t)); };
  spy($$('.toc a'), $$('.rsec[id]')); spy($$('.ms-nav a'), $$('.ms-sec[id]'));
  const steps = $$('.wstep');
  if (steps.length) { const io = new IntersectionObserver((es) => es.forEach((e) => { if (e.isIntersecting) { steps.forEach((s) => s.classList.toggle('on', s === e.target)); $$('.screen').forEach((s) => s.classList.toggle('on', s.dataset.i === e.target.dataset.i)); } }), { rootMargin: '-45% 0px -45% 0px' }); steps.forEach((s) => io.observe(s)); steps[0].classList.add('on'); }
  const dev = $('.ms-hero .device');
  if (dev && !RM) { const f = () => { const k = Math.min(scrollY / 420, 1); dev.style.setProperty('--rx', (8 * (1 - k)).toFixed(2) + 'deg'); dev.style.setProperty('--ty', (-24 * k).toFixed(1) + 'px'); }; addEventListener('scroll', f, { passive: true }); f(); }
})();
