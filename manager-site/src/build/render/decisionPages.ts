import { ic } from '../../shared/icons.ts';
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

// ---------- A-001: brand directions ----------

interface Direction {
  key: 'a' | 'b' | 'c';
  name: string;
  idea: [string, string];
  rationale: [string, string];
  colour: [string, string];
  small: [string, string];
}

const DIRECTIONS: Direction[] = [
  {
    key: 'a',
    name: 'Hearth',
    idea: ['A house silhouette with a round window.', '带圆形窗口的房屋剪影。'],
    rationale: [
      'The product is a drive that lives in your house, so the mark is the house: one soft-cornered silhouette with a round window cut through it, like a lit room. One filled shape survives 16px and any system effect, and it uses the blue the interface already has. It is the most literal and friendliest option; the risk is familiarity, because house shapes are common and close to a “Home” glyph in navigation.',
      '产品是放在家里的云盘，所以标志就是一座“家”：圆角的房屋剪影，中间镂空一个圆形窗口，像一间亮着灯的房间。单一实心形状在 16px 下依然清晰，也经得起系统的光影处理；颜色直接沿用界面现有的蓝色。它最直白、最亲切；风险是房屋图形很常见，并且与导航里可能出现的“主页”图标相近。',
    ],
    colour: ['None added (the existing blue)', '不新增颜色（沿用现有蓝色）'],
    small: ['Clear at 16px', '16px 下清晰'],
  },
  {
    key: 'b',
    name: 'Roofbox',
    idea: ['An amber roof over a teal device with a status light.', '琥珀色屋顶，下面是带指示灯的青绿色设备。'],
    rationale: [
      'Cloud Home is a real object on a shelf, a drive with a status light, that belongs to a household. An amber roof stands for the family, a teal rounded body for the device, and one cut-out light says “on and connected”. Two colours make it the most expressive and the easiest to tell apart from generic blue storage apps. The cost is a second brand colour next to the interface blue, and finer detail: the light is under 1px at 16px, so there the mark relies on the roof and the box.',
      'Cloud Home 是放在架子上的一台真实设备：带指示灯的硬盘，属于一个家庭。琥珀色屋顶代表“家人”，青绿色圆角机身代表“设备”，镂空的一盏灯表示“已开机、已连接”。双色让它最有表现力，也最容易与常见的蓝色网盘区分。代价是在界面蓝之外多了一个品牌色，细节也更多：16px 时指示灯不足 1px，此时靠屋顶和机身的形状辨认。',
    ],
    colour: ['Adds teal and amber (brand moments only)', '新增青绿与琥珀（仅用于品牌场景）'],
    small: ['Light is under 1px at 16px', '16px 时指示灯不足 1px'],
  },
  {
    key: 'c',
    name: 'Open Ring',
    idea: ['A graphite ring, open on the right, with one blue dot.', '石墨色圆环，右侧留有开口，里面一个蓝点。'],
    rationale: [
      'The ring is a “C” for Cloud and a circle of people around a table; the dot in its opening is your home device, always on, always reachable. It is the quietest of the three and the closest to Apple’s own restraint: neutral graphite, one small spot of the existing blue, no house or cloud cliché. It will not collide with a “Home” icon in navigation, it stays crisp at 16px, and it dims gracefully into mono and tinted icon looks. The trade-off is that the meaning is learned, not obvious; the wordmark carries the name.',
      '圆环既是 Cloud 的 “C”，也像一家人围坐的圆桌；开口处的圆点就是你家里的设备，始终在线、随时可达。它是三个方案里最安静的，也最贴近 Apple 式的克制：中性的石墨色加一点现有的蓝色，没有房子，也没有云的老套图形。不会与导航里的“主页”图标冲突；16px 下依然清晰；在单色与着色图标外观下也很从容。代价是含义需要学习而非一眼可见，由文字标识负责传达名称。',
    ],
    colour: ['None added (neutrals and the existing blue)', '不新增颜色（中性色加现有蓝色）'],
    small: ['Clear at 16px (2.25px ring)', '16px 下清晰（环宽约 2.25px）'],
  },
];

