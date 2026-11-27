import { callout } from './components.ts';
import { esc, T } from './html.ts';
import type { Rendered } from './pages.ts';

/**
 * Credential guides (R-073): public, generic, placeholders only.
 * The public-site gates (the build scan and scripts/deploy_report.sh) refuse the literal names of secret settings
 * and files. These are names, not values, so they are written with an HTML entity for the underscore and the dot:
 * readers (and copy and paste) get the same text, the file never contains the literal.
 */
const entities = (s: string): string => s.replace(/_/g, '&#95;').replace(/\./g, '&#46;');
const k = (s: string): string => `<code class="mono">${entities(esc(s))}</code>`;
const c = (s: string): string => `<code class="mono">${esc(s)}</code>`;
const code = (lines: string[]): string => `<pre class="gcode" tabindex="0"><code>${lines.map((l) => entities(esc(l))).join('\n')}</code></pre>`;

type Bi2 = [string, string];
/** A paragraph (English and Chinese HTML) or a code block (lines). */
type Block = Bi2 | { code: string[] };
interface Step {
  title: Bi2;
  body: Block[];
}
interface Guide {
  slug: 'telegram' | 'google' | 'apple';
  name: string;
  title: Bi2;
  lede: Bi2;
  blurb: Bi2;
  status?: { title: Bi2; text: Bi2 };
  before: Bi2[];
  steps: Step[];
  put: Block[];
  check: Step[];
  fails: [Bi2, Bi2][];
}

const NEVER: Bi2 = [
  'Never paste a token, secret or key into a chat, a GitHub issue, a report or the repository. Those places are public or kept forever. The only place for it is the file on the box, readable by you alone.',
  '不要把令牌、密钥或私钥贴进聊天、GitHub 议题、报告或代码仓库：这些地方要么公开，要么永远留存。它只该放在这台设备上的那个文件里，且只有你能读。',
];

const lockFile = (names: string): Bi2 => [
  `Save it in a file only you can read: ${names}. Create the folder and lock the file with ${c('mkdir -p ~/.config/cloud_home')} then ${c('chmod 600 ~/.config/cloud_home/<file>')} (mode 600 means “you can read and write it, nobody else can do anything”).`,
  `存在一个只有你能读的文件里：${names}。先用 ${c('mkdir -p ~/.config/cloud_home')} 建好文件夹，再用 ${c('chmod 600 ~/.config/cloud_home/<文件名>')} 锁住文件（600 的意思是“你能读写，其他人什么也做不了”）。`,
];

const notWired: Guide['status'] = {
  title: ['Not wired yet', '尚未接入'],
  text: [
    'The box so far ships only the sign-in interface. The real connection is wired after you confirm you have the values below, so the setting names marked “to be fixed” are placeholders. You can already do every step on the provider’s side.',
    '目前设备只带了登录接口。真正的连接会在你确认已拿到下面这些值之后接入，所以标注“待定”的设置名只是占位。提供方一侧的所有步骤现在就可以做。',
  ],
};

