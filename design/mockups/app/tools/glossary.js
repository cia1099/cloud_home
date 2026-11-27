// Prints glossary section E (the app string table) as Markdown, generated from assets/i18n.js.
// Run: bun design/mockups/app/tools/glossary.js   and replace section E of design/glossary.md with the output.
const fs = require('fs');
const path = require('path');
global.window = global;
(0, eval)(fs.readFileSync(path.join(__dirname, '..', 'assets', 'i18n.js'), 'utf8'));
const esc = s => s.replace(/\|/g, '\\|');
const out = [];
out.push('## E. Web app: complete string table (R-002)', '');
out.push('Generated from `design/mockups/app/assets/i18n.js` by `bun design/mockups/app/tools/glossary.js`: edit the .js file and regenerate, do not hand-edit this table. Placeholders `{n}`, `{name}`, `{folder}`, `{date}`, `{size}` are filled by the app; dates, sizes and numbers go through `Intl` (never concatenate). Keys starting with `a11y.` are for screen readers only.', '');
out.push('Wording decisions (Traditional is native Taiwan usage, not converted):', '');
out.push('| Concept | English | 简体中文 | 繁體中文 (Taiwan) | Why |', '|---|---|---|---|---|');
[
  ['Approve (admin)', 'Approve', '批准', '核准', '核准 is the usual Taiwan administrative term'],
  ['Pre-release', 'Pre-release', '预发布版', '預先發行版', ''],
  ['Release notes', 'Release notes', '发行说明', '版本資訊', 'Taiwan software says 版本資訊 or 版本說明'],
  ['Sign-in methods', 'Ways to sign in', '登录方式', '登入方式', '登入 is the Taiwan verb'],
  ['Documents (kind)', 'Documents', '文档', '文件', 'in Taiwan 文件 is a document; a file is 檔案'],
  ['Video, audio', 'Videos, Audio', '视频，音频', '影片，音訊', ''],
  ['Network', 'no connection', '无网络连接', '沒有網路連線', '網路, not 网络'],
  ['Link', 'Link', '链接', '連結', ''],
  ['Selected', 'selected', '已选择', '已選取', '選取 for selection'],
  ['Software', 'Software Update', '软件更新', '軟體更新', ''],
  ['Storage', 'Storage', '存储空间', '儲存空間', ''],
  ['Disk', 'Drive', '硬盘', '硬碟', ''],
  ['Text message', 'text message', '短信', '簡訊', '簡訊 is the Taiwan term; 短信 is mainland'],
  ['Shared with you', 'Shared with You', '分享给你', '與你分享', 'Taiwan phrasing is 與你分享 (not 分享給你)'],
  ['Mainland China', 'Mainland China', '中国大陆', '中國大陸', 'country list; 臺灣 is written with 臺 in Taiwan official usage'],
  ['South Korea', 'South Korea', '韩国', '南韓', 'Taiwan says 南韓'],
  ['Australia', 'Australia', '澳大利亚', '澳洲', 'Taiwan says 澳洲'],
  ['Quotes', '“ ”', '“ ”', '「 」', 'corner quotes in Traditional']
].forEach(r => out.push('| ' + r.join(' | ') + ' |'));
out.push('', '| Key | English | 简体中文 | 繁體中文 (Taiwan) |', '|---|---|---|---|');
for (const k of Object.keys(window.S)) out.push('| ' + k + ' | ' + window.S[k].map(esc).join(' | ') + ' |');
out.push('', 'Longest strings to test at 390 px in each language: `approval.rejected_body`, `approval.submitted_body`, `renamed_as`, `phase.failed_text`, `confirm_empty_text`, `download_targz_hint`, `ch_hint`, `upload_paused_offline`, `no_results_text`.');
if (process.argv.includes('--write')) {
  const g = path.join(__dirname, '..', '..', '..', 'glossary.md');
  const cur = fs.readFileSync(g, 'utf8'), i = cur.indexOf('## E. Web app');
  fs.writeFileSync(g, (i >= 0 ? cur.slice(0, i) : cur.replace(/\s*$/, '\n\n---\n\n')) + out.join('\n') + '\n');
} else console.log(out.join('\n'));
