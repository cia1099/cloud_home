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

// ---------- A-004: cloud icon variants C and D ----------

interface Variant {
  /** Files in design/brand/: `cloud-<key>.svg` (light), `cloud-<key>-dark.svg`, `cloud-<key>-favicon.svg` (16/32px). */
  key: 'c' | 'd';
  name: string;
  idea: [string, string];
  rationale: [string, string];
  use: [string, string];
}

const VARIANTS: Variant[] = [
  {
    key: 'c',
    name: 'Hearth',
    idea: ['A house drawn inside the cloud: the drive is the cloud, home is what it holds.', '云里画着一座房子：云是网盘，家是它装着的东西。'],
    rationale: [
      'An outline cloud with a small house inside it, like the cloud and house symbols composed into one. It is the more explicit picture of “a cloud drive at home”. The house needs about 48px to resolve, so it shines at large sizes and gives way to a simplified solid version at 16 and 32px.',
      '云的轮廓里有一座小房子，就像把“云”和“房子”两个符号合成一个。它把“家里的网盘”画得最直白。房子大约要 48px 才看得清，所以大尺寸最好看；16px 与 32px 改用简化的实心版本。',
    ],
    use: ['Large sizes: header logo, sign-in screen, social image', '大尺寸：页眉标志、登录界面、分享图'],
  },
  {
    key: 'd',
    name: 'Roofline',
    idea: ['A cloud wearing a roof, with a doorway cut into its base.', '给云戴上屋顶，底部开一扇门。'],
    rationale: [
      'A bolder silhouette: a roof over a solid cloud, and an arched doorway cut through its base. A cloud wearing a roof is a home. It reads better at 16px, where the roof and door still come through as one shape, so it is the stronger choice for the browser tab and small icons. It is also the best single mark if one icon must serve every size.',
      '更粗壮的轮廓：实心的云上加一个屋顶，云的底部镂出一扇拱门。戴着屋顶的云就是一个家。它在 16px 时依然清楚，屋顶和门还是一个整体，所以更适合浏览器标签页和小图标。如果只能用一个图标应付所有尺寸，它也是最稳妥的。',
    ],
    use: ['Small sizes: browser tab, favicon, 16 to 24px', '小尺寸：浏览器标签页、网站图标、16 到 24px'],
  },
];

function variantCard(v: Variant, assets: Assets): string {
  const L = v.key.toUpperCase();
  const src = (name: string): string => `../../${assets.file(repoPath('design/brand', `${name}.svg`))}`;
  const img = (name: string, s: number): string => `<img src="${src(name)}" width="${s}" height="${s}" alt="" loading="lazy" decoding="async">`;
  const stage = (dark: boolean): string => {
    const file = `cloud-${v.key}${dark ? '-dark' : ''}`;
    const small = `cloud-${v.key}-favicon`;
    const label = dark ? T('Dark', '深色') : T('Light', '浅色');
    const sizes = [16, 32, 64, 180].map((s) => `<figure>${img(s <= 32 ? small : file, s)}<figcaption>${s}px</figcaption></figure>`).join('');
    return `<div class="stage${dark ? ' dark' : ''}"><span class="stage-l">${label}</span>
<div class="mk-row" role="group" ${alBi({ en: `Variant ${L}, ${dark ? 'dark' : 'light'}: the icon at 16, 32, 64 and 180 pixels`, zh: `方案 ${L}${dark ? '深色' : '浅色'}：16、32、64、180 像素的图标` })}>${sizes}</div>
<div class="mk-row"><figure class="appicon">${img(file, 96)}<figcaption>${T('App icon', '应用图标')}</figcaption></figure></div></div>`;
  };
  return `<article class="card dir" id="variant-${v.key}"><div class="dir-h"><span class="dir-l" aria-hidden="true">${L}</span><div><h3>${T(`Variant ${L}: ${v.name}`, `方案 ${L}：${v.name}`)}</h3><p class="sub" style="margin:2px 0 0">${T(v.idea[0], v.idea[1])}</p></div>${v.key === 'c' ? `<span class="chip ok" style="margin-left:auto">${ic('check')}${T('Designer’s recommendation', '设计师推荐')}</span>` : ''}</div>
<div class="stages">${stage(false)}${stage(true)}</div>
<p style="margin:20px 0 0;text-wrap:pretty">${T(v.rationale[0], v.rationale[1])}</p>
<dl class="dl" style="margin-top:16px"><dt>${T('Best for', '适用于')}</dt><dd>${T(v.use[0], v.use[1])}</dd></dl></article>`;
}