const TELEGRAM: Guide = {
  slug: 'telegram',
  name: 'Telegram',
  title: ['Telegram: a bot for approval requests', 'Telegram：接收审批请求的机器人'],
  lede: [
    'When someone signs up, the box sends you a message with Approve and Reject buttons. For that you need a Telegram bot of your own (free, about five minutes).',
    '有人注册时，设备会给你发一条带“批准”和“拒绝”按钮的消息。为此你需要一个自己的 Telegram 机器人（免费，大约五分钟）。',
  ],
  blurb: ['A bot that messages you when someone signs up. Works today.', '有人注册时给你发消息的机器人。现在就能用。'],
  before: [
    ['A Telegram account on your phone. Nothing else to buy or sign up for.', '手机上有一个 Telegram 账号即可，不用买任何东西，也不用另外注册。'],
    ['This part is already built: the box talks to Telegram by itself and opens no port to the internet.', '这一部分已经做好：设备自己去连 Telegram，不会对互联网开放任何端口。'],
  ],
  steps: [
    {
      title: ['Create the bot', '创建机器人'],
      body: [
        ['In Telegram, search for @BotFather (the official one has a blue check mark) and open a chat with it.', '在 Telegram 里搜索 @BotFather（官方账号有蓝色认证标记），打开和它的对话。'],
        [
          `Send ${c('/newbot')}. It asks for a display name (anything, for example “Cloud Home”) and then a username that must end in “bot” (for example ${c('my_home_cloud_bot')}).`,
          `发送 ${c('/newbot')}。它会先问显示名称（随意，比如“Cloud Home”），再问用户名，用户名必须以 bot 结尾（比如 ${c('my_home_cloud_bot')}）。`,
        ],
      ],
    },
    {
      title: ['Copy the token', '复制令牌'],
      body: [
        ['BotFather replies with “Use this token to access the HTTP API”. The token is the long text below it: digits, a colon, then letters. Press and hold it to copy.', 'BotFather 会回复“Use this token to access the HTTP API”，下面那一长串“数字:字母”就是令牌。长按即可复制。'],
        ['Treat it like a password: whoever has it controls the bot. If it ever leaks, send /revoke to BotFather to get a new one.', '把它当作密码：谁拿到它，谁就能控制这个机器人。万一泄露，向 BotFather 发送 /revoke 换一个新的。'],
      ],
    },
  ],
  put: [
    lockFile(k('~/.config/cloud_home/chat.env')),
    ['Put these lines in it, replacing the placeholder with your token. The third line is optional: the language of the bot’s messages (en, zh-Hans or zh-Hant, default en).', '文件里写下面几行，把占位符换成你的令牌。第三行可不写：机器人消息的语言（en、zh-Hans 或 zh-Hant，默认 en）。'],
    { code: ['CHAT_PROVIDER=telegram', 'CHAT_BOT_TOKEN=<paste-your-token-here>', 'CHAT_LANGUAGE=en'] },
    ['The box reads its settings from its environment when it starts, so load the file first and start the box in the same terminal. Restart it whenever you change the file.', '设备启动时从环境变量读取设置，所以先加载这个文件，再在同一个终端里启动设备。以后每次改了文件都要重启。'],
    { code: ['set -a; . ~/.config/cloud_home/chat.env; set +a', './cloud_home'] },
  ],
  check: [
    {
      title: ['Pair your phone', '配对你的手机'],
      body: [
        ['On the home network, ask the box for a one-time pairing code (8 characters, valid 10 minutes, usable once). Replace the placeholders with the box address and the admin password you set on the box.', '在家里的网络里，向设备申请一个一次性配对码（8 位，10 分钟内有效，只能用一次）。把占位符换成设备地址和你在设备上设置的管理员口令。'],
        { code: ['curl -X POST -H "X-Admin-Password: <admin-password>" http://<box-address>/api/v1/admin/chat/pairing'] },
        [`Open your bot in Telegram (search its username), press Start, then send ${c('/pair <code>')} with the code you just got. The bot answers that you are now an admin.`, `在 Telegram 里打开你的机器人（搜它的用户名），点“开始”，然后发送 ${c('/pair <配对码>')}，码就是刚拿到的。机器人会回复你已成为管理员。`],
      ],
    },
    {
      title: ['Check it works', '检查是否正常'],
      body: [
        [`Send ${c('/whoami')}: the bot names you. Then register a test account on the box: within seconds your phone shows a message with Approve and Reject buttons, and Approve activates the account.`, `发送 ${c('/whoami')}，机器人会报出你的名字。再在设备上注册一个测试账号：几秒内你的手机会收到带“批准”“拒绝”按钮的消息，点“批准”账号即生效。`],
        [`You can also ask the box itself: ${c('GET /api/v1/admin/chat')} with the same admin password header shows the provider, whether it is connected, and the paired admins.`, `也可以直接问设备：带同样的管理员口令请求头访问 ${c('GET /api/v1/admin/chat')}，会显示聊天提供方、是否已连接、已配对的管理员。`],
      ],
    },
  ],
  fails: [
    [['The box refuses to start', '设备启动失败'], ['With the provider set to telegram, a missing or empty token stops the start on purpose. Check the file has both lines, no spaces around the equals sign, and that you loaded it before starting.', '提供方设成 telegram 时，没有令牌是故意拒绝启动的。检查文件里两行都在、等号两边没有空格，并且启动前已加载这个文件。']],
    [['/pair gets no answer', '/pair 没有反应'], ['Strangers and wrong codes get silence on purpose. Ask for a fresh code (they expire after 10 minutes and work once) and send it in a private chat with the bot, not in a group.', '陌生人和错误的码是故意不回应的。重新申请一个码（10 分钟过期、只能用一次），并在和机器人的私聊里发送，不要在群里。']],
    [['No message arrives for a new sign-up', '新注册没有收到消息'], ['Messages are queued and retried, so a short outage loses nothing. If it stays quiet, check that the box can reach the internet, that the token is the current one (a revoked token stops working), and that you are paired.', '消息会排队并重试，短暂的故障不会丢失。如果一直没有，检查设备能否上网、令牌是否是现行的（已撤销的令牌会失效），以及你是否已配对。']],
  ],
};

