/* Cloud Home app mockup. Everything here is sample data and a hand-rolled renderer so the design can be clicked through
   without a build step or network. It is NOT the implementation: ui/ is React. Names of states, routes and strings are the contract. */
(function () {
  'use strict';
  var stage = document.getElementById('stage'), wrap = document.getElementById('stage-wrap');
  var LANGS = [['en', 'English', 'en'], ['zh-Hans', '简体中文', 'zh-Hans-CN'], ['zh-Hant', '繁體中文', 'zh-Hant-TW']];
  var store = { get: function (k) { try { return localStorage.getItem('chapp.' + k); } catch (e) { return null; } }, set: function (k, v) { try { localStorage.setItem('chapp.' + k, v); } catch (e) {} } };
  var reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  var set = { lang: document.documentElement.lang, theme: store.get('theme') || (new URLSearchParams(location.search).get('theme')) || 'auto', dev: new URLSearchParams(location.search).get('dev') || store.get('dev') || 'auto' };
  var li = function () { return Math.max(0, LANGS.map(function (l) { return l[0]; }).indexOf(set.lang)); };
  var loc = function () { return LANGS[li()][2]; };
  function t(k, v) { var e = window.S[k]; var s = e ? e[li()] : k; if (v) for (var n in v) s = s.split('{' + n + '}').join(v[n]); return s; }
  function esc(s) { return String(s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); }
  function I(n, s, c) { return window.ICON(n, s, c); }
  function nf(n, o) { return new Intl.NumberFormat(loc(), o).format(n); }
  function size(b) { var u = ['B', 'KB', 'MB', 'GB', 'TB'], i = 0; while (b >= 1000 && i < 4) { b /= 1000; i++; } return nf(b, { maximumFractionDigits: b < 10 && i > 0 ? 1 : 0 }) + ' ' + u[i]; }
  var NOW = new Date(2026, 10, 25, 10, 30);
  function dAgo(d) { return new Date(NOW.getTime() - d * 864e5); }
  function dfmt(d) { if (d <= 1) return new Intl.RelativeTimeFormat(loc(), { numeric: 'auto' }).format(-d, 'day'); return new Intl.DateTimeFormat(loc(), { dateStyle: 'medium' }).format(dAgo(d)); }
  function dlong(d) { return new Intl.DateTimeFormat(loc(), { dateStyle: 'medium' }).format(new Date(NOW.getTime() + d * 864e5)); }
  var MB = 1e6, GB = 1e9;

  /* ---------- sample data ---------- */
  var F = {
    1: { t: 'folder', n: 'Family Photos', m: 2, s: 3.4 * GB, c: 6 }, 2: { t: 'folder', n: 'Music', m: 9, s: 1.1 * GB, c: 1 }, 3: { t: 'folder', n: 'Documents', m: 1, s: 86 * MB, c: 4 },
    4: { t: 'folder', n: '2026 暑假旅行', m: 30, s: 0, c: 0 }, 17: { t: 'folder', n: 'Kyoto 2026', m: 3, s: 9 * MB, c: 2 },
    5: { t: 'image', n: 'Kyoto autumn.jpg', m: 3, s: 4.2 * MB, a: 0 }, 6: { t: 'image', n: 'IMG_4821.jpg', m: 0, s: 3.1 * MB, a: 1 }, 7: { t: 'image', n: 'Grandma’s birthday.png', m: 12, s: 6.8 * MB, a: 3 },
    8: { t: 'audio', n: 'Spring Rain.mp3', m: 20, s: 7.4 * MB, d: '3:42' }, 9: { t: 'video', n: 'Beach day.mp4', m: 40, s: 248 * MB, a: 5, d: '2:18' },
    10: { t: 'pdf', n: 'Tax summary 2026.pdf', m: 1, s: 1.2 * MB }, 11: { t: 'doc', n: 'Recipes.docx', m: 60, s: 240e3 }, 12: { t: 'sheet', n: 'Household budget.xlsx', m: 5, s: 88e3 },
    13: { t: 'zip', n: 'Backup 2025.zip', m: 200, s: 1.8 * GB },
    14: { t: 'pdf', n: 'Annual family travel plan 2027, itinerary, bookings and insurance documents (final, reviewed by everyone).pdf', m: 7, s: 5.5 * MB },
    15: { t: 'image', n: 'Lake sunrise.jpg', m: 14, s: 5.3 * MB, a: 4 }, 16: { t: 'image', n: 'Pet portraits.jpg', m: 18, s: 2.9 * MB, a: 2 }
  };
  var CH = { 0: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], 1: [17, 5, 6, 7, 15, 16], 17: [5, 15], 2: [8], 3: [10, 11, 12, 14], 4: [] };
  var PAR = { 1: 0, 2: 0, 3: 0, 4: 0, 17: 1 };
  var GAL = [5, 6, 7, 15, 16];
  var KIND_ICON = { folder: 'folder', image: 'image', audio: 'music', video: 'video', pdf: 'pdf', doc: 'doc', sheet: 'sheet', zip: 'zip', file: 'file' };
  var USER = { name: 'Mei Chen', initial: 'M', admin: true };
  var TRASH0 = [{ id: 't1', t: 'image', n: 'Old screenshot.png', f: 'Family Photos', left: 7, s: 800e3, a: 2 }, { id: 't2', t: 'doc', n: 'Draft letter.docx', f: 'Documents', left: 5, s: 52e3 }, { id: 't3', t: 'folder', n: 'Unused exports', f: 'Files', left: 3, s: 2.4 * GB }, { id: 't4', t: 'video', n: 'Clip 0042.mov', f: 'Files', left: 1, s: 410 * MB, a: 1 }, { id: 't5', t: 'audio', n: 'Voice memo.m4a', f: 'Music', left: 0, s: 3.2 * MB }];
  var SHARES0 = [{ id: 's1', t: 'image', n: 'Kyoto autumn.jpg', a: 0, exp: 6, dl: true, hits: 14 }, { id: 's2', t: 'folder', n: 'Family Photos', exp: null, dl: true, hits: 3 }, { id: 's3', t: 'pdf', n: 'Tax summary 2026.pdf', exp: -2, dl: false, hits: 1 }];

  /* ---------- state ---------- */
  var ST = { view: store.get('view') || 'grid', sort: { k: 'name', dir: 1 }, sel: [], menu: null, dlg: null, toast: null, up: null, drag: false, form: {}, pw: false, trash: null, shares: null, upd: { phase: 'idle', pct: 0 }, pv: { playing: false, pos: 0 }, q: '' };
  var R = null, timers = {};

  function parse() {
    var h = location.hash.replace(/^#\/?/, ''), qi = h.indexOf('?'), q = {};
    if (qi >= 0) { new URLSearchParams(h.slice(qi + 1)).forEach(function (v, k) { q[k] = v; }); h = h.slice(0, qi); }
    var p = h.split('/').filter(Boolean);
    return { name: p[0] || 'files', parts: p.slice(1), q: q };
  }
  function H(path, q) { var s = '#/' + path; if (q) { var a = []; for (var k in q) a.push(k + '=' + encodeURIComponent(q[k])); s += '?' + a.join('&'); } return s; }

  function initState() {
    var q = R.q, s = q.state || '';
    if (R.name === 'files' && 'state' in q) { ST.sel = []; ST.menu = null; ST.dlg = null; ST.up = null; ST.drag = false; ST.q = ''; ST.toast = null; }
    if (R.name === 'trash' && !ST.trash) ST.trash = TRASH0.slice();
    if (R.name === 'shared' && !ST.shares) ST.shares = SHARES0.slice();
    if (!ST.trash) ST.trash = TRASH0.slice();
    if (!ST.shares) ST.shares = SHARES0.slice();
    if (R.name === 'files') {
      if (q.view) ST.view = q.view;
      if (s === 'selection') ST.sel = [1, 6, 9];
      if (s === 'download') { ST.sel = [1, 6, 9]; ST.dlg = { type: 'download', fmt: 'zip', phase: 0 }; }
      if (s === 'share') { ST.sel = [5]; ST.dlg = { type: 'share', phase: 0, exp: '7d', dl: true, id: 5 }; }
      if (s === 'newfolder') ST.dlg = { type: 'newfolder' };
      if (s === 'offline') ST.up = mkUp('run');
      if (s === 'menu') ST.menu = { type: 'file', id: 6, x: 360, y: 260 };
      if (s === 'search') { ST.q = q.q || 'kyoto'; }
      if (s === 'upload') ST.up = mkUp('run');
      if (s === 'upload-fail') ST.up = mkUp('fail');
      if (s === 'upload-done') ST.up = mkUp('done');
      if (s === 'upload-min') { ST.up = mkUp('run'); ST.up.min = true; }
    }
    if (R.name === 'settings' && s === 'otp') ST.dlg = { type: 'otp', phase: 0, which: 'phone' };
    if (R.name === 'settings' && s === 'password') ST.dlg = { type: 'password' };
    if (R.name === 'admin') { ST.upd = { phase: q.u || 'idle', pct: q.u === 'downloading' ? 62 : 0 }; if (q.u === 'busy') ST.upd.phase = 'available'; if (q.u === 'busy') ST.dlg = { type: 'busy' }; }
    if (R.name === 'preview') { ST.pv = { playing: false, pos: 0 }; if (s === 'playing') { ST.pv.playing = true; ST.pv.pos = 64; } }
    if (q.banner === 'updating') startRetry();
  }
  function mkUp(k) {
    var a = [{ n: 'Kyoto autumn.jpg', s: 4.2 * MB, p: 100, st: 'done' }, { n: 'IMG_4821.jpg', s: 3.1 * MB, p: 100, st: 'done', as: 'IMG_4821(1).jpg' }, { n: 'Beach day.mp4', s: 248 * MB, p: 46, st: 'up', v: 1.2 }, { n: 'Spring Rain.mp3', s: 7.4 * MB, p: 78, st: 'up', v: 2 }, { n: 'Backup 2025.zip', s: 1.8 * GB, p: 0, st: 'wait' }];
    if (k === 'fail') { a[2].st = 'err'; a[2].p = 46; a[3].st = 'done'; a[3].p = 100; a[4].st = 'cancel'; }
    if (k === 'done') a.forEach(function (x) { x.st = 'done'; x.p = 100; });
    return { items: a, min: false, folder: 'Family Photos' };
  }
  function freshUp() {
    return { min: false, folder: 'Family Photos', items: [{ n: 'IMG_4821.jpg', s: 3.1 * MB, p: 100, st: 'done', as: 'IMG_4821(1).jpg' }, { n: 'Beach day.mp4', s: 248 * MB, p: 3, st: 'up', v: 3 }, { n: 'Spring Rain.mp3', s: 7.4 * MB, p: 0, st: 'up', v: 2 }, { n: 'Backup 2025.zip', s: 1.8 * GB, p: 0, st: 'wait' }] };
  }
  function startRetry() { clearInterval(timers.retry); ST.retry = 12; timers.retry = setInterval(function () { ST.retry = ST.retry > 1 ? ST.retry - 1 : 12; var e = document.getElementById('retry-n'); if (e) e.textContent = ST.retry; }, 1000); }

  /* ---------- building blocks ---------- */
  function folderSVG() { return '<svg class="foldericon" viewBox="0 0 132 100" aria-hidden="true"><path d="M8 20a10 10 0 0 1 10-10h28l12 12h56a10 10 0 0 1 10 10v8H8z" style="fill:var(--folder)" opacity=".65"/><rect x="8" y="30" width="116" height="62" rx="11" fill="url(#fg1)"/><path d="M8.5 42a10 10 0 0 1 10-11.5h95a10 10 0 0 1 10 11.5" fill="none" stroke="#fff" stroke-opacity=".4"/></svg>'; }
  function thumb(f) {
    if (f.t === 'folder') return '<div class="thumb folder">' + folderSVG() + '</div>';
    if (f.t === 'image') return '<div class="thumb">' + ART(f.a) + '</div>';
    if (f.t === 'video') return '<div class="thumb">' + ART(f.a) + '<span class="playbadge">' + I('play', 14) + '</span></div>';
    return '<div class="thumb"><div class="fileglyph kind-' + f.t + '">' + I(KIND_ICON[f.t], 28) + '</div></div>';
  }
  function mini(f, cls) {
    if (f.t === 'folder') return '<span class="' + cls + ' f">' + I('folder', 26) + '</span>';
    if (f.t === 'image' || f.t === 'video') return '<span class="' + cls + '">' + ART(f.a) + '</span>';
    return '<span class="' + cls + ' kind-' + f.t + '">' + I(KIND_ICON[f.t], 18) + '</span>';
  }
  function meta(f) { return f.t === 'folder' ? t('items_n', { n: f.c }) : size(f.s); }
  function kindName(f) { return f.t === 'folder' ? t('folder') : f.n.split('.').pop().toUpperCase(); }
  function sorted(ids) {
    var k = ST.sort.k, d = ST.sort.dir;
    return ids.slice().sort(function (a, b) {
      var A = F[a], B = F[b]; if ((A.t === 'folder') !== (B.t === 'folder')) return A.t === 'folder' ? -1 : 1;
      var r = k === 'name' ? A.n.localeCompare(B.n, loc(), { numeric: true }) : k === 'size' ? A.s - B.s : B.m - A.m;
      return r * d;
    });
  }
  function nav(key, ico, label, href, cur, badge) {
    return '<a class="navi" href="' + href + '"' + (cur ? ' aria-current="page"' : '') + ' title="' + esc(label) + '">' + I(ico, 22) + '<span class="lbl">' + esc(label) + '</span><span class="lbl2" aria-hidden="true">' + esc(label) + '</span>' + (badge ? '<span class="badge" aria-label="' + badge + '">' + badge + '</span>' : '') + '</a>';
  }
  var USED = 428 * GB, TOTAL = 1000 * GB;
  var KINDS = [['photos', 212, 'photo'], ['videos', 118, 'video'], ['audio', 22, 'audio'], ['documents', 41, 'doc'], ['other', 35, 'other']];
  function meterHTML(big) { return '<div class="meter' + (big ? ' big' : '') + '" role="img" aria-label="' + esc(t('storage_used', { used: size(USED), total: size(TOTAL) })) + '">' + KINDS.map(function (k) { return '<i style="width:' + (k[1] * GB / TOTAL * 100) + '%;background:var(--k-' + k[2] + ')"></i>'; }).join('') + '</div>'; }
  function shell(active, bar, content, extra) {
    var pend = USER.admin ? 2 : 0;
    var side = '<aside class="side" aria-label="' + t('nav.sidebar') + '"><div class="brand"><span class="logo">' + I('cloud', 18) + '</span><span>' + t('app.name') + '</span></div><nav aria-label="' + t('nav.main') + '" style="display:grid;gap:2px">' +
      nav('files', 'folder', t('nav.files'), H('files', { state: 'populated' }), active === 'files') + nav('shared', 'share', t('nav.shared'), H('shared'), active === 'shared') + nav('trash', 'trash', t('nav.trash'), H('trash'), active === 'trash') +
      nav('storage', 'drive', t('nav.storage'), H('storage'), active === 'storage') + nav('settings', 'sliders', t('nav.settings'), H('settings'), active === 'settings') +
      (USER.admin ? nav('admin', 'shield', t('nav.admin'), H('admin'), active === 'admin', pend) : '') + '</nav><div class="side-sp"></div>' +
      '<a class="side-card" href="' + H('storage') + '">' + meterHTML() + '<span class="small num">' + t('storage_used', { used: size(USED), total: size(TOTAL) }) + '</span></a>' +
      '<button class="me" data-act="me" aria-haspopup="menu"><span class="avatar" aria-hidden="true">' + USER.initial + '</span><span class="who trunc" style="font-weight:500">' + USER.name + '</span></button></aside>';
    var tabs = '<nav class="tabs" aria-label="' + t('nav.main') + '">' + tab('folder', t('nav.files'), H('files', { state: 'populated' }), active === 'files') + tab('share', t('nav.shared'), H('shared'), active === 'shared') + tab('trash', t('nav.trash'), H('trash'), active === 'trash') +
      tab('sliders', t('nav.settings'), H('settings'), ['settings', 'storage', 'admin'].indexOf(active) >= 0, pend) + '</nav>';
    var strip = '';
    if (R.q.state === 'offline') strip = '<div class="offline-strip" role="status">' + I('wifioff', 16) + t('offline_short') + '</div>';
    if (R.q.banner === 'reconnect') strip = '<div class="offline-strip" role="status" style="background:var(--accent-tint);color:var(--accent)">' + I('refresh', 16, 'spin') + t('sse_reconnecting') + '</div>';
    return '<div class="app"><a class="skip" href="#/files?state=populated" data-act="skip">' + t('a11y.skip') + '</a>' + side + '<main class="main" id="main">' + bar + strip + '<div class="content" id="content" tabindex="-1">' + content + '</div>' + (extra || '') + '</main>' + tabs + '</div>';
  }
  function tab(ico, label, href, cur, badge) { return '<a href="' + href + '"' + (cur ? ' aria-current="page"' : '') + '>' + I(ico, 24) + '<span>' + esc(label) + '</span>' + (badge ? '<span class="badge" aria-label="' + badge + '">' + badge + '</span>' : '') + '</a>'; }
  function titleBar(title, right) { return '<header class="bar"><div class="title-bar trunc" aria-hidden="true">' + esc(title) + '</div>' + (right || '') + '</header>'; }
  function seg(items, cur, act, cls) { return '<div class="seg ' + (cls || '') + '" role="group">' + items.map(function (i) { return '<button data-act="' + act + '" data-v="' + i[0] + '" aria-pressed="' + (cur === i[0]) + '">' + (i[2] ? I(i[2], 16) : '') + (i[1] ? esc(i[1]) : '') + '</button>'; }).join('') + '</div>'; }
  function stateBlock(ico, tone, title, text, btn) { return '<div class="state" role="status"><div class="big ' + (tone || '') + '">' + I(ico, 44) + '</div><h2>' + esc(title) + '</h2><p>' + esc(text) + '</p>' + (btn || '') + '</div>'; }
  function langSeg() { return '<div class="langseg"><div class="seg" role="group" aria-label="' + t('language') + '">' + LANGS.map(function (l) { return '<button data-act="lang" data-v="' + l[0] + '" lang="' + l[0] + '" aria-pressed="' + (set.lang === l[0]) + '">' + l[1] + '</button>'; }).join('') + '</div></div>'; }

  /* ---------- auth ---------- */
  function authPage(k) {
    var f = ST.form, err = R.q.state === 'error';
    var logo = '<span class="logo">' + I('cloud', 34) + '</span>', body;
    var back = '<a class="btn plain block lg" href="' + H('signin') + '">' + t('approval.back_to_sign_in') + '</a>';
    function status(tone, ico, title, text, extra) { return '<div class="statusmark ' + tone + (tone === 'warn' ? ' pulse' : '') + '">' + I(ico, 36) + '</div><h1>' + esc(title) + '</h1><p class="sub">' + esc(text) + '</p>' + (extra || '') + back; }
    if (k === 'signin') {
      body = logo + '<h1>' + t('sign_in_title') + '</h1><p class="sub">' + t('sign_in_sub') + '</p><form data-form="signin" novalidate>' +
        '<div class="field"><label for="id">' + t('email_or_phone') + '</label><input class="input" id="id" name="id" autocomplete="username" inputmode="email" autocapitalize="off" value="' + esc(f.id || '') + '"' + (err ? ' aria-invalid="true" aria-describedby="e1"' : '') + '></div>' +
        '<div class="field"><label for="pw">' + t('password') + '</label><div class="pw"><input class="input" id="pw" name="pw" type="' + (ST.pw ? 'text' : 'password') + '" autocomplete="current-password"' + (err ? ' aria-invalid="true" aria-describedby="e1"' : '') + ' value="' + esc(f.pw || '') + '"><button type="button" class="ibtn" data-act="pw" aria-label="' + (ST.pw ? t('hide_password') : t('show_password')) + '">' + I(ST.pw ? 'lock' : 'key', 20) + '</button></div>' +
        (err ? '<p class="hint err" id="e1" role="alert">' + I('alert', 16) + t('wrong_credentials') + '</p>' : '') + '</div>' +
        '<button class="btn primary lg block" type="submit">' + t('sign_in') + '</button></form><p class="auth-foot">' + t('no_account') + ' <a href="' + H('register') + '">' + t('register') + '</a></p>';
    } else if (k === 'register') {
      body = logo + '<h1>' + t('register_title') + '</h1><p class="sub">' + t('register_sub') + '</p><form data-form="register" novalidate>' +
        '<div class="field"><label for="dn">' + t('display_name') + '</label><input class="input" id="dn" autocomplete="name" value="' + esc(f.dn || '') + '"></div>' +
        '<div class="field"><label for="id">' + t('email_or_phone') + '</label><input class="input" id="id" autocomplete="username" inputmode="email" value="' + esc(f.id || '') + '"></div>' +
        '<div class="field"><label for="pw">' + t('password') + '</label><input class="input" id="pw" type="password" autocomplete="new-password" aria-describedby="ph"><span class="hint" id="ph">' + t('password_hint') + '</span></div>' +
        '<button class="btn primary lg block" type="submit">' + t('register') + '</button></form><p class="auth-foot">' + t('have_account') + ' <a href="' + H('signin') + '">' + t('sign_in') + '</a></p>';
    } else if (k === 'pending') {
      body = status('acc', 'mail', t('approval.submitted_title'), t('approval.submitted_body'), '<div class="steps" role="list"><div class="s done" role="listitem"><span class="n">' + I('check', 16) + '</span>' + t('approval.step1') + '</div><div class="s now" role="listitem"><span class="n">' + I('clock', 16) + '</span>' + t('approval.step2') + '<span class="chip warn" style="margin-inline-start:auto">' + t('approval.waiting') + '</span></div><div class="s" role="listitem"><span class="n">3</span>' + t('approval.step3') + '</div></div>');
    } else if (k === 'account-pending') body = status('warn', 'clock', t('approval.pending_title'), t('approval.pending_body'));
    else if (k === 'rejected') body = status('err', 'lock', t('approval.rejected_title'), t('approval.rejected_body'));
    else if (k === 'closed') body = status('acc', 'lock', t('approval.closed_title'), t('approval.closed_body'));
    else body = status('warn', 'clock', t('approval.busy_title'), t('approval.busy_body'));
    return '<div class="auth">' + langSeg() + '<div class="auth-card" role="main">' + body + '</div></div>';
  }

  /* ---------- files ---------- */
  function crumbsFor(fid) {
    var path = [], c = fid; while (c) { path.unshift(c); c = PAR[c]; }
    var h = '<nav class="crumbs" aria-label="Breadcrumb">';
    if (!path.length) return h + '<span class="cur" aria-current="page">' + t('nav.files') + '</span></nav>';
    var back = fid ? H('files/' + (PAR[fid] || ''), { state: 'populated' }) : '';
    h += '<a class="hide-wide" href="' + back + '" aria-label="' + t('nav.files') + '" style="padding:4px 2px 4px 0;display:inline-flex;align-items:center">' + I('chevL', 22) + '</a>';
    h += '<a class="hide-phone" href="' + H('files', { state: 'populated' }) + '">' + t('nav.files') + '</a>';
    path.forEach(function (id, i) { var last = i === path.length - 1; h += '<span class="hide-phone">' + I('chevR', 16) + '</span>' + (last ? '<span class="cur" aria-current="page">' + esc(F[id].n) + '</span>' : '<a class="hide-phone" href="' + H('files/' + id, { state: 'populated' }) + '">' + esc(F[id].n) + '</a>'); });
    return h + '</nav>';
  }
  function filesBar(fid, offline) {
    if (ST.sel.length) {
      var one = ST.sel.length === 1, dis = offline ? ' disabled' : '';
      var acts = '<button class="abtn" data-act="dlg" data-v="download">' + I('download', 18) + t('download') + '</button><button class="abtn" data-act="dlg" data-v="share"' + (one ? '' : ' disabled') + '>' + I('share', 18) + t('share') + '</button><button class="abtn" data-act="noop"' + dis + '>' + I('move', 18) + t('move') + '</button>' + (one ? '<button class="abtn" data-act="noop">' + I('edit', 18) + t('rename') + '</button>' : '') + '<button class="abtn dng" data-act="trash">' + I('trash', 18) + t('delete') + '</button>';
      return '<header class="bar sel selbar-top"><div class="selbar"><button class="ibtn acc" data-act="desel" aria-label="' + t('deselect') + '">' + I('x', 20) + '</button><span class="count num" role="status">' + t('n_selected', { n: ST.sel.length }) + '</span><span class="grow"></span>' + acts + '</div></header>' +
        '<div class="selbar-bottom" role="toolbar">' + acts.replace(/class="abtn/g, 'class="abtn') + '</div>';
    }
    var searchEl = '<div class="search hide-phone" role="search">' + I('search', 16) + '<input type="search" data-search aria-label="' + t('search_placeholder') + '" placeholder="' + t('search_placeholder') + '" value="' + esc(ST.q) + '">' + (ST.q ? '<button class="x" data-act="clearq" aria-label="' + t('clear') + '">' + I('x', 12) + '</button>' : '') + '</div>';
    return '<header class="bar">' + crumbsFor(fid) + '<span class="grow hide-phone"></span>' + searchEl +
      '<div class="seg hide-phone" role="group" aria-label="' + t('view_as') + '"><button data-act="view" data-v="grid" aria-pressed="' + (ST.view === 'grid') + '" aria-label="' + t('view_grid') + '">' + I('grid', 17) + '</button><button data-act="view" data-v="list" aria-pressed="' + (ST.view === 'list') + '" aria-label="' + t('view_list') + '">' + I('list', 17) + '</button></div>' +
      '<button class="ibtn hide-phone" data-act="sortmenu" aria-label="' + t('sort_by') + '" aria-haspopup="menu">' + I('sort', 20) + '</button>' +
      '<button class="ibtn hide-wide" data-act="view" data-v="' + (ST.view === 'grid' ? 'list' : 'grid') + '" aria-label="' + (ST.view === 'grid' ? t('view_list') : t('view_grid')) + '">' + I(ST.view === 'grid' ? 'list' : 'grid', 20) + '</button>' +
      '<button class="btn primary hide-phone" data-act="uploadmenu" aria-haspopup="menu"' + (offline ? ' disabled' : '') + '>' + I('upload', 16) + t('upload') + '</button>' +
      '<button class="ibtn acc hide-wide" data-act="uploadmenu" aria-label="' + t('upload') + '" aria-haspopup="menu">' + I('plus', 24) + '</button></header>';
  }
  function itemsHTML(ids) {
    var sel = ST.sel, selecting = sel.length ? ' selecting' : '';
    if (ST.view === 'grid') {
      return '<div class="grid' + selecting + '" role="listbox" aria-multiselectable="true" aria-label="' + t('nav.files') + '">' + ids.map(function (id) {
        var f = F[id], on = sel.indexOf(id) >= 0;
        return '<div class="tile' + (R.q.state === 'drop' && id === 1 ? ' droptarget' : '') + '" role="option" tabindex="0" aria-selected="' + on + '" data-id="' + id + '" data-act="item"><button class="tick" tabindex="-1" data-act="tick" aria-label="' + t('a11y.select') + ' ' + esc(f.n) + '">' + I('check', 14) + '</button>' + thumb(f) + '<div class="nm" title="' + esc(f.n) + '">' + esc(f.n) + '</div><div class="mt num">' + meta(f) + '</div></div>';
      }).join('') + '</div>';
    }
    var arrow = function (k) { return ST.sort.k === k ? '<span style="display:inline-flex;' + (ST.sort.dir > 0 ? '' : 'transform:rotate(180deg)') + '">' + I('chevD', 12) + '</span>' : ''; };
    var head = '<div class="lhead" role="row"><span></span><button data-act="sort" data-v="name" aria-sort="' + (ST.sort.k === 'name' ? (ST.sort.dir > 0 ? 'ascending' : 'descending') : 'none') + '">' + t('col.name') + arrow('name') + '</button><button class="c-mod" data-act="sort" data-v="mod">' + t('col.modified') + arrow('mod') + '</button><button class="c-size" data-act="sort" data-v="size">' + t('col.size') + arrow('size') + '</button><span></span></div>';
    return head + '<div role="listbox" aria-multiselectable="true" aria-label="' + t('nav.files') + '" style="padding-bottom:120px">' + ids.map(function (id) {
      var f = F[id], on = sel.indexOf(id) >= 0;
      return '<div class="lrow" role="option" tabindex="0" aria-selected="' + on + '" data-id="' + id + '" data-act="item"><button class="cb" tabindex="-1" data-act="tick" aria-label="' + t('a11y.select') + ' ' + esc(f.n) + '">' + I('check', 13) + '</button><div class="nm">' + mini(f, 'ico') + '<span class="rn"><span class="trunc" style="display:block" title="' + esc(f.n) + '">' + esc(f.n) + '</span><span class="sub num">' + dfmt(f.m) + ' · ' + meta(f) + '</span></span></div><span class="meta c-mod num">' + dfmt(f.m) + '</span><span class="meta c-size num">' + meta(f) + '</span><button class="ibtn" data-act="rowmenu" data-id="' + id + '" aria-label="' + t('a11y.more') + ': ' + esc(f.n) + '" aria-haspopup="menu">' + I('more', 20) + '</button></div>';
    }).join('') + '</div>';
  }
  function skeleton() {
    var n = 12, h = '';
    if (ST.view === 'grid') { for (var i = 0; i < n; i++) h += '<div class="tile"><div class="thumb sk" style="box-shadow:none"></div><div class="sk" style="height:12px;margin:10px auto 0;width:70%"></div><div class="sk" style="height:10px;margin:8px auto 0;width:40%"></div></div>'; return '<div class="grid" role="status" aria-busy="true" aria-label="' + t('loading') + '">' + h + '</div>'; }
    for (i = 0; i < 9; i++) h += '<div class="lrow" style="grid-template-columns:32px 1fr"><span class="sk" style="width:32px;height:32px"></span><span class="sk" style="height:12px;width:' + (40 + (i * 13) % 40) + '%"></span></div>';
    return '<div role="status" aria-busy="true" aria-label="' + t('loading') + '">' + h + '</div>';
  }
  function filesPage(fid) {
    var s = R.q.state || 'populated', offline = s === 'offline', c, ids = CH[fid] || [];
    if (s === 'loading') c = skeleton();
    else if (s === 'error') c = stateBlock('alert', 'err', t('error_load_title'), t('error_load_text'), '<button class="btn primary" data-act="noop">' + I('refresh', 16) + t('retry') + '</button>');
    else if (s === 'drive') c = stateBlock('drive', 'warn', t('error_drive_title'), t('error_drive_text'), '<button class="btn plain" data-act="noop">' + t('retry') + '</button>');
    else if (s === 'offline-empty') c = stateBlock('wifioff', 'gray', t('offline_title'), t('offline_text'), '<button class="btn plain" data-act="noop">' + t('retry') + '</button>');
    else if (ST.q) {
      var q = ST.q.toLowerCase(), res = Object.keys(F).map(Number).filter(function (id) { return F[id].n.toLowerCase().indexOf(q) >= 0; });
      c = res.length ? '<div class="pad" style="padding-bottom:0"><h2 class="h2" style="font-size:19px">' + t('search_results_for', { q: esc(ST.q) }) + '</h2><p class="small">' + t('search_found', { n: res.length }) + '</p></div>' + itemsHTML(sorted(res)) : stateBlock('search', 'gray', t('no_results'), t('no_results_text', { q: ST.q }));
    }
    else if (!ids.length) c = stateBlock('cloud', '', t('empty_title'), t('empty_text'), '<button class="btn primary" data-act="uploadmenu">' + I('upload', 16) + t('upload_files') + '</button>');
    else c = itemsHTML(sorted(ids));
    var phoneSearch = '<div class="hide-wide pad" style="padding:8px 12px 0"><div class="search" role="search" style="width:100%">' + I('search', 16) + '<input type="search" data-search aria-label="' + t('search_placeholder') + '" placeholder="' + t('search_placeholder') + '" value="' + esc(ST.q) + '"></div></div>';
    var cs = (s === 'loading' || s === 'error' || s === 'drive' || s === 'offline-empty') ? '' : phoneSearch;
    return shell('files', filesBar(fid, offline), cs + c);
  }

  /* ---------- trash, shares, storage, settings, admin ---------- */
  function trashPage() {
    var s = R.q.state, list = s === 'empty' ? [] : ST.trash, total = list.reduce(function (a, x) { return a + x.s; }, 0);
    var bar = titleBar(t('trash_title'), list.length ? '<button class="btn danger" data-act="dlg" data-v="emptytrash">' + I('trash', 16) + t('empty_trash') + '</button>' : '');
    var body;
    if (s === 'loading') body = '<div class="page"><div class="cell-list">' + [1, 2, 3, 4].map(function () { return '<div class="srow"><span class="sk" style="width:40px;height:40px"></span><span class="sk" style="height:14px;width:50%"></span><span></span></div>'; }).join('') + '</div></div>';
    else if (!list.length) body = stateBlock('trash', 'gray', t('trash_empty_title'), t('trash_empty_text'));
    else body = '<div class="page"><div><h1 class="h1">' + t('trash_title') + '</h1><p class="lede" style="margin-top:6px">' + t('trash_note') + '</p></div><div class="cell-list" role="list">' + list.map(function (x) {
      var left = x.left === 0 ? t('trash_today') : x.left === 1 ? t('trash_day_left') : t('trash_days_left', { n: x.left }), urgent = x.left <= 1;
      return '<div class="srow" role="listitem">' + mini(x, 'ico') + '<div style="min-width:0"><div class="nm trunc">' + esc(x.n) + '</div><div class="sub"><span class="chip ' + (urgent ? 'warn' : '') + '">' + (urgent ? I('clock', 12) : '') + left + '</span><span>' + t('from_folder', { folder: esc(x.f) }) + '</span><span class="num">' + size(x.s) + '</span></div></div><div class="acts"><button class="btn tint" data-act="restore" data-id="' + x.id + '">' + I('restore', 16) + '<span class="hide-phone">' + t('restore') + '</span><span class="sr hide-wide">' + t('restore') + '</span></button><button class="ibtn" style="color:var(--err)" data-act="dlg" data-v="delforever" data-id="' + x.id + '" aria-label="' + t('delete_forever') + ': ' + esc(x.n) + '">' + I('trash', 20) + '</button></div></div>';
    }).join('') + '</div><p class="small num" style="text-align:center">' + t('items_n', { n: list.length }) + ' · ' + size(total) + '</p></div>';
    return shell('trash', bar, body);
  }
  function sharesPage() {
    var s = R.q.state, list = s === 'empty' ? [] : ST.shares;
    var body = !list.length ? stateBlock('share', '', t('shared_empty_title'), t('shared_empty_text'), '<a class="btn primary" href="' + H('files', { state: 'populated' }) + '">' + t('nav.files') + '</a>') :
      '<div class="page"><div><h1 class="h1">' + t('shared_title') + '</h1><p class="lede" style="margin-top:6px">' + t('anyone_with_link') + '</p></div><div class="cell-list" role="list">' + list.map(function (x) {
        var expd = x.exp !== null && x.exp < 0;
        return '<div class="srow" role="listitem">' + mini(x, 'ico') + '<div style="min-width:0"><div class="nm trunc">' + esc(x.n) + '</div><div class="sub">' + (expd ? '<span class="chip err">' + I('clock', 12) + t('expired') + '</span>' : '<span class="chip">' + (x.exp === null ? t('never_expires') : t('expires_on', { date: dlong(x.exp) })) + '</span>') + '<span class="chip ' + (x.dl ? 'acc' : '') + '">' + (x.dl ? I('download', 12) + t('download_on') : I('lock', 12) + t('download_off')) + '</span><span class="num">' + t('opened_n', { n: nf(x.hits) }) + '</span></div></div><div class="acts"><button class="btn tint" data-act="copylink"' + (expd ? ' disabled' : '') + '>' + I('link', 16) + '<span class="hide-phone">' + t('copy_link') + '</span><span class="sr hide-wide">' + t('copy_link') + '</span></button><button class="ibtn" data-act="dlg" data-v="revoke" data-id="' + x.id + '" aria-label="' + t('revoke_link') + ': ' + esc(x.n) + '">' + I('x', 20) + '</button></div></div>';
      }).join('') + '</div></div>';
    return shell('shared', titleBar(t('nav.shared')), body);
  }
  function storagePage() {
    var low = R.q.state === 'low', used = low ? 926 * GB : USED, K = low ? [['photos', 450, 'photo'], ['videos', 330, 'video'], ['audio', 40, 'audio'], ['documents', 61, 'doc'], ['other', 45, 'other']] : KINDS;
    var meter = '<div class="meter big" role="img" aria-label="' + esc(t('storage_used', { used: size(used), total: size(TOTAL) })) + '">' + K.map(function (k) { return '<i style="width:' + (k[1] * GB / TOTAL * 100) + '%;background:var(--k-' + k[2] + ')"></i>'; }).join('') + '</div>';
    var body = '<div class="page"><h1 class="h1">' + t('nav.storage') + '</h1>' +
      (low ? '<div class="phase" role="status" style="background:var(--warn-bg);color:var(--warn)">' + I('alert', 24) + '<div style="flex:1"><b>' + t('storage_low') + '</b></div><a class="btn text" href="' + H('files', { state: 'populated', view: 'list' }) + '" style="color:var(--warn)">' + t('storage_manage') + '</a></div>' : '') +
      '<div class="card pad-card" style="display:grid;gap:16px"><div style="display:flex;align-items:baseline;justify-content:space-between;gap:12px;flex-wrap:wrap"><span style="font-size:34px;font-weight:700;letter-spacing:-.03em" class="num">' + size(used) + '</span></div>' + meter + '<div class="row-kv num"><span>' + t('storage_used', { used: size(used), total: size(TOTAL) }) + '</span><span>' + t('storage_free', { free: size(TOTAL - used) }) + '</span></div></div>' +
      '<div><div class="group-label">' + t('storage_by_kind') + '</div><div class="cell-list legend" style="padding:0 12px">' + K.map(function (k) { return '<div class="li"><span class="dot" style="background:var(--k-' + k[2] + ')"></span><span style="flex:1;font-weight:500">' + t('kind.' + k[0]) + '</span><span class="small num">' + size(k[1] * GB) + '</span></div>'; }).join('') + '<div class="li"><span class="dot" style="background:none;box-shadow:inset 0 0 0 1.5px var(--text-3)"></span><span style="flex:1;font-weight:500">' + t('kind.trash') + '</span><span class="small num">' + size(2.8 * GB) + '</span></div></div></div>' +
      '<div><div class="group-label">' + t('storage_drive') + '</div><div class="cell-list"><div class="cell"><span class="tile-ic gray">' + I('drive', 20) + '</span><div class="grow"><div class="lbl">' + t('storage_drive_name') + '</div><div class="sub">' + size(TOTAL) + '</div></div><span class="chip ok">' + I('check', 12) + t('storage_online') + '</span></div></div></div></div>';
    return shell('storage', titleBar(t('nav.storage')), body);
  }
  function settingsPage() {
    var langs = '<div class="seg lg" role="group" aria-label="' + t('language') + '" style="flex-wrap:wrap">' + LANGS.map(function (l) { return '<button data-act="lang" data-v="' + l[0] + '" lang="' + l[0] + '" aria-pressed="' + (set.lang === l[0]) + '">' + l[1] + '</button>'; }).join('') + '</div>';
    var ap = seg([['auto', t('ap_auto')], ['light', t('ap_light')], ['dark', t('ap_dark')]], set.theme, 'theme', 'lg');
    var body = '<div class="page"><div style="display:flex;gap:18px;align-items:center"><span class="avatar lg" aria-hidden="true">' + USER.initial + '</span><div style="min-width:0"><h1 class="h1 trunc">' + USER.name + '</h1><p class="small">' + t('profile') + '</p></div></div>' +
      '<div><div class="group-label">' + t('profile') + '</div><div class="cell-list"><div class="cell"><label class="lbl" for="dn2" style="width:130px;flex:none">' + t('display_name') + '</label><input class="input" id="dn2" style="height:40px;font-size:16px" value="' + USER.name + '"><button class="btn tint" data-act="toast" data-v="saved">' + t('save') + '</button></div></div></div>' +
      '<div><div class="group-label">' + t('sign_in_methods') + '</div><div class="cell-list"><div class="cell"><span class="tile-ic blue">' + I('mail', 18) + '</span><div class="grow"><div class="lbl">' + t('email') + '</div><div class="sub trunc">mei.chen@example.com</div></div><span class="chip ok">' + I('check', 12) + t('verified') + '</span><button class="btn text" data-act="noop">' + t('change') + '</button></div>' +
      '<div class="cell"><span class="tile-ic green">' + I('phone', 18) + '</span><div class="grow"><div class="lbl">' + t('phone') + '</div><div class="sub num">+1 555 0134</div></div><span class="chip warn">' + I('alert', 12) + t('not_verified') + '</span><button class="btn tint" data-act="dlg" data-v="otp">' + t('verify') + '</button></div></div><p class="hint" style="padding:8px 16px 0">' + t('keep_one') + '</p></div>' +
      '<div><div class="cell-list"><button class="cell" style="width:100%" data-act="dlg" data-v="password"><span class="tile-ic orange">' + I('key', 18) + '</span><span class="grow lbl">' + t('change_password') + '</span>' + I('chevR', 18) + '</button></div></div>' +
      '<div><div class="group-label">' + t('language') + '</div>' + langs + '</div><div><div class="group-label">' + t('appearance') + '</div>' + ap + '</div>' +
      '<div class="cell-list"><a class="cell" href="' + H('storage') + '" style="color:inherit"><span class="tile-ic purple">' + I('drive', 18) + '</span><span class="grow"><span class="lbl" style="display:block">' + t('nav.storage') + '</span><span class="sub num">' + t('storage_used', { used: size(USED), total: size(TOTAL) }) + '</span></span>' + I('chevR', 18) + '</a>' +
      (USER.admin ? '<a class="cell" href="' + H('admin') + '" style="color:inherit"><span class="tile-ic gray">' + I('shield', 18) + '</span><span class="grow lbl">' + t('nav.admin') + '</span><span class="badge">2</span>' + I('chevR', 18) + '</a>' : '') + '</div>' +
      '<button class="btn plain lg block" data-act="signout" style="color:var(--err)">' + t('sign_out') + '</button></div>';
    return shell('settings', titleBar(t('nav.settings')), body);
  }
  function adminPage() {
    var u = ST.upd, v1 = 'v0.3.1-rc.2', v2 = 'v0.3.1', ph = u.phase, st = '';
    var hero = function (title, sub, right) { return '<div class="card pad-card"><div class="updhero"><span class="logo">' + I('cloud', 38) + '</span><div style="flex:1;min-width:0"><div class="h2">' + title + '</div><p class="small" style="margin-top:2px">' + sub + '</p></div>' + (right || '') + '</div></div>'; };
    if (ph === 'idle') st = hero(t('up_to_date'), t('current_version') + ' <span class="ver">' + v1 + '</span> · ' + t('checked_at', { t: dfmt(0) }), '<button class="btn plain" data-act="admin" data-v="checking">' + t('check_now') + '</button>');
    else if (ph === 'checking') st = hero(t('checking'), t('current_version') + ' <span class="ver">' + v1 + '</span>', '<span class="spin" style="color:var(--accent-fill)">' + I('refresh', 24) + '</span>');
    else if (ph === 'available') st = hero(t('update_available') + ': <span class="ver" style="font-size:17px">' + v2 + '</span>', t('current_version') + ' <span class="ver">' + v1 + '</span>', '<button class="btn primary lg" data-act="admin" data-v="install">' + I('up', 18) + t('install_update') + '</button>') + '<div class="card pad-card" style="display:grid;gap:10px"><div style="display:flex;justify-content:space-between;align-items:center;gap:12px"><b>' + t('release_notes') + '</b><a href="#/admin" data-act="noop" style="white-space:nowrap">' + t('release_notes') + ' ' + I('chevR', 12) + '</a></div><ul class="notes"><li>Admin page for updates</li><li>Faster folder loading</li><li>Fixes for uploads of very large files</li></ul><p class="small">' + t('update_covers') + '</p></div>';
    else if (ph === 'downloading' || ph === 'installing' || ph === 'restarting' || ph === 'waiting') {
      var label = ph === 'downloading' ? t('phase.downloading', { pct: u.pct }) : ph === 'installing' ? t('phase.installing') : ph === 'restarting' ? t('phase.restarting') : t('phase.waiting');
      var steps = ['downloading', 'installing', 'restarting'], idx = steps.indexOf(ph === 'waiting' ? 'restarting' : ph);
      st = hero(v1 + ' → ' + v2, label, '<span class="spin" style="color:var(--accent-fill)">' + I('refresh', 24) + '</span>') + '<div class="card pad-card" style="display:grid;gap:14px">' + (ph === 'downloading' ? '<div class="pbar" style="height:6px" role="progressbar" aria-valuenow="' + u.pct + '" aria-valuemin="0" aria-valuemax="100"><i style="width:' + u.pct + '%"></i></div>' : '<div class="indet" style="width:100%"></div>') +
        '<div class="steps" style="box-shadow:none;padding:0;background:none">' + steps.map(function (s, i) { return '<div class="s ' + (i < idx ? 'done' : i === idx ? 'now' : '') + '"><span class="n">' + (i < idx ? I('check', 16) : i + 1) + '</span>' + t(s === 'downloading' ? 'phase.downloading' : s === 'installing' ? 'phase.installing' : 'phase.restarting', { pct: s === 'downloading' ? (i < idx ? 100 : u.pct) : '' }) + '</div>'; }).join('') + '</div><p class="small">' + t('update_covers') + '</p></div>';
    }
    else if (ph === 'done') st = hero(t('phase.done', { v: v2 }), t('current_version') + ' <span class="ver">' + v2 + '</span>', '<span class="chip ok">' + I('check', 12) + t('done') + '</span>');
    else if (ph === 'failed') st = '<div class="card pad-card"><div class="updhero"><span class="statusmark warn" style="margin:0;flex:none">' + I('alert', 34) + '</span><div style="flex:1;min-width:0"><div class="h2">' + t('phase.failed') + '</div><p class="small" style="margin-top:4px">' + t('phase.failed_text', { err: '' }) + ' <span class="ver">health check timed out</span></p></div><button class="btn primary" data-act="admin" data-v="install">' + t('retry') + '</button></div></div>';
    var pend = '<div><div class="group-label">' + t('accounts') + ' · ' + t('approval.pending_n', { n: ST.pend === undefined ? 2 : ST.pend }) + '</div><div class="cell-list">' + (ST.pend === 0 ? '<div class="cell"><span class="sub">' + t('approval.pending_n', { n: 0 }) + '</span></div>' : [['Ming Li', 'ming.li@example.com', 1], ['Aunt Rose', '+1 555 0188', 3]].map(function (a) {
      return '<div class="cell" style="flex-wrap:wrap"><span class="avatar" aria-hidden="true" style="background:linear-gradient(160deg,#ffb35e,#ee7d1a)">' + a[0][0] + '</span><div class="grow"><div class="lbl">' + a[0] + '</div><div class="sub trunc">' + a[1] + ' · ' + t('approval.registered', { date: dfmt(a[2]) }) + '</div></div><button class="btn tint" data-act="approve">' + t('approval.approve') + '</button><button class="btn plain" data-act="approve">' + t('approval.reject') + '</button></div>';
    }).join('')) + '</div></div>';
    var ctl = '<div><div class="group-label">' + t('software_update') + '</div><div class="cell-list"><div class="cell"><span class="grow lbl">' + t('update_mode') + '</span>' + seg([['manual', t('manual')], ['auto', t('automatic')]], ST.mode || 'manual', 'mode') + '</div>' +
      '<div class="cell" style="flex-wrap:wrap"><span class="grow"><span class="lbl" style="display:block">' + t('update_window') + '</span><span class="sub">' + t('update_window_hint') + '</span></span><select class="input" style="width:170px;height:38px;font-size:15px" aria-label="' + t('update_window') + '"><option>02:00 – 04:00</option><option>00:00 – 02:00</option><option>04:00 – 06:00</option></select></div>' +
      '<div class="cell" style="flex-wrap:wrap"><span class="grow"><span class="lbl" style="display:block">' + t('channel') + '</span><span class="sub">' + t('ch_hint') + '</span></span>' + seg([['stable', t('ch_stable')], ['pre', t('ch_pre')]], ST.ch || 'pre', 'chan') + '</div></div></div>';
    return shell('admin', titleBar(t('admin_title')), '<div class="page"><h1 class="h1">' + t('admin_title') + '</h1><div><div class="group-label">' + t('software_update') + '</div><div style="display:grid;gap:14px">' + st + '</div></div>' + ctl + pend + '</div>');
  }

  /* ---------- public share ---------- */
  function publicPage() {
    var s = R.q.state, f = F[5];
    var inner = s === 'gone' ? '<div class="statusmark err">' + I('link', 36) + '</div><h1>' + t('public_gone_title') + '</h1><p class="sub">' + t('public_gone_text') + '</p>' :
      '<div class="cover" style="width:min(100%,360px);aspect-ratio:4/3;border-radius:16px;margin:0 auto">' + ART(f.a) + '</div><h1 style="font-size:24px">' + esc(f.n) + '</h1><p class="sub">' + t('shared_by', { name: USER.name }) + ' · <span class="num">' + size(f.s) + '</span></p>' + (s === 'viewonly' ? '<span class="chip" style="margin:0 auto">' + I('lock', 12) + t('download_off') + '</span>' : '<button class="btn primary lg block" data-act="toast" data-v="downloading">' + I('download', 18) + t('download') + '</button>') + '<p class="auth-foot">' + t('public_note') + ' · ' + t('expires_on', { date: dlong(6) }) + '</p>';
    return '<div class="auth">' + langSeg() + '<div class="auth-card" role="main"><span class="logo" style="width:40px;height:40px;border-radius:11px">' + I('cloud', 22) + '</span>' + inner + '</div></div>';
  }

  /* ---------- preview ---------- */
  function previewPage() {
    var id = +R.parts[0] || 5, f = F[id] || F[5], base = shell('files', filesBar(0), itemsHTML(sorted(CH[0])));
    var top = '<div class="pv-top"><a class="ibtn" href="' + H('files', { state: 'populated' }) + '" aria-label="' + t('close') + '">' + I('x', 20) + '</a><div class="t trunc">' + esc(f.n) + '<small class="num">' + (f.t === 'image' ? t('n_of_m', { n: GAL.indexOf(id) + 1, m: GAL.length }) : size(f.s)) + '</small></div><button class="ibtn" data-act="dlg" data-v="share" data-id="' + id + '" aria-label="' + t('share') + '">' + I('share', 20) + '</button><button class="ibtn" data-act="toast" data-v="downloading" aria-label="' + t('download') + '">' + I('download', 20) + '</button></div>';
    var body;
    if (f.t === 'image') {
      var i = GAL.indexOf(id), p = GAL[(i + GAL.length - 1) % GAL.length], n = GAL[(i + 1) % GAL.length];
      body = '<div class="pv-stage" data-swipe><a class="ibtn pv-nav p" href="' + H('preview/' + p) + '" aria-label="' + t('prev') + '">' + I('chevL', 24) + '</a><div class="pv-img" role="img" aria-label="' + esc(f.n) + '">' + ART(f.a) + '</div><a class="ibtn pv-nav n" href="' + H('preview/' + n) + '" aria-label="' + t('next') + '">' + I('chevR', 24) + '</a></div><div class="strip" role="list">' + GAL.map(function (g) { return '<button role="listitem" data-act="go" data-v="' + g + '" aria-current="' + (g === id) + '" aria-label="' + esc(F[g].n) + '">' + ART(F[g].a) + '</button>'; }).join('') + '</div>';
    } else if (R.q.state === 'error') {
      body = '<div class="pv-stage">' + '<div class="state" style="color:#f5f5f7"><div class="big gray" style="background:rgba(255,255,255,.1);color:#a1a1a6">' + I(f.t === 'audio' ? 'music' : 'video', 44) + '</div><h2>' + t('media_error_title') + '</h2><p style="color:#a1a1a6">' + t('media_error_text') + '</p><button class="btn primary" data-act="toast" data-v="downloading">' + I('download', 16) + t('download') + '</button></div></div>';
    } else {
      var dur = f.t === 'audio' ? 222 : 138, pos = ST.pv.pos, pct = Math.min(100, pos / dur * 100), fmt = function (s) { s = Math.floor(s); return Math.floor(s / 60) + ':' + ('0' + (s % 60)).slice(-2); };
      var scrub = '<div class="scrub"><div class="tr"><div class="buf" style="width:' + Math.min(100, pct + 22) + '%"></div><div class="fg" id="pv-fg" style="width:' + pct + '%"></div></div><div class="kn" id="pv-kn" style="inset-inline-start:' + pct + '%"></div><input type="range" min="0" max="' + dur + '" value="' + Math.floor(pos) + '" data-scrub aria-label="' + t('position') + '"></div><div class="times num"><span id="pv-t">' + fmt(pos) + '</span><span>−' + fmt(dur - pos) + '</span></div>';
      var play = '<button class="ibtn play" data-act="play" aria-label="' + (ST.pv.playing ? t('pause') : t('play')) + '">' + I(ST.pv.playing ? 'pause' : 'play', 26) + '</button>';
      if (f.t === 'audio') body = '<div class="pv-stage"><div class="audio-wrap"><div class="cover">' + ART(1) + '</div><div><h2>Spring Rain</h2><p>Mei Chen · ' + t('kind.audio') + '</p></div><div class="pv-ctl">' + scrub + '<div class="tport"><button class="ibtn" data-act="seek" data-v="-15" aria-label="−15">' + I('skipB', 22) + '</button>' + play + '<button class="ibtn" data-act="seek" data-v="15" aria-label="+15">' + I('skipF', 22) + '</button></div><div class="tport" style="gap:12px;color:#a1a1a6">' + I('volume', 18) + '<div style="width:140px" class="scrub"><div class="tr"><div class="fg" style="width:70%"></div></div><input type="range" aria-label="' + t('volume') + '" value="70"></div></div></div></div></div>';
      else body = '<div class="pv-stage"><div class="vid">' + ART(f.a) + (ST.pv.playing ? '' : '<div class="vid-big"><button class="ibtn" data-act="play" aria-label="' + t('play') + '">' + I('play', 34) + '</button></div>') + '<div class="vid-ctl">' + scrub + '<div class="row"><button class="ibtn" data-act="play" aria-label="' + (ST.pv.playing ? t('pause') : t('play')) + '">' + I(ST.pv.playing ? 'pause' : 'play', 22) + '</button><span style="flex:1"></span><button class="ibtn" aria-label="' + t('volume') + '">' + I('volume', 22) + '</button><button class="ibtn" aria-label="' + t('fullscreen') + '">' + I('expand', 22) + '</button></div></div></div></div>';
    }
    return base.replace(/<\/main>/, '</main>') + '<div class="pv" role="dialog" aria-modal="true" aria-label="' + t('preview') + ': ' + esc(f.n) + '">' + top + body + '</div>';
  }

  /* ---------- overlays ---------- */
  function dialogHTML() {
    var d = ST.dlg; if (!d) return '';
    var c = '', cls = '', foot = '';
    var ids = ST.sel, sum = ids.reduce(function (a, i) { return a + (F[i] ? F[i].s : 0); }, 0), folders = ids.filter(function (i) { return F[i] && F[i].t === 'folder'; }).length;
    function wrap(title, inner, actions, k) { return '<div class="scrim" data-act="scrim"><div class="dialog ' + (k || '') + '" role="' + (k === 'alert' ? 'alertdialog' : 'dialog') + '" aria-modal="true" aria-labelledby="dt"><span class="grab"></span><h2 id="dt">' + title + '</h2>' + inner + '<div class="row-end">' + actions + '</div></div></div>'; }
    if (d.type === 'download') {
      if (d.phase === 0) return wrap(t('download_n', { n: ids.length }), '<div role="radiogroup" aria-label="' + t('download_as') + '" style="display:grid;gap:10px"><button class="opt" role="radio" aria-checked="' + (d.fmt === 'zip') + '" data-act="fmt" data-v="zip"><span class="radio">' + I('check', 13) + '</span><span><b>' + t('download_zip') + '</b><span class="small">' + t('download_zip_hint') + '</span></span></button><button class="opt" role="radio" aria-checked="' + (d.fmt === 'targz') + '" data-act="fmt" data-v="targz"><span class="radio">' + I('check', 13) + '</span><span><b>' + t('download_targz') + '</b><span class="small">' + t('download_targz_hint') + '</span></span></button></div><p class="small">' + (folders ? t('download_includes', { n: folders }) + ' · ' : '') + t('download_size', { size: size(sum) }) + '</p>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn primary" data-act="startdl">' + I('download', 16) + t('download_start') + '</button>');
      return wrap(t('download_preparing'), '<div class="indet" style="width:100%"></div><p class="small">' + t('download_started') + '</p>', '<button class="btn primary" data-act="closedlg">' + t('done') + '</button>');
    }
    if (d.type === 'share') {
      var f = F[d.id] || F[5], opts = [['1d', 'exp_1d'], ['7d', 'exp_7d'], ['30d', 'exp_30d'], ['never', 'exp_never']];
      var head = '<div class="cell" style="padding:0;min-height:0">' + mini(f, 'ico') + '<div class="grow"><div class="lbl trunc">' + esc(f.n) + '</div><div class="sub">' + t('anyone_with_link') + '</div></div></div>';
      if (d.phase === 0) return wrap(t('share_link'), head + '<div class="field"><label>' + t('link_expires') + '</label><div class="seg lg" role="group" style="width:100%;flex-wrap:wrap">' + opts.map(function (o) { return '<button style="flex:1" data-act="exp" data-v="' + o[0] + '" aria-pressed="' + (d.exp === o[0]) + '">' + t(o[1]) + '</button>'; }).join('') + '</div></div><div class="cell" style="padding:0"><span class="grow"><span class="lbl" style="display:block">' + t('allow_download') + '</span><span class="sub">' + t('allow_download_hint') + '</span></span><button class="switch" role="switch" aria-checked="' + d.dl + '" data-act="dltoggle" aria-label="' + t('allow_download') + '"></button></div>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn primary" data-act="mklink">' + I('link', 16) + t('share_create') + '</button>');
      return wrap(t('share_link'), head + '<div class="linkbox"><span class="grow trunc">home.example/s/8fK2QxLm</span><button class="btn primary" data-act="copylink" style="height:32px">' + t('copy_link') + '</button></div><p class="small">' + t('anyone_with_link') + ' · ' + (d.exp === 'never' ? t('never_expires') : t('link_expires') + ': ' + t('exp_' + d.exp)) + '</p>', '<button class="btn primary" data-act="closedlg">' + t('done') + '</button>');
    }
    if (d.type === 'newfolder') return wrap(t('new_folder'), '<div class="field"><label for="nf">' + t('folder_name') + '</label><input class="input" id="nf" autofocus value="' + t('untitled_folder') + '"></div>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn primary" data-act="closedlg">' + t('create') + '</button>');
    if (d.type === 'emptytrash') return wrap(t('confirm_empty_title'), '<p>' + t('confirm_empty_text', { n: ST.trash.length, size: size(ST.trash.reduce(function (a, x) { return a + x.s; }, 0)) }) + '</p>', '<button class="btn danger-fill" data-act="doempty">' + t('empty_trash') + '</button><button class="btn plain" data-act="closedlg">' + t('cancel') + '</button>', 'alert');
    if (d.type === 'delforever') { var x = ST.trash.filter(function (q) { return q.id === d.id; })[0] || ST.trash[0]; return wrap(t('confirm_delete_title', { name: esc(x.n) }), '<p>' + t('confirm_delete_text') + '</p>', '<button class="btn danger-fill" data-act="dodel" data-id="' + x.id + '">' + t('delete_forever') + '</button><button class="btn plain" data-act="closedlg">' + t('cancel') + '</button>', 'alert'); }
    if (d.type === 'revoke') { var y = ST.shares.filter(function (q) { return q.id === d.id; })[0] || ST.shares[0]; return wrap(t('revoke_title', { name: esc(y.n) }), '<p>' + t('revoke_text') + '</p>', '<button class="btn danger-fill" data-act="dorevoke" data-id="' + y.id + '">' + t('revoke_link') + '</button><button class="btn plain" data-act="closedlg">' + t('cancel') + '</button>', 'alert'); }
    if (d.type === 'busy') return wrap(t('install_update'), '<div class="phase" style="background:var(--warn-bg);color:var(--warn);padding:14px">' + I('alert', 22) + '<span>' + t('busy_transfers', { n: 2 }) + '</span></div>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn danger-fill" data-act="forceinstall">' + t('install_anyway') + '</button>');
    if (d.type === 'password') return wrap(t('change_password'), '<div class="field"><label for="p1">' + t('current_password') + '</label><input class="input" id="p1" type="password" autofocus></div><div class="field"><label for="p2">' + t('new_password') + '</label><input class="input" id="p2" type="password"><span class="hint">' + t('password_hint') + '</span></div>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn primary" data-act="savedlg">' + t('save') + '</button>');
    if (d.type === 'otp') {
      if (d.phase === 0) return wrap(t('verify') + ' · ' + t('phone'), '<p>+1 555 0134</p>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn primary" data-act="sendcode">' + t('send_code') + '</button>');
      return wrap(t('one_time_code'), '<p>' + t('code_sent_to', { to: '+1 555 0134' }) + '</p><input class="input otp num" inputmode="numeric" autocomplete="one-time-code" maxlength="6" aria-label="' + t('one_time_code') + '" autofocus placeholder="······"><p class="hint num">' + t('code_resend_in', { s: 42 }) + '</p>', '<button class="btn plain" data-act="closedlg">' + t('cancel') + '</button><button class="btn primary" data-act="savedlg">' + t('verify') + '</button>');
    }
    return '';
  }
  function menuHTML() {
    var m = ST.menu; if (!m) return '';
    var it = function (ico, label, act, extra) { return '<button role="menuitem" data-act="' + act + '"' + (extra || '') + '>' + I(ico, 18) + esc(label) + '</button>'; }, items = '';
    if (m.type === 'file') { var f = F[m.id]; items = it('expand', t('open'), 'openid', ' data-id="' + m.id + '"') + it('download', t('download'), 'dlmenu') + it('share', t('share'), 'sharemenu', ' data-id="' + m.id + '"') + '<hr>' + it('edit', t('rename'), 'noop') + it('move', t('move'), 'noop') + '<hr>' + it('trash', t('delete'), 'trashid', ' data-id="' + m.id + '" class="dng"'); }
    if (m.type === 'upload') items = it('upload', t('upload_files'), 'startup') + it('folder', t('upload_folder'), 'startup') + '<hr>' + it('plus', t('new_folder'), 'newfolder');
    if (m.type === 'sort') items = [['name', 'col.name'], ['mod', 'col.modified'], ['size', 'col.size']].map(function (s) { return '<button role="menuitemradio" aria-checked="' + (ST.sort.k === s[0]) + '" data-act="sort" data-v="' + s[0] + '">' + (ST.sort.k === s[0] ? I('check', 18) : '<span style="width:18px"></span>') + t(s[1]) + '</button>'; }).join('');
    if (m.type === 'me') items = '<div style="padding:8px 10px;display:flex;gap:10px;align-items:center"><span class="avatar">' + USER.initial + '</span><span style="font-weight:600">' + USER.name + '</span></div><hr>' + LANGS.map(function (l) { return '<button role="menuitemradio" aria-checked="' + (set.lang === l[0]) + '" data-act="lang" data-v="' + l[0] + '" lang="' + l[0] + '">' + (set.lang === l[0] ? I('check', 18) : '<span style="width:18px"></span>') + l[1] + '</button>'; }).join('') + '<hr><a role="menuitem" href="' + H('settings') + '">' + I('sliders', 18) + t('nav.settings') + '</a><button role="menuitem" data-act="signout" class="dng">' + I('x', 18) + t('sign_out') + '</button>';
    return '<div class="menu" role="menu" style="left:' + m.x + 'px;top:' + m.y + 'px" data-menu>' + items + '</div>';
  }
  function upHTML() {
    var u = ST.up; if (!u) return '';
    var items = u.items, act = items.filter(function (x) { return x.st === 'up' || x.st === 'wait'; }), err = items.filter(function (x) { return x.st === 'err'; }).length, done = items.filter(function (x) { return x.st === 'done'; }).length, offline = R.q.state === 'offline';
    var total = items.reduce(function (a, x) { return a + x.s; }, 0), sent = items.reduce(function (a, x) { return a + x.s * x.p / 100; }, 0), pct = Math.round(sent / total * 100);
    var title = act.length ? t('uploading_n', { n: act.length }) : err ? t('upload_failed') : t('upload_done'), sub = act.length ? (offline ? t('upload_paused_offline') : t('upload_remaining', { t: nf(2, { style: 'unit', unit: 'minute', unitDisplay: 'long' }) }) + ' · ' + t('upload_to', { folder: esc(u.folder) })) : err ? t('upload_failed_hint') : t('upload_done_n', { n: done });
    var C = 2 * Math.PI * 11, ring = '<span class="ring-wrap ' + (!act.length && !err ? 'ok' : err && !act.length ? 'err' : '') + '"><svg class="ring" viewBox="0 0 28 28" aria-hidden="true"><circle class="tr" cx="14" cy="14" r="11"/><circle class="fg" cx="14" cy="14" r="11" stroke-dasharray="' + C + '" stroke-dashoffset="' + (C * (1 - (act.length ? pct : 100) / 100)) + '" style="' + (!act.length ? (err ? 'stroke:var(--err-fill)' : 'stroke:var(--ok)') : '') + '"/></svg>' + (!act.length ? I(err ? 'x' : 'check', 14) : '') + '</span>';
    var rows = u.min ? '' : '<div class="up-list" role="list">' + items.map(function (x, i) {
      var state = x.st === 'up' ? '<div class="pbar" role="progressbar" aria-valuenow="' + Math.round(x.p) + '" aria-valuemin="0" aria-valuemax="100" aria-label="' + esc(x.n) + '"><i style="width:' + x.p + '%"></i></div><div class="st num">' + Math.round(x.p) + '% · ' + size(x.s * x.p / 100) + ' / ' + size(x.s) + '</div>' : x.st === 'wait' ? '<div class="st">' + (offline ? t('upload_paused_offline') : t('upload_waiting')) + ' · ' + size(x.s) + '</div>' : x.st === 'done' ? '<div class="st ok">' + I('check', 12) + ' ' + (x.as ? t('renamed_as', { name: esc(x.as) }) : t('upload_done')) + '</div>' : x.st === 'err' ? '<div class="st err">' + t('upload_failed') + ' · ' + t('upload_failed_hint') + '</div>' : '<div class="st">' + t('upload_cancelled') + '</div>';
      var btn = x.st === 'up' || x.st === 'wait' ? '<button class="ibtn" data-act="upcancel" data-i="' + i + '" aria-label="' + t('upload_cancel') + ': ' + esc(x.n) + '">' + I('x', 18) + '</button>' : x.st === 'err' ? '<button class="ibtn acc" data-act="upretry" data-i="' + i + '" aria-label="' + t('retry') + ': ' + esc(x.n) + '">' + I('refresh', 18) + '</button>' : '<span style="width:36px"></span>';
      return '<div class="up-row" role="listitem"><span class="tile-ic ' + (x.n.match(/jpg|png/) ? 'orange' : x.n.match(/mp4/) ? 'purple' : x.n.match(/mp3/) ? 'red' : 'gray') + '" style="width:30px;height:30px">' + I(x.n.match(/jpg|png/) ? 'image' : x.n.match(/mp4/) ? 'video' : x.n.match(/mp3/) ? 'music' : 'zip', 16) + '</span><div class="t"><div class="nm trunc">' + esc(x.as || x.n) + '</div>' + state + '</div>' + btn + '</div>';
    }).join('') + '</div>';
    return '<section class="up" id="up" role="region" aria-label="' + t('upload') + '"><div class="up-h">' + ring + '<div class="t"><b>' + title + '</b><span class="num">' + sub + '</span></div><button class="ibtn" data-act="upmin" aria-expanded="' + !u.min + '" aria-label="' + (u.min ? t('upload_expand') : t('upload_minimize')) + '">' + I(u.min ? 'expand' : 'chevD', 18) + '</button><button class="ibtn" data-act="upclose" aria-label="' + (act.length ? t('upload_cancel') : t('close')) + '">' + I('x', 18) + '</button></div>' + rows + '</section>';
  }
  function bannerHTML() {
    var b = R.q.banner, h = '';
    if (b === 'newversion' || b === 'newversion-busy') h += '<div class="banner" role="status"><span class="badge-ic">' + I('up', 20) + '</span><div class="tx"><b>' + t('new_version') + '</b><span class="s">' + (b === 'newversion' ? t('new_version_text', { v: 'v0.3.1' }) : t('reload_after_upload')) + '</span></div>' + (b === 'newversion' ? '<button class="btn primary" data-act="reload">' + t('reload') + '</button><button class="btn text" data-act="dismissbanner">' + t('later') + '</button>' : '') + '</div>';
    if (b === 'updating') h += '<div class="updating" role="status" aria-live="polite"><div class="box"><span class="logo" style="width:72px;height:72px;border-radius:20px">' + I('cloud', 40) + '</span><h2>' + t('updating') + '</h2><p class="lede" style="font-size:16px">' + t('updating_text') + '</p><div class="indet"></div><span class="small num">' + t('updating_retry', { s: '<span id="retry-n">' + (ST.retry || 12) + '</span>' }) + '</span></div></div>';
    if (R.q.state === 'drop') h +='<div class="drop" aria-hidden="true"><div class="in"><span class="dz">' + I('upload', 40) + '</span><h2>' + t('drop_here') + '</h2><p>' + t('drop_into', { folder: R.parts[0] && F[R.parts[0]] ? esc(F[R.parts[0]].n) : t('nav.files') }) + '</p></div></div>';
    return h;
  }
  function toastHTML() { return ST.toast ? '<div class="toast" role="status">' + esc(ST.toast.msg) + (ST.toast.act ? '<button class="btn" data-act="' + ST.toast.act + '">' + ST.toast.label + '</button>' : '') + '</div>' : ''; }

  /* ---------- render ---------- */
  var AUTH = ['signin', 'register', 'pending', 'account-pending', 'rejected', 'closed', 'busy'];
  function render() {
    var n = R.name, html;
    if (AUTH.indexOf(n) >= 0) html = authPage(n);
    else if (n === 'public') html = publicPage();
    else if (n === 'trash') html = trashPage();
    else if (n === 'shared') html = sharesPage();
    else if (n === 'storage') html = storagePage();
    else if (n === 'settings') html = settingsPage();
    else if (n === 'admin') html = adminPage();
    else if (n === 'preview') html = previewPage();
    else html = filesPage(+R.parts[0] || 0);
    var app = AUTH.indexOf(n) < 0 && n !== 'public';
    stage.innerHTML = html + (app ? upHTML() : '') + bannerHTML() + menuHTML() + dialogHTML() + toastHTML();
    var ae = stage.querySelector('[autofocus]'); if (ae) ae.focus();
    document.title = t('app.name');
    if (!document.getElementById('dock').contains(document.activeElement)) paintDock();
  }
  function toast(msg, act, label) { clearTimeout(timers.toast); ST.toast = { msg: msg, act: act, label: label }; timers.toast = setTimeout(function () { ST.toast = null; var e = stage.querySelector('.toast'); if (e) e.remove(); }, 4500); }
  function paintUp() { var e = document.getElementById('up'); if (!e || !ST.up) return; var tmp = document.createElement('div'); tmp.innerHTML = upHTML(); e.replaceWith(tmp.firstChild); }
  setInterval(function () {
    if (!ST.up || R.q.state === 'offline' || ST.up.items.every(function (x) { return x.st !== 'up' && x.st !== 'wait'; })) return;
    var run = ST.up.items.filter(function (x) { return x.st === 'up'; });
    run.forEach(function (x) { x.p = Math.min(100, x.p + x.v * 2); if (x.p >= 100) x.st = 'done'; });
    if (run.length < 2) { var w = ST.up.items.filter(function (x) { return x.st === 'wait'; })[0]; if (w) { w.st = 'up'; w.v = 1.5; } }
    paintUp();
  }, 700);
  setInterval(function () {
    if (R.name !== 'preview' || !ST.pv.playing) return; ST.pv.pos += 1; var f = F[+R.parts[0]]; var dur = f && f.t === 'audio' ? 222 : 138; if (ST.pv.pos >= dur) { ST.pv.playing = false; ST.pv.pos = 0; render(); return; }
    var pct = ST.pv.pos / dur * 100, a = document.getElementById('pv-fg'), k = document.getElementById('pv-kn'), tt = document.getElementById('pv-t');
    if (a) a.style.width = pct + '%'; if (k) k.style.insetInlineStart = pct + '%'; if (tt) tt.textContent = Math.floor(ST.pv.pos / 60) + ':' + ('0' + Math.floor(ST.pv.pos % 60)).slice(-2);
  }, 1000);
  function runUpdate() {
    var u = ST.upd; u.phase = 'downloading'; u.pct = 8; render(); clearInterval(timers.upd);
    timers.upd = setInterval(function () {
      if (R.name !== 'admin') { clearInterval(timers.upd); return; }
      if (u.phase === 'downloading') { u.pct += 14; if (u.pct >= 100) { u.pct = 100; u.phase = 'installing'; } }
      else if (u.phase === 'installing') u.phase = 'restarting';
      else if (u.phase === 'restarting') { u.phase = 'waiting'; R.q.banner = 'updating'; startRetry(); }
      else if (u.phase === 'waiting') { u.phase = 'done'; delete R.q.banner; toast(t('phase.done', { v: 'v0.3.1' })); }
      render();
    }, 1600);
  }

  /* ---------- events ---------- */
  function rel(el) { var a = stage.getBoundingClientRect(), b = el.getBoundingClientRect(); return { x: b.left - a.left, y: b.bottom - a.top, r: b.right - a.left, t: b.top - a.top }; }
  function placeMenu(type, el, extra) {
    var p = rel(el), w = 240, W = stage.clientWidth, Hh = stage.clientHeight, x = Math.max(8, Math.min(p.r - w, W - w - 8)), y = p.y + 6;
    if (type === 'me') { y = Math.max(8, p.t - 280); x = 12; }
    if (y > Hh - 260 && type !== 'me') y = Math.max(8, p.t - 250);
    ST.menu = Object.assign({ type: type, x: x, y: y }, extra || {});
  }
  function toggle(id) { var i = ST.sel.indexOf(id); if (i >= 0) ST.sel.splice(i, 1); else ST.sel.push(id); }
  function isTouch() { return stage.clientWidth < 700 || matchMedia('(pointer: coarse)').matches; }
  function openItem(id) { var f = F[id]; if (f.t === 'folder') location.hash = H('files/' + id, { state: 'populated' }); else if (f.t === 'image' || f.t === 'audio' || f.t === 'video') location.hash = H('preview/' + id); else toast(t('preview') + ': ' + f.n); }

  stage.addEventListener('click', function (e) {
    var el = e.target.closest('[data-act]'); if (!el) { if (ST.menu) { ST.menu = null; render(); } return; }
    var a = el.getAttribute('data-act'), v = el.getAttribute('data-v'), id = +el.getAttribute('data-id') || el.getAttribute('data-id');
    if (a === 'scrim') { if (e.target !== el) return; ST.dlg = null; return render(); }
    if (ST.menu && a !== 'rowmenu' && a !== 'uploadmenu' && a !== 'sortmenu' && a !== 'me') ST.menu = null;
    switch (a) {
      case 'tick': e.stopPropagation(); toggle(+el.closest('[data-id]').getAttribute('data-id')); break;
      case 'item': { var iid = +el.getAttribute('data-id'); if (isTouch()) { if (ST.sel.length) toggle(iid); else return openItem(iid); } else if (e.metaKey || e.ctrlKey || e.shiftKey) toggle(iid); else ST.sel = [iid]; break; }
      case 'desel': ST.sel = []; break;
      case 'view': ST.view = v; store.set('view', v); break;
      case 'sort': if (ST.sort.k === v) ST.sort.dir *= -1; else ST.sort = { k: v, dir: 1 }; break;
      case 'clearq': ST.q = ''; break;
      case 'lang': set.lang = v; store.set('lang', v); document.documentElement.lang = v; ST.menu = null; break;
      case 'theme': set.theme = v; store.set('theme', v); applyTheme(); break;
      case 'pw': ST.pw = !ST.pw; break;
      case 'rowmenu': e.stopPropagation(); if (ST.menu) { ST.menu = null; break; } placeMenu('file', el, { id: id }); break;
      case 'uploadmenu': if (ST.menu) { ST.menu = null; break; } placeMenu('upload', el); break;
      case 'sortmenu': if (ST.menu) { ST.menu = null; break; } placeMenu('sort', el); break;
      case 'me': if (ST.menu) { ST.menu = null; break; } placeMenu('me', el); break;
      case 'openid': openItem(id); return;
      case 'dlmenu': toast(t('download') + '…'); break;
      case 'sharemenu': ST.dlg = { type: 'share', phase: 0, exp: '7d', dl: true, id: id }; break;
      case 'trashid': case 'trash': { var n = a === 'trash' ? ST.sel.length : 1; ST.sel = []; toast(t('moved_to_trash', { n: n }), 'undo', t('undo')); break; }
      case 'undo': ST.toast = null; break;
      case 'dlg': ST.dlg = v === 'download' ? { type: 'download', fmt: 'zip', phase: 0 } : v === 'share' ? { type: 'share', phase: 0, exp: '7d', dl: true, id: id || ST.sel[0] || 5 } : { type: v, id: el.getAttribute('data-id'), phase: 0 }; break;
      case 'closedlg': ST.dlg = null; break;
      case 'savedlg': ST.dlg = null; toast(t('saved')); break;
      case 'fmt': ST.dlg.fmt = v; break;
      case 'startdl': ST.dlg.phase = 1; break;
      case 'exp': ST.dlg.exp = v; break;
      case 'dltoggle': ST.dlg.dl = !ST.dlg.dl; break;
      case 'mklink': ST.dlg.phase = 1; break;
      case 'copylink': toast(t('link_copied')); break;
      case 'newfolder': ST.dlg = { type: 'newfolder' }; break;
      case 'startup': ST.up = freshUp(); break;
      case 'upmin': ST.up.min = !ST.up.min; break;
      case 'upclose': ST.up = null; break;
      case 'upcancel': ST.up.items[+el.getAttribute('data-i')].st = 'cancel'; break;
      case 'upretry': ST.up.items[+el.getAttribute('data-i')].st = 'up'; ST.up.items[+el.getAttribute('data-i')].v = 2; break;
      case 'restore': ST.trash = ST.trash.filter(function (x) { return x.id !== el.getAttribute('data-id'); }); toast(t('restored_to', { folder: t('nav.files') }), 'undo', t('undo')); break;
      case 'doempty': ST.trash = []; ST.dlg = null; break;
      case 'dodel': ST.trash = ST.trash.filter(function (x) { return x.id !== el.getAttribute('data-id'); }); ST.dlg = null; break;
      case 'dorevoke': ST.shares = ST.shares.filter(function (x) { return x.id !== el.getAttribute('data-id'); }); ST.dlg = null; break;
      case 'sendcode': ST.dlg.phase = 1; break;
      case 'mode': ST.mode = v; break;
      case 'chan': ST.ch = v; break;
      case 'approve': ST.pend = (ST.pend === undefined ? 2 : ST.pend) - 1; toast(t('saved')); break;
      case 'admin': if (v === 'checking') { ST.upd.phase = 'checking'; clearTimeout(timers.chk); timers.chk = setTimeout(function () { ST.upd.phase = 'available'; render(); }, 1400); } else runUpdate(); return render();
      case 'forceinstall': ST.dlg = null; return runUpdate();
      case 'reload': delete R.q.banner; toast(t('done')); break;
      case 'dismissbanner': delete R.q.banner; break;
      case 'toast': toast(v === 'saved' ? t('saved') : t('download') + '…'); break;
      case 'play': ST.pv.playing = !ST.pv.playing; break;
      case 'seek': ST.pv.pos = Math.max(0, ST.pv.pos + (+v)); break;
      case 'go': location.hash = H('preview/' + v); return;
      case 'signout': ST.menu = null; location.hash = H('signin'); return;
      case 'skip': e.preventDefault(); document.getElementById('content').focus(); return;
      case 'noop': break;
      default: return;
    }
    render();
  });
  stage.addEventListener('dblclick', function (e) { var el = e.target.closest('[data-act=item]'); if (el && !isTouch()) openItem(+el.getAttribute('data-id')); });
  stage.addEventListener('contextmenu', function (e) { var el = e.target.closest('[data-act=item]'); if (!el) return; e.preventDefault(); var r = stage.getBoundingClientRect(); ST.menu = { type: 'file', id: +el.getAttribute('data-id'), x: Math.min(e.clientX - r.left, stage.clientWidth - 250), y: Math.min(e.clientY - r.top, stage.clientHeight - 260) }; render(); });
  stage.addEventListener('input', function (e) {
    var el = e.target;
    if (el.hasAttribute('data-search')) { ST.q = el.value; var pos = el.selectionStart; render(); var n = stage.querySelectorAll('[data-search]'); for (var i = 0; i < n.length; i++) if (n[i].offsetParent) { n[i].focus(); n[i].setSelectionRange(pos, pos); } }
    else if (el.id && (el.closest('[data-form]'))) ST.form[el.id] = el.value;
    else if (el.hasAttribute('data-scrub')) { ST.pv.pos = +el.value; }
  });
  stage.addEventListener('change', function (e) { if (e.target.hasAttribute('data-scrub')) render(); });
  stage.addEventListener('submit', function (e) {
    e.preventDefault(); var k = e.target.getAttribute('data-form'), f = ST.form, id = (f.id || '').trim().toLowerCase();
    if (k === 'signin') {
      if (!id || f.pw === 'wrong') return (location.hash = H('signin', { state: 'error' })), render();
      location.hash = id.indexOf('pending') >= 0 ? H('account-pending') : id.indexOf('reject') >= 0 ? H('rejected') : H('files', { state: 'populated' });
    } else location.hash = id.indexOf('closed') >= 0 ? H('closed') : id.indexOf('busy') >= 0 ? H('busy') : H('pending');
  });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') { if (ST.dlg) ST.dlg = null; else if (ST.menu) ST.menu = null; else if (ST.sel.length) ST.sel = []; else if (R.name === 'preview') return (location.hash = H('files', { state: 'populated' })); else return; render(); return; }
    if (R.name === 'preview') { var id = +R.parts[0], i = GAL.indexOf(id); if (i >= 0 && (e.key === 'ArrowRight' || e.key === 'ArrowLeft')) location.hash = H('preview/' + GAL[(i + (e.key === 'ArrowRight' ? 1 : GAL.length - 1)) % GAL.length]); if (e.key === ' ' && F[id].t !== 'image' && e.target.tagName !== 'BUTTON') { e.preventDefault(); ST.pv.playing = !ST.pv.playing; render(); } return; }
    var el = document.activeElement;
    if (el && el.getAttribute && el.getAttribute('data-act') === 'item') {
      var iid = +el.getAttribute('data-id');
      if (e.key === 'Enter') { e.preventDefault(); openItem(iid); } else if (e.key === ' ') { e.preventDefault(); toggle(iid); render(); var again = stage.querySelector('[data-id="' + iid + '"]'); if (again) again.focus(); }
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'a' && R.name === 'files' && e.target.tagName !== 'INPUT') { e.preventDefault(); ST.sel = (CH[+R.parts[0] || 0] || []).slice(); render(); }
  });
  /* drag and drop: real drag events show the same overlay as ?state=drop */
  var dc = 0;
  function dropOn() { if (R.name !== 'files' || stage.querySelector('.drop')) return; var d = document.createElement('div'); d.innerHTML = '<div class="drop" aria-hidden="true"><div class="in"><span class="dz">' + I('upload', 40) + '</span><h2>' + t('drop_here') + '</h2><p>' + t('drop_into', { folder: R.parts[0] && F[R.parts[0]] ? esc(F[R.parts[0]].n) : t('nav.files') }) + '</p></div></div>'; stage.appendChild(d.firstChild); }
  function dropOff() { var d = stage.querySelector('.drop'); if (d) d.remove(); }
  stage.addEventListener('dragenter', function (e) { e.preventDefault(); dc++; dropOn(); });
  stage.addEventListener('dragover', function (e) { e.preventDefault(); });
  stage.addEventListener('dragleave', function () { dc = Math.max(0, dc - 1); if (!dc) dropOff(); });
  stage.addEventListener('drop', function (e) { e.preventDefault(); dc = 0; dropOff(); if (R.name === 'files') { ST.up = freshUp(); render(); } });
  stage.addEventListener('scroll', function (e) { var c = e.target; if (c.classList && c.classList.contains('content')) { var b = c.parentNode.querySelector('.bar'); if (b) b.classList.toggle('scrolled', c.scrollTop > 48); } }, true);
  var tx = null;
  stage.addEventListener('touchstart', function (e) { tx = e.touches[0].clientX; }, { passive: true });
  stage.addEventListener('touchend', function (e) { if (tx === null || !e.target.closest('[data-swipe]')) return; var dx = e.changedTouches[0].clientX - tx; tx = null; if (Math.abs(dx) < 50) return; var id = +R.parts[0], i = GAL.indexOf(id); location.hash = H('preview/' + GAL[(i + (dx < 0 ? 1 : GAL.length - 1)) % GAL.length]); }, { passive: true });

  function applyTheme() { var th = set.theme === 'auto' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : set.theme; document.documentElement.setAttribute('data-theme', th); }
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function () { if (set.theme === 'auto') applyTheme(); });
  window.addEventListener('hashchange', function () { R = parse(); initState(); render(); stage.querySelector('.content') && (stage.querySelector('.content').scrollTop = 0); });

  /* ---------- review dock (mockup chrome, not product) ---------- */
  var DOCK = [
    ['Sign in and approval (R-006, R-062)', [['Sign in', 'signin'], ['Wrong password', 'signin?state=error'], ['Register', 'register'], ['Request submitted', 'pending'], ['Account pending', 'account-pending'], ['Rejected', 'rejected'], ['Registration closed', 'closed'], ['Too busy', 'busy']]],
    ['File browser (R-007)', [['Grid', 'files?state=populated'], ['List', 'files?state=populated&view=list'], ['Loading', 'files?state=loading'], ['Empty folder', 'files/4?state=populated'], ['Load error', 'files?state=error'], ['Drive missing', 'files?state=drive'], ['Offline', 'files?state=offline'], ['Offline, nothing loaded', 'files?state=offline-empty'], ['Multi-select', 'files?state=selection'], ['Search', 'files?state=search&q=kyoto'], ['No results', 'files?state=search&q=zzz'], ['Row menu', 'files?state=menu'], ['New folder', 'files?state=newfolder'], ['Nested folder', 'files/17?state=populated']]],
    ['Upload and download (R-008, R-010)', [['Drag over', 'files?state=drop'], ['Uploading', 'files?state=upload'], ['Minimized', 'files?state=upload-min'], ['Failed item', 'files?state=upload-fail'], ['Finished', 'files?state=upload-done'], ['Batch download', 'files?state=download']]],
    ['Preview (R-009)', [['Image gallery', 'preview/5'], ['Audio', 'preview/8?state=playing'], ['Video', 'preview/9'], ['Video playing', 'preview/9?state=playing'], ['Cannot play', 'preview/9?state=error']]],
    ['Trash, sharing, storage (R-011)', [['Trash', 'trash'], ['Trash empty', 'trash?state=empty'], ['Trash loading', 'trash?state=loading'], ['Shared', 'shared'], ['Shared empty', 'shared?state=empty'], ['Create link', 'files?state=share'], ['Public page', 'public'], ['Public: view only', 'public?state=viewonly'], ['Public: link gone', 'public?state=gone'], ['Storage', 'storage'], ['Storage low', 'storage?state=low']]],
    ['Account and admin (R-012, R-013)', [['Settings', 'settings'], ['Verify with code', 'settings?state=otp'], ['Change password', 'settings?state=password'], ['Update: current', 'admin'], ['available', 'admin?u=available'], ['checking', 'admin?u=checking'], ['downloading', 'admin?u=downloading'], ['installing', 'admin?u=installing'], ['restarting', 'admin?u=restarting'], ['failed', 'admin?u=failed'], ['transfers running', 'admin?u=busy']]],
    ['Following updates (R-027)', [['New version', 'files?state=populated&banner=newversion'], ['New version, upload running', 'files?state=upload&banner=newversion-busy'], ['Updating, reconnecting', 'files?state=populated&banner=updating'], ['Live link lost', 'files?state=populated&banner=reconnect']]]
  ];
  function dockBtn() { var b = document.getElementById('dock-btn'); b.innerHTML = I('sliders', 16) + 'Review'; b.setAttribute('aria-expanded', !document.getElementById('dock').hidden); }
  function paintDock() {
    var d = document.getElementById('dock'), cur = location.hash.replace(/^#\//, '');
    d.innerHTML = '<div><h3>Language</h3><div class="row">' + LANGS.map(function (l) { return '<button class="pill" lang="' + l[0] + '" data-d="lang" data-v="' + l[0] + '" aria-pressed="' + (set.lang === l[0]) + '">' + l[1] + '</button>'; }).join('') + '</div></div>' +
      '<div><h3>Appearance</h3><div class="row">' + [['auto', 'Auto'], ['light', 'Light'], ['dark', 'Dark']].map(function (x) { return '<button class="pill" data-d="theme" data-v="' + x[0] + '" aria-pressed="' + (set.theme === x[0]) + '">' + x[1] + '</button>'; }).join('') + '</div></div>' +
      '<div><h3>Device frame</h3><div class="row">' + [['auto', 'Window'], ['phone', 'Phone 390'], ['tablet', 'Tablet 834']].map(function (x) { return '<button class="pill" data-d="dev" data-v="' + x[0] + '" aria-pressed="' + (set.dev === x[0]) + '">' + x[1] + '</button>'; }).join('') + '</div></div>' +
      DOCK.map(function (g) { return '<div><h3>' + g[0] + '</h3><div class="row">' + g[1].map(function (i) { return '<a class="pill" href="#/' + i[1] + '" aria-current="' + (cur === i[1]) + '">' + i[0] + '</a>'; }).join('') + '</div></div>'; }).join('') +
      '<p class="dock-note">Mockup chrome, not part of the product. Sample data only. Sign in with any text; use “pending” or “reject” in the name to see the approval screens, or the password “wrong” for the error.</p>';
    dockBtn();
  }
  document.getElementById('dock-btn').addEventListener('click', function () { var d = document.getElementById('dock'); d.hidden = !d.hidden; dockBtn(); });
  document.getElementById('dock').addEventListener('click', function (e) {
    var b = e.target.closest('[data-d]'); if (!b) return; var k = b.getAttribute('data-d'), v = b.getAttribute('data-v');
    if (k === 'lang') { set.lang = v; store.set('lang', v); document.documentElement.lang = v; }
    if (k === 'theme') { set.theme = v; store.set('theme', v); applyTheme(); }
    if (k === 'dev') { set.dev = v; store.set('dev', v); wrap.setAttribute('data-dev', v); }
    render(); paintDock(); var again = document.querySelector('#dock [data-d="' + k + '"][data-v="' + v + '"]'); if (again) again.focus();
  });
  wrap.setAttribute('data-dev', set.dev); applyTheme();
  if (!location.hash) location.hash = H('signin');
  R = parse(); initState(); render();
})();
