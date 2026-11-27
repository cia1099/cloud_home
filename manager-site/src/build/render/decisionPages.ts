import { ic, LOGO } from '../../shared/icons.ts';
import type { Assets } from '../assets.ts';
import { callout, decisionCard, historyItem } from './components.ts';
import { alBi, T } from './html.ts';
import type { Rendered } from './pages.ts';
import type { Site } from '../site.ts';
import { repoPath } from '../util.ts';

const back = `<p class="small" style="margin:0"><a href="../">${T('‹ All decisions', '‹ 全部决策')}</a></p>`;

const head = (title: string, lede: string): string =>
  `<header style="padding-block:40px 28px;max-width:720px">${back}<h1 class="display" style="margin-top:16px">${title}</h1><p class="lede" style="margin-top:12px">${lede}</p></header>`;

/** The owner's answer: the same card as on /decisions/ (buttons included) while pending, the recorded result afterwards. */
function answer(site: Site, id: string, intro: string): string {
  const a = site.approvals.find((x) => x.id === id);
  if (!a) return '';
  const body = a.state === 'pending' ? decisionCard(a, '../../', false) : `<div class="card" style="padding:8px 24px"><ul class="hist">${historyItem(a)}</ul></div>`;
  return `<section class="sec" id="answer"><div class="sec-h"><h2 class="title">${T('Your answer', '你的回答')}</h2></div>${intro}${body}</section>`;
}

// ---------- A-003: cloud icon variants ----------

interface Variant {
  key: 'a' | 'b';
  name: string;
  idea: [string, string];
  rationale: [string, string];
  use: [string, string];
  /** Files in design/brand/ for the light stage, the dark stage and the 16/32px sizes. */
  light: string;
  dark: string;
  small: string;
}

const VARIANTS: Variant[] = [
  {
    key: 'a',
    name: 'Sky',
    idea: ['Your cloud on a blue tile, finished like a system icon.', '你喜欢的那朵云，蓝色底，按系统图标的质感完成。'],
    rationale: [
      'Same tile and same white cloud as today. What changes is the finish: a continuous-curvature corner instead of a plain rounded rectangle, a faint gradient inside the cloud, a soft blue shade along its lower edge, one gentle shadow and a hint of light from the top left. At 16 and 32px a simplified file is used, with a slightly larger cloud and no effects, so it stays crisp. It keeps the white-on-blue identity you said you like, and it reads the same on light and dark backgrounds.',
      '底和云都和现在一样，改变的是质感：用连续曲率的圆角取代普通圆角矩形，云朵内部有极淡的渐变，下缘有一抹柔和的蓝色阴影，加上一道轻柔的投影和左上方的一点光。16px 与 32px 使用简化版本：云稍大、不加特效，保证清晰。它保留了你喜欢的“蓝底白云”，在浅色和深色背景上看起来一样。',
    ],
    use: ['Header logo, favicon, product app icon', '页眉标志、网站图标、产品应用图标'],
    light: 'cloud-a',
    dark: 'cloud-a',
    small: 'cloud-a-favicon',
  },
  {
    key: 'b',
    name: 'Porcelain',
    idea: ['A pale tile with a blue cloud; graphite tile in dark mode.', '浅色底配蓝色云；深色模式下换成石墨色底。'],
    rationale: [
      'The inverse: the tile is pale (graphite in dark mode) and the cloud carries the colour, as a blue gradient with a soft highlight at the top. It is quieter and looks more like a built-in system app, but it is less recognisable as the icon you know, and at 16px a blue cloud on a pale tile has less contrast than white on blue. Because the tile changes with the theme, it has two files.',
      '反过来：底是浅色（深色模式下是石墨色），颜色由云承担，是带顶部柔光的蓝色渐变。它更安静，更像系统自带的应用，但不如你熟悉的那个图标好认；16px 时，浅底上的蓝云对比度也不如蓝底白云。因为底色随主题变化，所以有两个文件。',
    ],
    use: ['Sign-in screen, quiet contexts', '登录界面等较安静的场合'],
    light: 'cloud-b',
    dark: 'cloud-b-dark',
    small: '',
  },
];