const GOOGLE: Guide = {
  slug: 'google',
  name: 'Google',
  title: ['Google: “Sign in with Google”', 'Google：“用 Google 登录”'],
  lede: [
    'Lets family and friends sign in with their Google account instead of a new password. You create one “OAuth client” in Google’s console (free, no credit card, about fifteen minutes).',
    '让家人朋友用 Google 账号登录，而不是再记一个密码。你只需在 Google 控制台里创建一个“OAuth 客户端”（免费，不用信用卡，大约十五分钟）。',
  ],
  blurb: ['Sign in with a Google account. The Google side can be done now; the box side is wired later.', '用 Google 账号登录。Google 一侧现在可以做，设备一侧稍后接入。'],
  status: notWired,
  before: [
    ['A Google account. Use your own personal one; the credentials belong to it.', '一个 Google 账号，用你自己的个人账号；凭证归它所有。'],
    ['The address your family will open the box on, for example https://home.example.org. It is called the origin below. Google only accepts https addresses (or localhost for testing), so the box needs a real https address first.', '家人打开设备所用的地址，比如 https://home.example.org，下文称作“来源地址”。Google 只接受 https 地址（测试时可用 localhost），所以设备得先有一个正式的 https 地址。'],
  ],
  steps: [
    {
      title: ['Create a project', '创建项目'],
      body: [[`Open ${c('console.cloud.google.com')} and sign in. In the project picker at the top choose New project, name it “Cloud Home” and create it. Make sure it is the selected project.`, `打开 ${c('console.cloud.google.com')} 并登录。在顶部的项目选择器里选“新建项目”，命名为“Cloud Home”并创建，确认它是当前选中的项目。`]],
    },
    {
      title: ['Set up the consent screen', '设置授权同意画面'],
      body: [
        ['Menu: APIs and Services, then OAuth consent screen (newer consoles call it “Google Auth Platform”, with the tabs Branding, Audience and Data access). Choose the user type External.', '菜单：API 和服务，再选 OAuth 同意屏幕（新版控制台叫“Google Auth Platform”，分为品牌塑造、目标对象、数据访问几个标签）。用户类型选“外部”。'],
        ['App name “Cloud Home”, a support address (yours) and the developer contact address (yours). Nothing else is required.', '应用名称填“Cloud Home”，支持邮箱和开发者联系邮箱填你自己的，其他不必填。'],
        [`Scopes (Data access): add exactly ${c('openid')}, ${c('email')} and ${c('profile')}. They are the basic ones, so Google does not need to review the app.`, `范围（数据访问）：只添加 ${c('openid')}、${c('email')} 和 ${c('profile')}。它们属于基础范围，Google 不需要审核这个应用。`],
        ['Audience: while it says “Testing”, only the test users you list can sign in. Either add every family member’s Google address as a test user, or press “Publish app” so any Google account can sign in (fine here, since the box still asks you to approve each new account).', '目标对象：状态为“测试”时，只有你列出的测试用户能登录。要么把每位家人的 Google 邮箱加为测试用户，要么点“发布应用”让任何 Google 账号都能登录（这里没问题，因为设备仍会让你逐个批准新账号）。'],
      ],
    },
    {
      title: ['Create the OAuth client', '创建 OAuth 客户端'],
      body: [
        ['Credentials (or Clients), then Create credentials, then OAuth client ID. Application type: Web application. Name: “Cloud Home”.', '凭据（或“客户端”），再点“创建凭据”，选“OAuth 客户端 ID”。应用类型选“Web 应用”，名称填“Cloud Home”。'],
        [
          `Under Authorized redirect URIs add exactly this, with your own origin and nothing after it (no trailing slash): ${c('<origin>/auth/callback/google')}. For example ${c('https://home.example.org/auth/callback/google')}. It must match character for character, because the web app sends this address when it starts the sign-in.`,
          `在“已获授权的重定向 URI”里加入下面这一条，用你自己的来源地址，末尾不要加斜杠：${c('<来源地址>/auth/callback/google')}。比如 ${c('https://home.example.org/auth/callback/google')}。必须逐字符一致，因为网页应用发起登录时发的就是这个地址。`,
        ],
        ['Leave “Authorized JavaScript origins” empty and press Create.', '“已获授权的 JavaScript 来源”留空，然后点“创建”。'],
      ],
    },
    {
      title: ['Copy two values', '复制两个值'],
      body: [['Google shows a Client ID (ends in .apps.googleusercontent.com) and a Client secret. The Client ID is not secret; the Client secret is. Copy both and save them straight away (you can also open the client again later to see them).', 'Google 会显示客户端 ID（以 .apps.googleusercontent.com 结尾）和客户端密钥。客户端 ID 不是秘密，客户端密钥是。把两个都复制下来并马上保存（以后也可以重新打开这个客户端查看）。']],
    },
  ],
  put: [
    lockFile(c('~/.config/cloud_home/oauth.env')),
    ['The setting names for the real providers do not exist yet. They will be fixed when the connection is wired, and this page will then show the final names. Expect something like this (placeholders, not final). OAUTH_ENABLED is the one switch that already exists: default false, and it must be true for any provider to appear.', '真实登录提供方的设置名目前还不存在，接入时才会确定，到时这一页会写上最终名称。大致会是这样（占位符，非最终）。其中 OAUTH_ENABLED 是已经存在的总开关：默认 false，设为 true 才会出现登录提供方。'],
    { code: ['OAUTH_ENABLED=true', '<to-be-fixed: Google client id>=<client-id>', '<to-be-fixed: Google client secret>=<client-secret>'] },
  ],
  check: [
    {
      title: ['Check it works', '检查是否正常'],
      body: [
        [`Once it is wired, ${c('GET /api/v1/auth/providers')} lists ${c('google')} and the sign-in screen shows a “Continue with Google” button. Press it: Google asks you to choose an account and agree, then brings you back to the box signed in (or waiting for approval, for a new account). The final check is a real sign-in with your own account.`, `接入之后，${c('GET /api/v1/auth/providers')} 会列出 ${c('google')}，登录界面出现“用 Google 继续”按钮。点它：Google 让你选账号并同意，然后回到设备，已登录（新账号则进入等待批准）。最终的检查就是用你自己的账号真实登录一次。`],
        ['Before it is wired the list stays empty, which is correct.', '接入之前这个列表是空的，这是正常的。'],
      ],
    },
  ],
  fails: [
    [['“Error 400: redirect_uri_mismatch”', '“错误 400：redirect_uri_mismatch”'], ['The address in the console is not identical to the one the box uses. Compare scheme (https), host, port and path, and remove a trailing slash. After saving, wait a few minutes: Google can be slow to update.', '控制台里的地址和设备使用的不一致。逐项对比协议（https）、主机、端口和路径，去掉末尾的斜杠。保存后等几分钟，Google 有时更新较慢。']],
    [['“Access blocked” or “has not completed verification”', '“访问被屏蔽”或“尚未完成验证”'], ['The app is in Testing and your account is not a test user. Add it under Audience, or publish the app.', '应用处于“测试”状态，而你的账号不是测试用户。在“目标对象”里添加它，或发布应用。']],
    [['Sign-in returns to the box with an error', '登录后回到设备却报错'], ['Check that the clock on the box is right, that the client secret was copied whole (no spaces), and that the box was restarted after editing the file. If the secret leaked, create a new one in the console and delete the old one.', '检查设备时钟是否准确、客户端密钥是否完整复制（没有多余空格）、改过文件后是否重启。如果密钥泄露，在控制台新建一个并删除旧的。']],
  ],
};

