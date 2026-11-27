/* Cloud Home app icons: original line icons, 24px grid, 1.75px stroke, round caps/joins, currentColor.
   No SF Symbols, no Apple glyphs. Also: generated abstract artwork used as photo thumbnails. */
(function () {
  var P = {
    folder: '<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.2h7.5A2.5 2.5 0 0 1 21 9.7v7.8a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5z"/>',
    file: '<path d="M7 3h6.5L19 8.5V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M13.5 3v5.5H19"/>',
    doc: '<path d="M7 3h6.5L19 8.5V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M13.5 3v5.5H19M8.5 13h7M8.5 16.5h5"/>',
    sheet: '<path d="M7 3h6.5L19 8.5V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M13.5 3v5.5H19M8.5 13h7M8.5 16.5h7M12 13v3.5"/>',
    zip: '<path d="M7 3h6.5L19 8.5V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M13.5 3v5.5H19M10.5 8h1.5M10.5 11h1.5M10.5 14h1.5"/><rect x="9.5" y="15.5" width="3.5" height="3" rx="1"/>',
    image: '<rect x="3.5" y="4.5" width="17" height="15" rx="3"/><circle cx="9" cy="10" r="1.6"/><path d="M4 17l5-4.5 3.5 3 2.5-2.5 5 4.5"/>',
    music: '<path d="M9 18V6.5l10-2V16"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="16.5" cy="16" r="2.5"/>',
    video: '<rect x="3" y="5.5" width="13.5" height="13" rx="3"/><path d="M16.5 10.5L21 8v8l-4.5-2.5z"/>',
    pdf: '<path d="M7 3h6.5L19 8.5V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M13.5 3v5.5H19M8.5 17.5c1.5-1 2.5-3 3-5.5.5 2 1.7 3.6 3.5 4.3"/>',
    grid: '<rect x="4" y="4" width="6.5" height="6.5" rx="1.8"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.8"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.8"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.8"/>',
    list: '<path d="M9 6.5h11M9 12h11M9 17.5h11"/><circle cx="4.8" cy="6.5" r=".6"/><circle cx="4.8" cy="12" r=".6"/><circle cx="4.8" cy="17.5" r=".6"/>',
    search: '<circle cx="10.5" cy="10.5" r="6"/><path d="M15 15l5 5"/>',
    upload: '<path d="M12 15.5V4M7.5 8.5L12 4l4.5 4.5M5 14v3.5A2.5 2.5 0 0 0 7.5 20h9a2.5 2.5 0 0 0 2.5-2.5V14"/>',
    download: '<path d="M12 4v11.5M7.5 11L12 15.5 16.5 11M5 14v3.5A2.5 2.5 0 0 0 7.5 20h9a2.5 2.5 0 0 0 2.5-2.5V14"/>',
    trash: '<path d="M4.5 7h15M9.5 7V5a1.5 1.5 0 0 1 1.5-1.5h2A1.5 1.5 0 0 1 14.5 5v2M6.5 7l.8 11.2A2 2 0 0 0 9.3 20h5.4a2 2 0 0 0 2-1.8L17.5 7M10 11v5M14 11v5"/>',
    share: '<path d="M12 14.5V4M8 7.5L12 3.5l4 4M7 10.5H6.5A1.5 1.5 0 0 0 5 12v6.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V12a1.5 1.5 0 0 0-1.5-1.5H17"/>',
    link: '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3A4 4 0 0 0 11 18.7l1-1"/>',
    sliders: '<path d="M4 7h9M17 7h3M4 12h3M11 12h9M4 17h11M19 17h1"/><circle cx="15" cy="7" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="17" r="2"/>',
    user: '<circle cx="12" cy="8.5" r="3.7"/><path d="M4.5 20c.6-3.8 3.6-6 7.5-6s6.9 2.2 7.5 6"/>',
    users: '<circle cx="9.5" cy="8.5" r="3.3"/><path d="M3.5 19.5c.5-3.3 3-5.2 6-5.2s5.5 1.9 6 5.2M16 5.4a3.3 3.3 0 0 1 0 6.2M17.5 14.6c2 .5 3.4 2.2 3.8 4.9"/>',
    cloud: '<path d="M7.5 18.5a4.5 4.5 0 0 1-.6-8.96A5.6 5.6 0 0 1 17.7 9a4.8 4.8 0 0 1-.7 9.5z"/>',
    check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
    x: '<path d="M6 6l12 12M18 6L6 18"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    chevR: '<path d="M9.5 5.5L16 12l-6.5 6.5"/>',
    chevL: '<path d="M14.5 5.5L8 12l6.5 6.5"/>',
    chevD: '<path d="M5.5 9.5L12 16l6.5-6.5"/>',
    more: '<circle cx="5.5" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="18.5" cy="12" r="1.3"/>',
    restore: '<path d="M4.5 12a7.5 7.5 0 1 0 2.4-5.5M4.5 5v4.5H9"/>',
    clock: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
    shield: '<path d="M12 3.5l7 2.6v5.4c0 4.4-2.9 7.4-7 9-4.1-1.6-7-4.6-7-9V6.1z"/><path d="M9 12l2.2 2.2L15.2 10"/>',
    alert: '<path d="M12 4l9 15.5H3z"/><path d="M12 10v4.2M12 17v.01"/>',
    wifioff: '<path d="M3.5 9.5a13 13 0 0 1 4-2.4M20.5 9.5a13 13 0 0 0-8.5-3.4M6.5 13a8 8 0 0 1 2.7-1.8M17.5 13a8 8 0 0 0-3.4-1.9M9.5 16.3a4 4 0 0 1 5 0M12 19.5v.01M4 4l16 16"/>',
    refresh: '<path d="M19.5 12a7.5 7.5 0 0 1-13 5.1M4.5 12a7.5 7.5 0 0 1 13-5.1M17.5 3.5v4h-4M6.5 20.5v-4h4"/>',
    play: '<path d="M8 5.5v13l11-6.5z" fill="currentColor"/>',
    pause: '<rect x="6.5" y="5" width="3.8" height="14" rx="1.2" fill="currentColor" stroke="none"/><rect x="13.7" y="5" width="3.8" height="14" rx="1.2" fill="currentColor" stroke="none"/>',
    volume: '<path d="M4 9.5h3l4.5-3.5v12L7 14.5H4z"/><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11"/>',
    info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8v.01"/>',
    lock: '<rect x="5" y="10.5" width="14" height="9.5" rx="2.5"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5"/>',
    mail: '<rect x="3.5" y="5.5" width="17" height="13" rx="3"/><path d="M4.5 8l7.5 5.5L19.5 8"/>',
    phone: '<rect x="7" y="3" width="10" height="18" rx="2.8"/><path d="M11 18h2"/>',
    drive: '<rect x="3.5" y="8" width="17" height="9" rx="3"/><path d="M7 12.5h.01M10 12.5h4"/><path d="M6 8l1.5-3.5h9L18 8"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6L7 7M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4"/>',
    moon: '<path d="M19.5 14.5A7.5 7.5 0 0 1 9.5 4.5a7.5 7.5 0 1 0 10 10z"/>',
    globe: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.5 2.6 3.5 5.4 3.5 8.5s-1 5.9-3.5 8.5c-2.5-2.6-3.5-5.4-3.5-8.5s1-5.9 3.5-8.5z"/>',
    sort: '<path d="M4 7h16M4 12h10M4 17h5"/>',
    move: '<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.2h7.5A2.5 2.5 0 0 1 21 9.7V11"/><path d="M3 7.5v10A2.5 2.5 0 0 0 5.5 20H12M15 16.5h6M18.5 13.5l3 3-3 3"/>',
    copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="2.5"/><path d="M15.5 8.5V6A2.5 2.5 0 0 0 13 3.5H6A2.5 2.5 0 0 0 3.5 6v7A2.5 2.5 0 0 0 6 15.5h2.5"/>',
    edit: '<path d="M4.5 19.5l1-4.2L16.3 4.5a2 2 0 0 1 2.8 0l.4.4a2 2 0 0 1 0 2.8L8.7 18.5z"/><path d="M14.5 6.5l3 3"/>',
    expand: '<path d="M14 4.5h5.5V10M10 19.5H4.5V14M19.5 4.5L13.5 10.5M4.5 19.5l6-6"/>',
    skipB: '<path d="M7 5.5v13M18 6v12l-8.5-6z" />',
    skipF: '<path d="M17 5.5v13M6 6v12l8.5-6z" />',
    up: '<circle cx="12" cy="12" r="8.5"/><path d="M12 16.5V8M8.5 11.5L12 8l3.5 3.5"/>',
    key: '<circle cx="8" cy="12" r="3.5"/><path d="M11.5 12H20M17 12v3M14.5 12v2"/>',
    star: '<path d="M12 4l2.4 5 5.4.7-4 3.7 1 5.4L12 16.1 7.2 18.8l1-5.4-4-3.7 5.4-.7z"/>',
    spinner: '<path d="M12 3.5a8.5 8.5 0 1 1-8.5 8.5" />'
  };
  window.ICON = function (name, size, cls) {
    size = size || 20;
    return '<svg class="ic' + (cls ? ' ' + cls : '') + '" width="' + size + '" height="' + size + '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">' + (P[name] || P.file) + '</svg>';
  };

  /* Original generated artwork for photo thumbnails: layered hills, a sun/moon, soft gradients. */
  var PAL = [
    ['#ffd9a8', '#ff9e7a', '#7b5ea7', '#3b3a6b'],
    ['#bde4ff', '#6db3f2', '#2e6fb7', '#173f73'],
    ['#d7f0c8', '#8fcf8a', '#3f9a6a', '#1d5a4a'],
    ['#ffe8f0', '#f6a6c1', '#b45fa6', '#4d2f78'],
    ['#fff2c2', '#f7c85a', '#e08a3c', '#8a3f2c'],
    ['#cfe3f7', '#93b4de', '#5a6fa8', '#2a2f63']
  ];
  window.ART = function (i, label) {
    var c = PAL[i % PAL.length], id = 'a' + i + Math.random().toString(36).slice(2, 6);
    return '<svg class="art" viewBox="0 0 160 120" preserveAspectRatio="xMidYMid slice" aria-hidden="true">' +
      '<defs><linearGradient id="' + id + '" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="' + c[0] + '"/><stop offset="1" stop-color="' + c[1] + '"/></linearGradient></defs>' +
      '<rect width="160" height="120" fill="url(#' + id + ')"/>' +
      '<circle cx="' + (40 + (i * 23) % 80) + '" cy="' + (34 + (i * 7) % 16) + '" r="14" fill="#fff" opacity=".78"/>' +
      '<path d="M0 84C26 62 48 66 74 78s48 0 86-22v64H0z" fill="' + c[2] + '" opacity=".85"/>' +
      '<path d="M0 100c30-18 56-12 84 0s48 6 76-8v28H0z" fill="' + c[3] + '"/></svg>';
  };
})();