function variantCard(v: Variant, assets: Assets): string {
  const L = v.key.toUpperCase();
  const src = (name: string): string => `../../${assets.file(repoPath('design/brand', `${name}.svg`))}`;
  const img = (name: string, s: number): string => `<img src="${src(name)}" width="${s}" height="${s}" alt="" loading="lazy" decoding="async">`;
  const stage = (dark: boolean): string => {
    const file = dark ? v.dark : v.light;
    const small = v.small || file;
    const label = dark ? T('Dark', '深色') : T('Light', '浅色');
    const sizes = [16, 32, 64, 180].map((s) => `<figure>${img(s <= 32 ? small : file, s)}<figcaption>${s}px</figcaption></figure>`).join('');
    return `<div class="stage${dark ? ' dark' : ''}"><span class="stage-l">${label}</span>
<div class="mk-row" role="group" ${alBi({ en: `Variant ${L}, ${dark ? 'dark' : 'light'}: the icon at 16, 32, 64 and 180 pixels`, zh: `方案 ${L}${dark ? '深色' : '浅色'}：16、32、64、180 像素的图标` })}>${sizes}</div>
<div class="mk-row"><figure class="appicon">${img(file, 96)}<figcaption>${T('App icon', '应用图标')}</figcaption></figure></div></div>`;
  };
  return `<article class="card dir" id="variant-${v.key}"><div class="dir-h"><span class="dir-l" aria-hidden="true">${L}</span><div><h3>${T(`Variant ${L}: ${v.name}`, `方案 ${L}：${v.name}`)}</h3><p class="sub" style="margin:2px 0 0">${T(v.idea[0], v.idea[1])}</p></div>${v.key === 'a' ? `<span class="chip ok" style="margin-left:auto">${ic('check')}${T('Designer’s recommendation', '设计师推荐')}</span>` : ''}</div>
<div class="stages">${stage(false)}${stage(true)}</div>
<p style="margin:20px 0 0;text-wrap:pretty">${T(v.rationale[0], v.rationale[1])}</p>
<dl class="dl" style="margin-top:16px"><dt>${T('Best for', '适用于')}</dt><dd>${T(v.use[0], v.use[1])}</dd></dl></article>`;
}

/** Before and after: the header logo as it is today next to the recommended refinement, at the sizes the header and the favicon use. */
function beforeAfter(assets: Assets): string {
  const after = (s: number): string => `<img src="../../${assets.file(repoPath('design/brand', s <= 32 ? 'cloud-a-favicon.svg' : 'cloud-a.svg'))}" width="${s}" height="${s}" alt="" loading="lazy" decoding="async">`;
  const row = (inner: (s: number) => string): string => [28, 64, 128].map((s) => `<figure>${inner(s)}<figcaption>${s}px</figcaption></figure>`).join('');
  const now = (s: number): string => `<span class="logo-now" style="width:${s}px;height:${s}px">${LOGO}</span>`;
  const pair = (dark: boolean): string =>
    `<div class="stage${dark ? ' dark' : ''}"><span class="stage-l">${dark ? T('Dark', '深色') : T('Light', '浅色')}</span><div class="mk-row"><div class="ba"><b class="ba-t">${T('Today', '现在')}</b><div class="mk-row">${row(now)}</div></div><div class="ba"><b class="ba-t">${T('Variant A', '方案 A')}</b><div class="mk-row">${row(after)}</div></div></div></div>`;
  return `<section class="sec"><div class="sec-h"><h2 class="title">${T('Before and after', '改前与改后')}</h2></div><div class="stages">${pair(false)}${pair(true)}</div></section>`;
}