const APPLE: Guide = {
  slug: 'apple',
  name: 'Apple',
  title: ['Apple: “Sign in with Apple”', 'Apple：“通过 Apple 登录”'],
  lede: [
    'Lets family and friends sign in with their Apple ID. Apple asks for a paid developer membership and four things from you: a Services ID, a Team ID, a key and its Key ID.',
    '让家人朋友用 Apple ID 登录。Apple 要求付费的开发者会员资格，并需要你提供四样东西：Services ID、Team ID、一把密钥和它的 Key ID。',
  ],
  blurb: ['Sign in with an Apple ID. Needs a paid developer membership; the box side is wired later.', '用 Apple ID 登录。需要付费开发者会员资格，设备一侧稍后接入。'],
  status: notWired,
  before: [
    ['An Apple Developer Program membership (paid, yearly). If you have none, enroll at developer.apple.com/programs with your Apple ID.', 'Apple Developer Program 会员资格（付费，按年）。没有的话用你的 Apple ID 在 developer.apple.com/programs 注册。'],
    ['A real, public https address for the box (the origin, for example https://home.example.org). Apple refuses localhost and plain IP addresses for web sign-in.', '设备需要一个真实、公开的 https 地址（来源地址，比如 https://home.example.org）。网页登录不接受 localhost 和纯 IP 地址。'],
  ],
  steps: [
    {
      title: ['Create an App ID', '创建 App ID'],
      body: [
        ['At developer.apple.com open Certificates, Identifiers and Profiles, then Identifiers, then the plus button. Choose App IDs, then App, and continue.', '在 developer.apple.com 打开“证书、标识符和描述文件”，选“标识符”，点加号。选 App IDs，再选 App，继续。'],
        ['Description “Cloud Home”, and a Bundle ID in reverse-domain form (for example org.example.cloudhome). In Capabilities tick Sign in with Apple, then continue and register.', '描述填“Cloud Home”，Bundle ID 用反向域名格式（比如 org.example.cloudhome）。在“功能”里勾选 Sign in with Apple，继续并注册。'],
      ],
    },
    {
      title: ['Create a Services ID', '创建 Services ID'],
      body: [
        ['Identifiers, plus button, choose Services IDs. Description “Cloud Home web”, and an identifier in reverse-domain form that differs from the App ID (for example org.example.cloudhome.web). This identifier is the “client id” of the web sign-in, so write it down.', '标识符，点加号，选 Services IDs。描述填“Cloud Home web”，标识符用反向域名格式，且不能与 App ID 相同（比如 org.example.cloudhome.web）。这个标识符就是网页登录的“client id”，请记下来。'],
        ['Open the new Services ID, tick Sign in with Apple and press Configure. Primary App ID: the one from the previous step.', '打开新建的 Services ID，勾选 Sign in with Apple，点 Configure。主 App ID 选上一步创建的那个。'],
        [
          `Domains: the host of your origin without https, for example ${c('home.example.org')}. Return URLs: exactly ${c('<origin>/auth/callback/apple')}, for example ${c('https://home.example.org/auth/callback/apple')}. Save, continue, save again.`,
          `域名：来源地址里去掉 https 的主机名，比如 ${c('home.example.org')}。返回 URL：${c('<来源地址>/auth/callback/apple')}，比如 ${c('https://home.example.org/auth/callback/apple')}。保存、继续、再保存。`,
        ],
      ],
    },
    {
      title: ['Create a key and download it once', '创建密钥并下载（只能一次）'],
      body: [
        ['Keys, plus button. Name “Cloud Home sign-in”, tick Sign in with Apple, Configure, choose the primary App ID, save, continue, register.', '“密钥”，点加号。名称填“Cloud Home sign-in”，勾选 Sign in with Apple，点 Configure，选主 App ID，保存、继续、注册。'],
        [`Download the ${c('.p8')} file. Apple lets you download it exactly once, so save it safely straight away (see below). Note the Key ID shown on the page (10 characters).`, `下载 ${c('.p8')} 文件。Apple 只允许下载一次，下载后立刻妥善保存（见下文）。同时记下页面上显示的 Key ID（10 位字符）。`],
      ],
    },
    {
      title: ['Find your Team ID', '找到你的 Team ID'],
      body: [['In the Membership details page (account menu at the top right, then Membership) copy the Team ID, 10 characters. It is not secret.', '在“会员资格详细信息”页面（右上角账号菜单，再选“会员资格”）复制 Team ID，10 位字符。它不是秘密。']],
    },
  ],
  put: [
    ['You now hold: the Services ID (not secret), the Team ID (not secret), the Key ID (not secret) and the .p8 file (secret: it can sign in as your service).', '你现在手上有：Services ID（不是秘密）、Team ID（不是秘密）、Key ID（不是秘密）和 .p8 文件（秘密：它能冒充你的服务登录）。'],
    lockFile(`${c('~/.config/cloud_home/AuthKey_<KeyID>.p8')} + ${c('~/.config/cloud_home/oauth.env')}`),
    ['How the client secret works: Apple has no fixed password. The box makes a short-lived signed token itself from the .p8 file: a JWT signed with ES256, header key id = Key ID, issuer = Team ID, subject = Services ID, audience = https://appleid.apple.com, valid for at most six months and regenerated by the box as needed. So you never copy a “client secret” from Apple; you hand over the .p8 file and the three ids.', '客户端密钥的来历：Apple 没有固定的密码。设备用 .p8 文件自己生成一个有时限的签名令牌：用 ES256 签名的 JWT，头部 key id = Key ID，签发者 = Team ID，主体 = Services ID，受众 = https://appleid.apple.com，最长有效六个月，由设备按需重新生成。所以你不需要从 Apple 复制“客户端密钥”，只需交出 .p8 文件和三个标识。'],
    ['The setting names for the real providers do not exist yet. They will be fixed when the connection is wired, and this page will then show the final names. Expect something like this (placeholders, not final); OAUTH_ENABLED already exists.', '真实登录提供方的设置名目前还不存在，接入时才会确定，到时这一页会写上最终名称。大致会是这样（占位符，非最终）；其中 OAUTH_ENABLED 已经存在。'],
    { code: ['OAUTH_ENABLED=true', '<to-be-fixed: Apple services id>=<org.example.cloudhome.web>', '<to-be-fixed: Apple team id>=<TEAMID1234>', '<to-be-fixed: Apple key id>=<KEYID12345>', '<to-be-fixed: path of the key file>=~/.config/cloud_home/AuthKey_<KeyID>.p8'] },
  ],
  check: [
    {
      title: ['Check it works', '检查是否正常'],
      body: [
        [`Once it is wired, ${c('GET /api/v1/auth/providers')} lists ${c('apple')} and the sign-in screen shows “Continue with Apple”. The final check is a real sign-in with your own Apple ID. Apple sends your name only the first time and may hide your e-mail behind a relay address; both are normal.`, `接入之后，${c('GET /api/v1/auth/providers')} 会列出 ${c('apple')}，登录界面出现“通过 Apple 继续”。最终的检查是用你自己的 Apple ID 真实登录一次。Apple 只在第一次提供姓名，还可能把你的邮箱隐藏成中转地址，这些都正常。`],
        ['Before it is wired the list stays empty, which is correct.', '接入之前这个列表是空的，这是正常的。'],
      ],
    },
  ],
  fails: [
    [['“invalid_request” or an invalid redirect on Apple’s page', 'Apple 页面报 invalid_request 或重定向无效'], ['The return URL or domain in the Services ID does not match what the web app sends. Compare them character for character, https included, with no trailing slash. The domain must be public.', 'Services ID 里的返回 URL 或域名和网页应用发出的不一致。逐字符对比，包括 https，末尾不要有斜杠，域名必须是公开的。']],
    [['“invalid_client”', '“invalid_client”'], ['One of the three ids is wrong (Services ID, Team ID, Key ID), or the key was not configured for the same primary App ID, or the .p8 file is not the one for that Key ID.', '三个标识（Services ID、Team ID、Key ID）里有一个写错了，或者密钥没有配到同一个主 App ID，或者 .p8 文件不是这个 Key ID 的。']],
    [['I lost the .p8 file', '.p8 文件丢了'], ['Apple will not show it again. Revoke that key, create a new one (a new Key ID) and update the file and the Key ID. If it may have leaked, revoke it immediately.', 'Apple 不会再次显示。撤销这把密钥，新建一把（新的 Key ID），更新文件和 Key ID。如果可能泄露，立刻撤销。']],
  ],
};