function directionCard(d: Direction, assets: Assets): string {
  const f = (name: string): string => assets.file(repoPath('design/brand', `direction-${d.key}`, `${name}.svg`));
  const L = d.key.toUpperCase();
  const img = (name: string, w: number, h: number): string => `<img src="../../${f(name)}" width="${w}" height="${h}" alt="" loading="lazy" decoding="async">`;
  const stage = (dark: boolean): string => {
    const sfx = dark ? '-dark' : '';
    const label = dark ? T('Dark', '深色') : T('Light', '浅色');
    return `<div class="stage${dark ? ' dark' : ''}"><span class="stage-l">${label}</span>
<div class="mk-row" role="group" ${dark ? alBi({ en: `Direction ${L}, dark: mark at 16, 32 and 64 pixels`, zh: `方案 ${L} 深色：16、32、64 像素的标志` }) : alBi({ en: `Direction ${L}, light: mark at 16, 32 and 64 pixels`, zh: `方案 ${L} 浅色：16、32、64 像素的标志` })}>${[16, 32, 64].map((s) => `<figure>${img(`mark${sfx}`, s, s)}<figcaption>${s}px</figcaption></figure>`).join('')}</div>
<div class="mk-row"><figure class="appicon">${img(`app-icon${sfx}`, 84, 84)}<figcaption>${T('App icon', '应用图标')}</figcaption></figure><figure class="wm">${img(`wordmark${sfx}`, 183, 48)}<figcaption>${T('Wordmark', '文字标识')}</figcaption></figure></div></div>`;
  };
  return `<article class="card dir" id="direction-${d.key}"><div class="dir-h"><span class="dir-l" aria-hidden="true">${L}</span><div><h3>${T(`Direction ${L}: ${d.name}`, `方案 ${L}：${d.name}`)}</h3><p class="sub" style="margin:2px 0 0">${T(d.idea[0], d.idea[1])}</p></div>${d.key === 'c' ? `<span class="chip ok" style="margin-left:auto">${ic('check')}${T('Designer’s recommendation', '设计师推荐')}</span>` : ''}</div>
<div class="stages">${stage(false)}${stage(true)}</div>
<p style="margin:20px 0 0;text-wrap:pretty">${T(d.rationale[0], d.rationale[1])}</p>
<dl class="dl" style="margin-top:16px"><dt>${T('Colour', '颜色')}</dt><dd>${T(d.colour[0], d.colour[1])}</dd><dt>${T('Small sizes', '小尺寸')}</dt><dd>${T(d.small[0], d.small[1])}</dd></dl></article>`;
}

export function brandPage(site: Site, assets: Assets): Rendered {
  const intro = `<p class="sub" style="margin:0 0 16px;max-width:640px">${T('To pick a direction, tap Approve and write the letter (A, B or C) in the issue body. If you approve without a letter, the designer’s recommendation, C, is used.', '想选某个方案，点“批准”，然后在议题正文里写上字母（A、B 或 C）。批准但不写字母，则采用设计师推荐的 C。')}</p>`;
  const main = `<main id="main" class="wrap">${head(T('Pick a brand for Cloud Home', '为 Cloud Home 选一个品牌标志'), T('Three directions for the logo, app icon and wordmark. Each one is shown at the sizes it will really be used, in light and dark.', '三个方向的标志、应用图标与文字标识。每个都按实际使用的尺寸展示，浅色与深色各一份。'))}
<div style="max-width:720px">${callout('info', { en: 'The designer recommends C, Open Ring', zh: '设计师推荐 C：开口圆环' }, { en: 'It is the quietest, adds no new colour, stays crisp at 16px and will not clash with a Home icon in the app. Nothing is applied anywhere until you choose.', zh: '它最安静，不新增颜色，16px 下依然清晰，也不会与应用里的“主页”图标冲突。你选定之前，不会改动任何地方。' })}</div>
<div class="stack" style="margin-top:32px">${DIRECTIONS.map((d) => directionCard(d, assets)).join('')}</div>
<p class="small" style="margin-top:16px;max-width:640px">${T('Wordmarks use the system font here; they will be turned into outlined shapes before the chosen one ships. Product name stays “Cloud Home” in every language.', '这里的文字标识使用系统字体；选定后会转成轮廓图形再发布。产品名在所有语言里都是 “Cloud Home”。')}</p>
${answer(site, 'A-001', intro)}</main>`;
  return { page: { path: '/decisions/brand/', nav: 'decisions', title: { en: 'Brand directions', zh: '品牌方案' } }, main };
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