export function brandPage(site: Site, assets: Assets): Rendered {
  const intro = `<p class="sub" style="margin:0 0 16px;max-width:640px">${T('Approve: variant A (Sky). Deny: variant B (Porcelain). If you want something else, say so in the issue body and the designer will adjust.', '批准：选方案 A（Sky）。拒绝：选方案 B（Porcelain）。如果两个都不合适，在议题正文里写明想怎么改，设计师会调整。')}</p>`;
  const main = `<main id="main" class="wrap">${head(T('Your cloud icon, refined', '把你的云图标打磨一下'), T('You asked for the report site’s cloud to feel more like Apple design and more elegant. Here are two refinements, each at the sizes it will really be used, in light and dark.', '你希望报告网站的云图标更有 Apple 的味道、更优雅。这里是两个打磨方案，按实际使用的尺寸展示，浅色与深色各一份。'))}
<div style="max-width:720px">${callout('info', { en: 'The designer recommends A, Sky', zh: '设计师推荐 A：Sky' }, { en: 'It is the icon you said you like, with a more refined finish. B is quieter but less recognisable. Nothing is applied to the product until you choose; the report site keeps today’s logo meanwhile.', zh: '这就是你说喜欢的那个图标，只是质感更讲究。B 更安静，但辨识度较低。你选定之前，产品里不会改动任何地方；报告网站暂时仍用现在的标志。' })}</div>
${beforeAfter(assets)}
<div class="stack" style="margin-top:32px">${VARIANTS.map((v) => variantCard(v, assets)).join('')}</div>
<p class="small" style="margin-top:16px;max-width:640px">${T('The tiles here are the site-logo version. For a system app icon the artwork is delivered as a full square and the system rounds it. Product name stays “Cloud Home” in every language.', '这里的底是网站标志版本。系统应用图标会提供完整的方形图稿，由系统自己裁圆角。产品名在所有语言里都是 “Cloud Home”。')}</p>
${answer(site, 'A-003', intro)}</main>`;
  return { page: { path: '/decisions/brand/', nav: 'decisions', title: { en: 'Cloud icon', zh: '云图标' } }, main };
}

// ---------- A-002: one admin or several ----------

const node = (icon: 'phone' | 'server' | 'heart', label: string, sub = ''): string => `<div class="dg-n"><span class="k-ic">${ic(icon)}</span><b>${label}</b>${sub ? `<span class="small">${sub}</span>` : ''}</div>`;
const arrow = `<span class="dg-a" aria-hidden="true"></span>`;

function diagram(): string {
  const reg = node('heart', T('A new person signs up', '有人注册账号'));
  const box = node('server', 'Cloud Home', T('Holds the request as “pending”', '把请求记为“待审批”'));
  const one = `<figure class="dg" ${alBi({ en: 'Diagram: with one admin, the request goes to one phone', zh: '示意图：只有一位管理员时，请求发到一部手机' })}><figcaption><b>${T('One admin', '一位管理员')}</b></figcaption>${reg}${arrow}${box}${arrow}<div class="dg-row">${node('phone', T('Admin', '管理员'), T('Gets the message', '收到消息'))}</div></figure>`;
  const many = `<figure class="dg" ${alBi({ en: 'Diagram: with several admins, the same request goes to every phone and the first tap wins', zh: '示意图：有多位管理员时，同一个请求发到每部手机，先点的算数' })}><figcaption><b>${T('Several admins', '多位管理员')}</b></figcaption>${reg}${arrow}${box}${arrow}<div class="dg-row">${node('phone', T('Admin 1', '管理员 1'), T('Gets the message', '收到消息'))}${node('phone', T('Admin 2', '管理员 2'), T('Gets the same message', '收到同一条消息'))}</div><p class="small dg-n2">${T('The first tap decides. The other phone is told “already decided”.', '谁先点谁说了算，另一部手机会看到“已处理”。')}</p></figure>`;
  return `<div class="grid g2" style="margin-top:8px">${one}${many}</div>`;
}

const bubble = (who: 'bot' | 'me', text: string, extra = ''): string => `<div class="msg ${who}"><div class="bub">${text}${extra}</div></div>`;
const btns = (a: string, r: string, off = false): string => `<div class="bbar${off ? ' off' : ''}" aria-hidden="true"><span>${a}</span><span>${r}</span></div>`;