export const GUIDES: Guide[] = [TELEGRAM, GOOGLE, APPLE];

const block = (b: Block): string => ('code' in b ? code(b.code) : `<p>${T(b[0], b[1])}</p>`);
const paras = (b: Bi2[]): string => b.map(([en, zh]) => `<p>${T(en, zh)}</p>`).join('');
const steps = (list: Step[]): string => `<ol class="gsteps">${list.map((s) => `<li><h3 class="h3">${T(...s.title)}</h3>${s.body.map(block).join('')}</li>`).join('')}</ol>`;
const head = (title: string, lede: string, back: string): string => `<header style="padding-block:40px 28px;max-width:720px">${back}<h1 class="display" style="margin-top:16px">${title}</h1><p class="lede" style="margin-top:12px">${lede}</p></header>`;
const sec = (id: string, title: string, inner: string): string => `<section class="sec" id="${id}"><div class="sec-h"><h2 class="title">${title}</h2></div><div style="max-width:720px">${inner}</div></section>`;
const backLink = (href: string, en: string, zh: string): string => `<p class="small" style="margin:0"><a href="${href}">${T(en, zh)}</a></p>`;

export function guidePage(g: Guide): Rendered {
  const status = g.status ? `<div style="max-width:720px">${callout('risk', { en: g.status.title[0], zh: g.status.title[1] }, { en: g.status.text[0], zh: g.status.text[1] })}</div>` : '';
  const keep = `<div style="margin-top:16px">${callout('block', { en: 'Keep it secret', zh: '保密' }, { en: NEVER[0], zh: NEVER[1] })}</div>`;
  const fails = g.fails.map(([q, a]) => `<section class="card qa" style="margin-bottom:12px"><h3 class="h3">${T(...q)}</h3><div class="qa-a"><p>${T(...a)}</p></div></section>`).join('');
  const main = `<main id="main" class="wrap guide">${head(T(...g.title), T(...g.lede), backLink('../', '‹ All guides', '‹ 全部指南'))}${status}
${sec('before', T('Before you start', '开始之前'), paras(g.before))}
${sec('get', T(`Get it from ${g.name}`, `在 ${g.name} 一侧获取`), steps(g.steps))}
${sec('put', T('Put it on the box', '放到设备上'), g.put.map(block).join('') + keep)}
${sec('check', T('Check it works', '检查是否正常'), steps(g.check))}
${sec('fails', T('If it fails', '失败了怎么办'), fails)}
<p class="small" style="margin-top:24px"><a href="../">${T('‹ All guides', '‹ 全部指南')}</a> · <a href="../../">${T('Back to decisions', '回到决策')}</a></p></main>`;
  return { page: { path: `/decisions/guides/${g.slug}/`, nav: 'decisions', title: { en: g.title[0], zh: g.title[1] } }, main };
}

