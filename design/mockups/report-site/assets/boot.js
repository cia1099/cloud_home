/* Runs in <head> before first paint: resolves language, appearance and data state, no flash.
   Review overrides: ?lang=en|zh  ?theme=light|dark|auto  ?state=real|sample
   state=real   numbers and events from the project's true state on 24 Nov 2026 (no web UI yet)
   state=sample populated demo numbers, always labelled as sample */
(function () {
  var d = document.documentElement, s = {}, q = new URLSearchParams(location.search), ss = {};
  try { s = localStorage; ss = sessionStorage; } catch (e) {}
  var l = q.get('lang') || s.chL || (/^zh/i.test(navigator.language) ? 'zh' : 'en');
  if (l !== 'zh') l = 'en';
  d.dataset.l = l;
  d.lang = l === 'zh' ? 'zh-Hans' : 'en';
  var t = q.get('theme') || s.chT || 'auto';
  d.dataset.tp = t;
  d.dataset.theme = t === 'auto' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : t;
  var st = q.get('state') || ss.chS || 'real';
  if (st !== 'sample') st = 'real';
  try { ss.chS = st; } catch (e) {}
  d.dataset.state = st;
  d.classList.add('js');
})();