function chat(): string {
  const req = (decided: boolean): string =>
    bubble(
      'bot',
      `<b>${T('New sign-up request', '新的注册申请')}</b><br>${T('Name: Jamie<br>Account: m***@e***.com<br>Time: 14:02 · from 192.0.2.44', '名称：Jamie<br>账号：m***@e***.com<br>时间：14:02 · 来自 192.0.2.44')}`,
      decided ? `<hr><span>${T('✅ Approved by Alex at 14:03', '✅ 已由 Alex 于 14:03 批准')}</span>` : btns(T('Approve', '批准'), T('Reject', '拒绝')),
    );
  const phone = (who: string, body: string): string => `<figure class="phone-chat"><figcaption>${who}</figcaption><div class="pc-body">${body}</div></figure>`;
  const alex = phone(T('Alex’s phone', 'Alex 的手机'), `${req(true)}${bubble('me', T('Tapped Approve', '点了“批准”'))}`);
  const sam = phone(T('Sam’s phone, a moment later', 'Sam 的手机，稍后'), `${req(true)}${bubble('me', T('Tapped Reject', '点了“拒绝”'))}${bubble('bot', T('Already decided by Alex at 14:03. Nothing changed.', '已由 Alex 于 14:03 处理，没有任何改动。'))}`);
  return `<div class="grid g2">${alex}${sam}</div><p class="small" style="margin:12px 0 0">${T('A made-up example. Real messages hide most of the account name, and the bot speaks the language set for it (English, 简体中文 or 繁體中文).', '示例为虚构。真实消息会隐去账号的大部分字符，机器人使用设置好的语言（英文、简体中文或繁体中文）。')}</p>`;
}

const qa = (q: string, a: string): string => `<section class="card qa"><h3 class="h3">${q}</h3><div class="qa-a">${a}</div></section>`;
const p = (s: string): string => `<p>${s}</p>`;