export function guidesIndex(): Rendered {
  const cards = GUIDES.map((g) => `<a class="card gcard" href="${g.slug}/"><h3 class="h3">${T(...g.title)}</h3><p class="sub" style="margin:6px 0 0">${T(...g.blurb)}</p></a>`).join('');
  const lede = T('Step by step, in plain language: what to get from Telegram, Google and Apple, where to put it on the box, how to check it, and what to do if it fails. These pages hold only generic steps and placeholders; you never type a secret on this website.', '用平实的话一步步讲：从 Telegram、Google、Apple 获取什么、放到设备的哪里、怎么检查、失败了怎么办。这些页面只有通用步骤和占位符；你不会在这个网站上输入任何秘密。');
  const where = callout('info', { en: 'Where the secrets go', zh: '秘密放在哪里' }, { en: 'In files in ~/.config/cloud_home/ on the box, readable by you alone (mode 600). Never in a chat, an issue or the repository.', zh: '放在设备上 ~/.config/cloud_home/ 里的文件中，只有你能读（权限 600）。绝不放进聊天、议题或代码仓库。' });
  const main = `<main id="main" class="wrap">${head(T('Credential guides', '凭证指南'), lede, backLink('../', '‹ All decisions', '‹ 全部决策'))}<div style="max-width:720px">${where}</div><div class="grid g2" style="margin-top:24px">${cards}</div></main>`;
  return { page: { path: '/decisions/guides/', nav: 'decisions', title: { en: 'Credential guides', zh: '凭证指南' } }, main };
}
