// Smoke test for the app mockup (run: bun design/mockups/app/tools/smoke.js).
// Runs assets/app.js against a stub DOM, renders every screen and state of the review dock in all three languages,
// and fails on exceptions, empty output, "undefined"/"NaN" in markup, missing string keys or incomplete translations.
const fs = require('fs');
const path = require('path');
const base = path.join(__dirname, '..', 'assets') + path.sep;
function El(id) { this.id = id; this.listeners = {}; this._html = ''; this.hidden = true; this.style = {}; this.classList = { contains() { return false; }, toggle() {} }; this.clientWidth = 1200; this.clientHeight = 800; this.parentNode = null; }
El.prototype = {
  addEventListener(t, f) { (this.listeners[t] = this.listeners[t] || []).push(f); },
  set innerHTML(v) { this._html = v; }, get innerHTML() { return this._html; },
  querySelector() { return null; }, querySelectorAll() { return []; }, setAttribute(k, v) { this[k] = v; }, getAttribute() { return null; },
  getBoundingClientRect() { return { left: 0, top: 0, right: 100, bottom: 40 }; }, contains() { return false; }, focus() {}, appendChild() {}, remove() {}
};
const els = {}; ['stage', 'stage-wrap', 'dock-btn', 'dock'].forEach(i => els[i] = new El(i));
global.document = { getElementById: i => els[i], documentElement: { lang: 'en', setAttribute() {} }, addEventListener() {}, querySelector: () => null, createElement: () => new El('x'), activeElement: null, title: '' };
const winL = {};
global.window = global; global.localStorage = { _d: {}, getItem(k) { return this._d[k] || null; }, setItem(k, v) { this._d[k] = v; } };
global.matchMedia = () => ({ matches: false, addEventListener() {} });
global.location = { hash: '', search: '' };
global.addEventListener = (t, f) => { winL[t] = f; };
global.setInterval = () => 0; global.setTimeout = () => 0; global.clearInterval = () => {}; global.clearTimeout = () => {};
(0, eval)(fs.readFileSync(base + 'icons.js', 'utf8'));
(0, eval)(fs.readFileSync(base + 'i18n.js', 'utf8'));
const used = new Set(); const rawS = window.S;
window.S = new Proxy(rawS, { get(t, k) { if (typeof k === 'string') used.add(k); return t[k]; } });
const src = fs.readFileSync(base + 'app.js', 'utf8');
(0, eval)(src);
const DOCK = eval(src.match(/var DOCK = (\[[\s\S]*?\n  \]);/)[1]);
const routes = [].concat(...DOCK.map(g => g[1].map(i => i[1])));
let fails = 0, n = 0;
for (const lang of ['en', 'zh-Hans', 'zh-Hant']) {
  els.dock.listeners.click[0]({ target: { closest: () => ({ getAttribute: k => (k === 'data-d' ? 'lang' : lang) }) } });
  for (const r of routes) {
    location.hash = '#/' + r;
    try { winL.hashchange(); n++; } catch (e) { fails++; console.log('FAIL', lang, r, e.stack.split('\n').slice(0, 3).join(' | ')); continue; }
    const h = els.stage._html;
    if (!h || h.length < 200) { fails++; console.log('EMPTY', lang, r, h.length); }
    if (/undefined|NaN|\[object/.test(h)) { fails++; console.log('BAD TEXT', lang, r, (h.match(/.{0,30}(undefined|NaN|\[object).{0,30}/) || [''])[0]); }
  }
}
// click-driven dialogs that have no preset route
function click(act, v, id) { els.stage.listeners.click[0]({ target: { closest: () => ({ getBoundingClientRect: () => ({ left: 0, top: 0, right: 100, bottom: 40 }), getAttribute: k => ({ 'data-act': act, 'data-v': v, 'data-id': id })[k] || null }) }, stopPropagation() {}, preventDefault() {} }); }
location.hash = '#/trash'; winL.hashchange();
[['dlg', 'emptytrash'], ['dlg', 'delforever', 't1'], ['dodel', null, 't1'], ['restore', null, 't2']].forEach(a => { try { click(...a); if (/undefined|NaN/.test(els.stage._html)) { fails++; console.log('BAD', a); } } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/shared'; winL.hashchange();
[['dlg', 'revoke', 's1'], ['dorevoke', null, 's1']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/shared?tab=with'; winL.hashchange();
[['sharetab', 'by'], ['sharetab', 'with']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/settings?state=phone'; winL.hashchange();
[['cpopen'], ['cppick', 'TW'], ['sendphone'], ['cpopen'], ['cppick', 'JP'], ['sendphone'], ['closedlg']].forEach(a => { try { click(...a); if (/undefined|NaN|\[object/.test(els.stage._html)) { fails++; console.log('BAD', a); } } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/register?mode=phone'; winL.hashchange();
[['regmode', 'email'], ['regmode', 'phone'], ['cpopen'], ['cppick', 'HK']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/files?state=share-people'; winL.hashchange();
[['who', '0'], ['who', '1'], ['aud', 'link'], ['aud', 'people'], ['mklink'], ['closedlg']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/files?state=share'; winL.hashchange();
[['exp', 'never'], ['dltoggle'], ['mklink'], ['closedlg']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/files?state=selection'; winL.hashchange();
[['dlg', 'download'], ['startdl'], ['closedlg'], ['trash'], ['uploadmenu'], ['startup'], ['upmin'], ['sortmenu'], ['view', 'list'], ['sort', 'size']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
location.hash = '#/admin?u=available'; winL.hashchange();
[['admin', 'checking'], ['mode', 'auto'], ['approve']].forEach(a => { try { click(...a); } catch (e) { fails++; console.log('FAIL click', a, e.message); } });
const missing = [...used].filter(k => !(k in rawS) && !/^(then|toJSON|Symbol.*)$/.test(k));
console.log('rendered', n, 'fails', fails, 'missing keys', missing);
for (const k in rawS) if (rawS[k].length !== 3 || rawS[k].some(s => !s)) { fails++; console.log('BAD ENTRY', k); }
const unusedKeys = Object.keys(rawS).filter(k => !used.has(k));
console.log('unused keys', unusedKeys.length, unusedKeys.join(' '));
process.exit(fails || missing.length ? 1 : 0);