export function adminsPage(site: Site): Rendered {
  const intro = `<p class="sub" style="margin:0 0 16px;max-width:640px">${T('Approve: several equal admins are allowed (pairing stays one code at a time). Deny: strictly one admin, and a second pairing is refused. Until you answer, whoever pairs becomes an admin.', '批准：允许多位地位相同的管理员（配对仍是一次一个码）。拒绝：严格只有一位管理员，第二次配对会被拒绝。你回答之前，谁先配对谁就是管理员。')}</p>`;
  const pairing = `<ol class="steps card" style="padding:22px 24px;margin:0;list-style:none"><li><b>${T('Ask for a code', '申请配对码')}</b><span>${T('On the home network, call the private admin endpoint with the admin password. You get an 8-character code.', '在家里的网络中，用管理员密码调用私有的管理接口，得到一个 8 位配对码。')}</span></li><li><b>${T('Send it to the bot', '发给机器人')}</b><span>${T('In a private chat, the future admin sends “/pair” and the code. It works once and expires after 10 minutes.', '未来的管理员在私聊里发送“/pair”和这个码。只能用一次，10 分钟后失效。')}</span></li><li><b>${T('Done', '完成')}</b><span>${T('The phone is now an admin. Anyone else who writes to the bot is silently ignored.', '这部手机成为管理员。其他任何人给机器人发消息，都会被悄悄忽略。')}</span></li></ol>`;
  const main = `<main id="main" class="wrap">${head(T('One administrator or several?', '一位管理员，还是多位？'), T('When someone signs up for an account, an admin has to say yes. This page explains who gets asked, and what changes if more than one person can.', '有人注册账号时，需要管理员点头。这一页解释：会问谁，以及允许多人时有什么不同。'))}
<div style="max-width:720px">${callout('info', { en: 'Recommendation: allow several, pair one for now', zh: '建议：允许多位，现在只配对一位' }, { en: 'It costs nothing and lets you add a family member later without changing anything. You stay the only admin until you decide otherwise.', zh: '不增加任何成本，以后想加一位家人也无需改动任何东西。你不另做决定，就一直是唯一的管理员。' })}</div>
<section class="sec"><div class="sec-h"><h2 class="title">${T('How it looks', '看一眼流程')}</h2></div>${diagram()}</section>
<section class="sec"><div class="sec-h"><h2 class="title">${T('An example conversation', '聊天示例')}</h2></div>${chat()}</section>
<section class="sec"><div class="sec-h"><h2 class="title">${T('Questions you may have', '你可能想问的')}</h2></div><div class="stack" style="max-width:720px">${[
    qa(T('Who gets the Approve / Reject message?', '谁会收到“批准 / 拒绝”的消息？'), p(T('Every paired admin, in their own private chat with the bot. With one admin that is just you; with several, each of them gets the same message with the same two buttons.', '每位已配对的管理员，各自在与机器人的私聊里收到。只有一位时就是你自己；有多位时，每人收到同样的消息和同样的两个按钮。'))),
    qa(T('What if two admins tap at the same moment?', '两位管理员同时点了怎么办？'), p(T('The first tap wins and the account is decided. The other admin sees “already decided by …” with the name and time, and nothing is changed twice. The original message is edited to show the result and loses its buttons.', '先点的那一下算数，账号随即有了结果。另一位会看到“已由 … 处理”，带着名字和时间，不会重复执行。原消息会被改成结果的样子，按钮消失。'))),
    qa(T('What if the admin loses their phone?', '管理员丢了手机怎么办？'), p(T('Remove the lost phone from the admin list and pair a new one. Both steps use the private admin endpoint, which only answers on the home network and only with the admin password, so it has to be done from home. With several admins, the others keep working meanwhile; with one admin, new requests wait (they are also listed on the private endpoint) until you have paired again.', '把丢失的手机从管理员名单里移除，再配对一部新的。这两步都要用私有的管理接口，它只在家里的网络里应答，并且需要管理员密码，所以必须在家里操作。有多位管理员时，其他人可以继续处理；只有一位时，新的请求会等着（私有接口里也能看到列表），直到你重新配对。'))),
    qa(T('Do all admins have the same power?', '所有管理员的权力一样吗？'), p(T('Yes. There is no “main” admin: each one can approve or reject any request, and every decision is written to the audit log with who made it. An admin can be unpaired from the private endpoint.', '一样。没有“主管理员”：每位都能批准或拒绝任何请求，每个决定都会连同决定人写入审计日志。管理员可以从私有接口解除配对。'))),
    qa(T('How does someone become an admin?', '怎样成为管理员？'), pairing),
  ].join('')}</div></section>
<section class="sec"><div class="sec-h"><h2 class="title">${T('Why we recommend several', '为什么建议允许多位')}</h2></div><div class="card" style="max-width:720px"><ul class="why"><li>${T('<b>Nothing to give up.</b> With a single paired phone it behaves exactly like one admin.', '<b>没有任何损失。</b>只配对一部手机时，效果和只有一位管理员完全一样。')}</li><li>${T('<b>A safety net.</b> A second person can approve requests while the first is away or has lost a phone.', '<b>多一层保险。</b>第一位不在或丢了手机时，第二个人还能审批。')}</li><li>${T('<b>Still safe.</b> Pairing always needs the private endpoint and the admin password, so nobody can add themselves.', '<b>依然安全。</b>配对始终需要私有接口和管理员密码，没有人能自己加入。')}</li><li>${T('<b>The cost of “one only”</b> is a lost phone locking out approvals until you re-pair from home.', '<b>“只允许一位”的代价：</b>丢了手机就没法审批，直到你在家里重新配对。')}</li></ul></div></section>
${answer(site, 'A-002', intro)}</main>`;
  return { page: { path: '/decisions/admins/', nav: 'decisions', title: { en: 'One admin or several?', zh: '一位还是多位管理员？' } }, main };
}