/** Before and after: the header logo as it is today next to each variant, at the sizes the header and the favicon use. */
function beforeAfter(assets: Assets): string {
  const after = (key: Variant['key'], dark: boolean) => (s: number): string => `<img src="../../${assets.file(repoPath('design/brand', `cloud-${key}${s <= 32 ? '-favicon' : dark ? '-dark' : ''}.svg`))}" width="${s}" height="${s}" alt="" loading="lazy" decoding="async">`;
  const row = (inner: (s: number) => string): string => [28, 64, 128].map((s) => `<figure>${inner(s)}<figcaption>${s}px</figcaption></figure>`).join('');
  const now = (s: number): string => `<span class="logo-now" style="width:${s}px;height:${s}px">${LOGO}</span>`;
  const pair = (dark: boolean): string =>
    `<div class="stage${dark ? ' dark' : ''}"><span class="stage-l">${dark ? T('Dark', '深色') : T('Light', '浅色')}</span><div class="mk-row"><div class="ba"><b class="ba-t">${T('Today', '现在')}</b><div class="mk-row">${row(now)}</div></div>${VARIANTS.map((v) => `<div class="ba"><b class="ba-t">${T(`Variant ${v.key.toUpperCase()}`, `方案 ${v.key.toUpperCase()}`)}</b><div class="mk-row">${row(after(v.key, dark))}</div></div>`).join('')}</div></div>`;
  return `<section class="sec"><div class="sec-h"><h2 class="title">${T('Before and after', '改前与改后')}</h2></div><div class="stages">${pair(false)}${pair(true)}</div></section>`;
}

export function brandPage(site: Site, assets: Assets): Rendered {
  const intro = `<p class="sub" style="margin:0 0 16px;max-width:640px">${T('Approve means “I choose the variant named in my note”: write C or D in the issue note. Deny: say in the note what to change and the designer will adjust.', '批准表示“我选备注里写的那个方案”：请在议题备注里写 C 或 D。拒绝：在备注里写明想怎么改，设计师会调整。')}</p>`;
  const main = `<main id="main" class="wrap">${head(T('Your cloud icon, refined', '把你的云图标打磨一下'), T('You asked for an icon in the style of Apple’s SF Symbols that says “a cloud drive at home”. Here are two new directions, each at the sizes it will really be used, in light and dark.', '你希望图标采用 Apple SF Symbols 的风格，并且能表达“家里的网盘”。这里是两个新方向，按实际使用的尺寸展示，浅色与深色各一份。'))}
<div style="max-width:720px">${callout('info', { en: 'The designer recommends C, Hearth', zh: '设计师推荐 C：Hearth' }, { en: 'Use C at large sizes (logo, sign-in screen): it is the more explicit picture, a house inside the cloud. D, Roofline, is better at 16px, so it is the pick if one mark must serve every size. Nothing is applied to the product until you choose; the report site keeps today’s logo meanwhile.', zh: '大尺寸（标志、登录界面）用 C：它的画面更直白，云里有一座房子。D（Roofline）在 16px 时更清楚，如果只能用一个图标应付所有尺寸，就选 D。你选定之前，产品里不会改动任何地方；报告网站暂时仍用现在的标志。' })}</div>
${beforeAfter(assets)}
<div class="stack" style="margin-top:32px">${VARIANTS.map((v) => variantCard(v, assets)).join('')}</div>
<p class="small" style="margin-top:16px;max-width:640px">${T('The tiles here are the site-logo version. For a system app icon the artwork is delivered as a full square and the system rounds it. Product name stays “Cloud Home” in every language.', '这里的底是网站标志版本。系统应用图标会提供完整的方形图稿，由系统自己裁圆角。产品名在所有语言里都是 “Cloud Home”。')}</p>
${answer(site, 'A-004', intro)}</main>`;
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
