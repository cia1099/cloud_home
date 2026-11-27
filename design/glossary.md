# Cloud Home glossary

One source for UI wording, shared with the frontend engineer. Add a term here before it appears in a screen.

**Which table applies where**

| Table | Used by | Languages |
|---|---|---|
| A. Report site | `manager-site/` (public report site, R-001) | English, 简体中文 only |
| B, C, D | the web app in `ui/` (R-005, R-006 to R-013, R-062): the first, shorter draft | English, 简体中文, 繁體中文 |
| **E** | **the web app: the complete string table of the R-002 screens, same keys as `design/mockups/app/assets/i18n.js`. Where E and B/C/D differ, E wins** (it was written against the real screens) | English, 简体中文, 繁體中文 |

Rules

- **繁體中文 is written as real Traditional terminology for Taiwan** (what a Taiwanese user would say), not a character conversion of the Simplified text. Where the two differ in vocabulary, not just script, the row says so in the Notes column. Examples: 檔案 (not 文件), 資料夾, 上傳, 帳號 (not 賬戶), 軟體, 使用者, 登入.
- 简体中文 follows mainland usage: 文件, 文件夹, 上传, 账户, 登录.
- Chinese is not shorter than English in buttons and labels. Design and test with the longest string (listed in the "Longest" notes where it matters).
- English: sentence case for body text and messages, title case for buttons and menu items in the web app (Apple style: "Upload Files"), sentence case on the report site. Chinese has no case; no spaces between Chinese and numbers are required, but a thin space between Latin words and Chinese characters is not used.
- Punctuation: Simplified uses full-width `，。：；！？` and curly quotes “ ”. Traditional (Taiwan) uses full-width `，。：；！？` and corner quotes 「 」. Never mix.
- Numbers, dates and sizes are formatted with `Intl` for the active locale (`en`, `zh-Hans-CN`, `zh-Hant-TW`), not hard-coded.
- Names kept in Latin script in all languages: Cloud Home, GitHub, SQLite, SSE, ZIP, tar.gz, `v0.3.1-rc.2`, R-012, A-003.
- Error messages say what happened and what to do next, without apologising.

---

## A. Report site (English and 简体中文 only)

### A1. Status and priority

| Key | English | 简体中文 | Notes |
|---|---|---|---|
| status.todo | To do | 待办 | |
| status.doing | In progress | 进行中 | SPEC status `doing` |
| status.review | In review | 审核中 | |
| status.done | Done | 已完成 | |
| status.blocked | Blocked | 受阻 | the reason is shown next to it |
| status.dropped | Dropped | 已取消 | struck through, not counted |
| priority.p0 | P0 (must) | P0（必须） | |
| priority.p1 | P1 (should) | P1（应该） | |
| priority.p2 | P2 (nice to have) | P2（锦上添花） | |
| status.on_track | On track | 进展顺利 | report status chip |
| status.at_risk | At risk | 有风险 | |
| status.off_track | Off track | 已偏离计划 | |
| state.working | Working | 工作中 | role card |
| state.idle | Idle | 空闲 | |

### A2. Actions and navigation

| Key | English | 简体中文 | Notes |
|---|---|---|---|
| nav.overview | Overview | 概览 | |
| nav.reports | Reports | 报告 | |
| nav.timeline | Timeline | 时间线 | |
| nav.requirements | Requirements | 需求 | |
| nav.product | Product | 产品 | |
| nav.decisions | Decisions | 决策 | |
| nav.more | More | 更多 | phone tab bar |
| action.ask | Ask a question | 提问 | opens `[Q] ` issue |
| action.new_req | New requirement | 新需求 | `[ADD] ` issue |
| action.change | Change | 修改 | `[CHG R-nnn] ` issue |
| action.remove | Remove | 移除 | `[DEL R-nnn] ` issue |
| action.approve | Approve | 批准 | `[DECISION A-nnn] approve` |
| action.deny | Deny | 拒绝 | `[DECISION A-nnn] deny` |
| action.review | Review | 去处理 | banner button |
| action.read_report | Read the report | 阅读报告 | |
| action.view_table | View as table | 以表格查看 | burn-down |
| action.skip | Skip to content | 跳到正文 | |
| ui.language | Language | 语言 | switch label |
| ui.appearance | Appearance | 外观 | |
| ui.auto / light / dark | Auto / Light / Dark | 自动 / 浅色 / 深色 | |
| ui.close / prev / next | Close / Previous image / Next image | 关闭 / 上一张 / 下一张 | lightbox |
| ui.play / pause / replay | Play / Pause / Replay | 播放 / 暂停 / 重播 | walkthrough |
| ui.compare | Compare before and after | 对比前后效果 | slider name |
| ui.search_req | Search requirements | 搜索需求 | placeholder: `R-012, upload…` / `R-012、上传…` |

### A3. Decisions and requests

| Key | English | 简体中文 | Notes |
|---|---|---|---|
| decision.pending | pending decision | 待处理的决策 | |
| decision.n_waiting | {n} decisions are waiting for you | {n} 个决策在等你 | |
| decision.none | Nothing is waiting for your decision | 没有需要你决定的事 | |
| decision.caught_up | You are all caught up | 全部处理完毕 | |
| decision.what / why / risk | What / Why / Risk | 内容 / 原因 / 风险 | |
| decision.if_approved / denied | If approved / If denied | 批准后 / 拒绝后 | |
| decision.approved / denied | Approved / Denied | 已批准 / 已拒绝 | history chips |
| decision.history | History | 历史 | |
| decision.step1 | Tap Approve or Deny | 点“批准”或“拒绝” | |
| decision.step2 | Press Submit new issue | 按“Submit new issue” | GitHub's own button label stays English |
| decision.step3 | The manager picks it up | 经理在下个周期处理 | |
| decision.note | Opens a pre-filled issue on GitHub. Nothing is sent until you press Submit there. | 在 GitHub 上打开预填好的议题。你按下提交之前不会发送任何内容。 | |
| req.read_only | A read-only view of SPEC.md with live status. | 来自 SPEC.md 的只读视图，状态实时更新。 | |
| req.no_match | No requirement matches. | 没有匹配的需求。 | |
| req.done_of | {d} of {n} planned done | 计划内 {d}/{n} 已完成 | |

### A4. Roles and report vocabulary

| Key | English | 简体中文 | Notes |
|---|---|---|---|
| role.manager | Manager | 经理 | |
| role.backend | Backend engineer | 后端工程师 | |
| role.frontend | Frontend engineer | 前端工程师 | |
| role.designer | Designer | 设计师 | |
| role.tester | Tester | 测试工程师 | |
| area.site / web / backend / private / quality / release | Report site / Web app / Backend / Private access / Quality / Release | 报告网站 / 网页应用 / 后端 / 私有访问 / 质量 / 发布 | |
| report.milestone / weekly | Milestone / Weekly | 里程碑 / 周报 | |
| report.latest | Latest report | 最新报告 | |
| report.sample | Sample | 示例 | label on template and demo content |
| report.not_built | Not built yet | 尚未开发 | diagram module |
| report.built | Built | 已完成 | |
| kpi.tests_passing | Tests passing | 测试通过 | |
| kpi.latest_build | Latest build | 最新版本 | |
| kpi.pre_release | Pre-release | 预发布版 | |
| chart.burndown | Requirement burn-down | 需求燃尽图 | |
| chart.by_area | Progress by area | 按领域看进度 | |
| ota.check / download / verify / selftest / switch / restart / health / done | Check / Download / Verify / Self-test / Switch / Restart / Health check / Done or rolled back | 检查 / 下载 / 验证 / 自检 / 切换 / 重启 / 健康检查 / 完成或回滚 | |
| ota.rollback | If any step fails: the old version keeps running and the database copy from before the update is restored. | 任何一步失败：保持旧版本运行，并恢复更新前的数据库副本。 | |
| product.empty_title | The web app is not built yet. | 网页应用还没有开发。 | |
| product.empty_text | Screenshots and walkthroughs will appear here. | 截图和演示会出现在这里。 | |

---

## B. Web app: files and drive (English, 简体中文, 繁體中文)

| Key | English | 简体中文 | 繁體中文 (Taiwan) | Notes |
|---|---|---|---|---|
| app.name | Cloud Home | Cloud Home | Cloud Home | not translated |
| nav.files | Files | 文件 | 檔案 | |
| nav.recent | Recent | 最近 | 最近項目 | |
| nav.shared | Shared | 已分享 | 已分享 | |
| nav.trash | Trash | 回收站 | 垃圾桶 | TW: macOS and Google Drive use 垃圾桶, not 回收站 |
| nav.settings | Settings | 设置 | 設定 | |
| file | File | 文件 | 檔案 | |
| folder | Folder | 文件夹 | 資料夾 | |
| new_folder | New Folder | 新建文件夹 | 新增資料夾 | TW verb: 新增 |
| upload | Upload | 上传 | 上傳 | |
| upload_files | Upload Files | 上传文件 | 上傳檔案 | |
| upload_folder | Upload Folder | 上传文件夹 | 上傳資料夾 | |
| uploading_n | Uploading {n} items | 正在上传 {n} 个项目 | 正在上傳 {n} 個項目 | |
| upload_done | Upload complete | 上传完成 | 上傳完成 | |
| upload_cancel | Cancel upload | 取消上传 | 取消上傳 | |
| upload_failed | Upload failed | 上传失败 | 上傳失敗 | |
| renamed_as | Saved as “{name}” because that name was taken | 因名称已存在，已保存为“{name}” | 因名稱已存在，已儲存為「{name}」 | server adds `(1)` |
| download | Download | 下载 | 下載 | |
| download_zip | Download as ZIP | 下载为 ZIP | 下載為 ZIP | |
| download_targz | Download as tar.gz | 下载为 tar.gz | 下載為 tar.gz | |
| rename | Rename | 重命名 | 重新命名 | |
| move | Move | 移动 | 移動 | |
| copy | Copy | 复制 | 複製 | |
| delete | Move to Trash | 移到回收站 | 移至垃圾桶 | |
| delete_forever | Delete Forever | 永久删除 | 永久刪除 | |
| restore | Restore | 恢复 | 還原 | TW: 還原 |
| empty_trash | Empty Trash | 清空回收站 | 清空垃圾桶 | |
| trash_days_left | {n} days left | 还剩 {n} 天 | 剩下 {n} 天 | 7-day retention |
| share | Share | 分享 | 分享 | |
| share_link | Share link | 分享链接 | 分享連結 | TW: 連結 |
| copy_link | Copy Link | 复制链接 | 複製連結 | |
| link_expires | Link expires | 链接有效期 | 連結有效期限 | |
| allow_download | Allow downloads | 允许下载 | 允許下載 | |
| revoke_link | Stop Sharing | 停止分享 | 停止分享 | |
| search | Search | 搜索 | 搜尋 | TW: 搜尋 |
| search_placeholder | Search files | 搜索文件 | 搜尋檔案 | |
| no_results | No results | 没有结果 | 沒有結果 | |
| sort_by | Sort by | 排序方式 | 排序方式 | |
| name / size / modified | Name / Size / Date Modified | 名称 / 大小 / 修改日期 | 名稱 / 大小 / 修改日期 | |
| view_grid / list | Grid / List | 网格 / 列表 | 格狀 / 清單 | TW: 清單 |
| select_all | Select All | 全选 | 全選 | |
| n_selected | {n} selected | 已选择 {n} 项 | 已選取 {n} 個項目 | TW: 選取 |
| preview | Preview | 预览 | 預覽 | |
| thumbnail | Thumbnail | 缩略图 | 縮圖 | |
| storage | Storage | 存储空间 | 儲存空間 | |
| storage_used | {used} of {total} used | 已使用 {used}（共 {total}） | 已使用 {used}（共 {total}） | |
| empty_folder | This folder is empty | 此文件夹为空 | 這個資料夾是空的 | invite: “Drop files here to upload” |
| drop_here | Drop files here to upload | 将文件拖到此处上传 | 將檔案拖曳到這裡上傳 | |
| loading | Loading… | 正在载入… | 載入中… | |
| offline | You are offline | 当前无网络连接 | 目前沒有網路連線 | TW: 網路 |
| retry | Try Again | 重试 | 重試 | |
| cancel | Cancel | 取消 | 取消 | |
| done | Done | 完成 | 完成 | |
| save | Save | 保存 | 儲存 | TW: 儲存 |
| undo | Undo | 撤销 | 復原 | TW: 復原 |

## C. Web app: account, sign-in, admin

| Key | English | 简体中文 | 繁體中文 (Taiwan) | Notes |
|---|---|---|---|---|
| sign_in | Sign In | 登录 | 登入 | |
| sign_out | Sign Out | 退出登录 | 登出 | |
| register | Create Account | 创建账户 | 建立帳號 | |
| account | Account | 账户 | 帳號 | TW: 帳號 |
| display_name | Display name | 显示名称 | 顯示名稱 | |
| email | Email | 邮箱 | 電子郵件 | |
| phone | Phone number | 手机号 | 手機號碼 | |
| email_or_phone | Email or phone number | 邮箱或手机号 | 電子郵件或手機號碼 | login identifier |
| password | Password | 密码 | 密碼 | |
| change_password | Change Password | 修改密码 | 變更密碼 | TW: 變更 |
| one_time_code | One-time code | 验证码 | 驗證碼 | |
| send_code | Send Code | 发送验证码 | 傳送驗證碼 | TW: 傳送 |
| verify | Verify | 验证 | 驗證 | |
| wrong_credentials | The email, phone number or password is not right. | 邮箱、手机号或密码不正确。 | 電子郵件、手機號碼或密碼不正確。 | |
| profile | Profile | 个人资料 | 個人資料 | |
| language | Language | 语言 | 語言 | picker lists 简体中文, 繁體中文, English by their own names |
| appearance | Appearance | 外观 | 外觀 | |
| admin | Administrator | 管理员 | 管理員 | |
| software_update | Software Update | 软件更新 | 軟體更新 | TW: 軟體 |
| current_version | Current version | 当前版本 | 目前版本 | TW: 目前 |
| update_available | Update available | 有可用更新 | 有可用的更新 | |
| release_notes | Release notes | 发行说明 | 版本資訊 | |
| install_update | Install Update | 安装更新 | 安裝更新 | |
| update_mode | Update mode | 更新方式 | 更新方式 | |
| manual / automatic | Manual / Automatic | 手动 / 自动 | 手動 / 自動 | |
| update_window | Update window | 更新时段 | 更新時段 | |
| channel | Channel | 渠道 | 發行管道 | |
| updating | Updating, reconnecting… | 正在更新，正在重新连接… | 正在更新，重新連線中… | calm, not an error (R-027) |
| new_version | A new version is available | 有新版本可用 | 有新版本可用 | |
| reload | Reload | 重新载入 | 重新載入 | |

## D. Approval states (R-062)

Screens: after registering in `approval` mode (`202`), and on login errors `403 ACCOUNT_PENDING` / `ACCOUNT_REJECTED`, `REGISTRATION_CLOSED`, `REGISTRATION_BUSY`. Tone: calm and plain, no blame, tell people what happens next.

| Key | English | 简体中文 | 繁體中文 (Taiwan) | Notes |
|---|---|---|---|---|
| approval.submitted_title | Request submitted | 请求已提交 | 申請已送出 | |
| approval.submitted_body | Your request is waiting for the administrator’s approval. You will be able to sign in once it is approved. | 你的请求正在等待管理员批准。批准后即可登录。 | 你的申請正在等待管理員核准，核准後即可登入。 | TW: 核准 for approve |
| approval.waiting | Waiting for approval | 等待批准 | 等待核准 | status line |
| approval.pending_title | Your account is pending | 账户待审核 | 帳號審核中 | `ACCOUNT_PENDING` |
| approval.pending_body | The administrator has not approved this account yet. Try signing in again later. | 管理员尚未批准此账户，请稍后再试。 | 管理員尚未核准這個帳號，請稍後再試。 | correct password, not yet approved |
| approval.rejected_title | Request not approved | 请求未获批准 | 申請未獲核准 | `ACCOUNT_REJECTED` (UI label "Rejected" = 已拒绝 / 已拒絕) |
| approval.rejected_body | The administrator did not approve this account. If you think this is a mistake, contact them directly. | 管理员没有批准此账户。如果你认为有误，请直接联系管理员。 | 管理員沒有核准這個帳號。如果你認為有誤，請直接聯絡管理員。 | no owner contact shown |
| approval.rejected_short | Rejected | 已拒绝 | 已拒絕 | |
| approval.approved_short | Approved | 已批准 | 已核准 | |
| approval.closed_title | Registration is closed | 暂不开放注册 | 目前不開放註冊 | `REGISTRATION_CLOSED` |
| approval.closed_body | This Cloud Home is not accepting new accounts. Ask the administrator for access. | 这台 Cloud Home 目前不接受新账户。请向管理员申请。 | 這台 Cloud Home 目前不接受新帳號，請向管理員申請。 | |
| approval.busy_title | Too many requests right now | 当前申请过多 | 目前申請人數過多 | `REGISTRATION_BUSY` |
| approval.busy_body | Please try again in a little while. | 请稍后再试。 | 請稍後再試。 | |
| approval.back_to_sign_in | Back to Sign In | 返回登录 | 返回登入 | |
| approval.admin_pending_n | {n} accounts waiting for approval | {n} 个账户等待批准 | {n} 個帳號等待核准 | admin badge |
| approval.approve / reject | Approve / Reject | 批准 / 拒绝 | 核准 / 拒絕 | admin buttons (report site uses 批准 / 拒绝 too) |

### Notes for Traditional Chinese reviewers

- 核准 is used for administrative approval (申請核准); 批准 is also understood, but 核准 is the usual Taiwan wording.
- 垃圾桶 (trash), 儲存 (save), 復原 (undo), 還原 (restore), 清單 (list), 連結 (link), 網路 (network), 軟體 (software), 登入 / 登出 are the Taiwan forms; do not substitute the mainland forms.
- Longest strings to test at 390px: `approval.pending_body`, `approval.rejected_body`, `renamed_as`, `trash_days_left`, `wrong_credentials`.

---

## E. Web app: complete string table (R-002)

Generated from `design/mockups/app/assets/i18n.js` by `bun design/mockups/app/tools/glossary.js`: edit the .js file and regenerate, do not hand-edit this table. Placeholders `{n}`, `{name}`, `{folder}`, `{date}`, `{size}` are filled by the app; dates, sizes and numbers go through `Intl` (never concatenate). Keys starting with `a11y.` are for screen readers only.

Wording decisions (Traditional is native Taiwan usage, not converted):

| Concept | English | 简体中文 | 繁體中文 (Taiwan) | Why |
|---|---|---|---|---|
| Approve (admin) | Approve | 批准 | 核准 | 核准 is the usual Taiwan administrative term |
| Pre-release | Pre-release | 预发布版 | 預先發行版 |  |
| Release notes | Release notes | 发行说明 | 版本資訊 | Taiwan software says 版本資訊 or 版本說明 |
| Sign-in methods | Ways to sign in | 登录方式 | 登入方式 | 登入 is the Taiwan verb |
| Documents (kind) | Documents | 文档 | 文件 | in Taiwan 文件 is a document; a file is 檔案 |
| Video, audio | Videos, Audio | 视频，音频 | 影片，音訊 |  |
| Network | no connection | 无网络连接 | 沒有網路連線 | 網路, not 网络 |
| Link | Link | 链接 | 連結 |  |
| Selected | selected | 已选择 | 已選取 | 選取 for selection |
| Software | Software Update | 软件更新 | 軟體更新 |  |
| Storage | Storage | 存储空间 | 儲存空間 |  |
| Disk | Drive | 硬盘 | 硬碟 |  |
| Text message | text message | 短信 | 簡訊 | 簡訊 is the Taiwan term; 短信 is mainland |
| Shared with you | Shared with You | 分享给你 | 與你分享 | Taiwan phrasing is 與你分享 (not 分享給你) |
| Mainland China | Mainland China | 中国大陆 | 中國大陸 | country list; 臺灣 is written with 臺 in Taiwan official usage |
| South Korea | South Korea | 韩国 | 南韓 | Taiwan says 南韓 |
| Australia | Australia | 澳大利亚 | 澳洲 | Taiwan says 澳洲 |
| Quotes | “ ” | “ ” | 「 」 | corner quotes in Traditional |

| Key | English | 简体中文 | 繁體中文 (Taiwan) |
|---|---|---|---|
| app.name | Cloud Home | Cloud Home | Cloud Home |
| nav.files | Files | 文件 | 檔案 |
| nav.shared | Shared | 已分享 | 已分享 |
| nav.trash | Trash | 回收站 | 垃圾桶 |
| nav.storage | Storage | 存储空间 | 儲存空間 |
| nav.settings | Settings | 设置 | 設定 |
| nav.admin | Administration | 管理 | 管理 |
| nav.sidebar | Sidebar | 侧边栏 | 側邊欄 |
| nav.main | Main navigation | 主导航 | 主要導覽 |
| a11y.skip | Skip to files | 跳到文件列表 | 跳到檔案清單 |
| a11y.more | More actions | 更多操作 | 更多動作 |
| a11y.select | Select | 选择 | 選取 |
| upload | Upload | 上传 | 上傳 |
| upload_files | Upload Files | 上传文件 | 上傳檔案 |
| upload_folder | Upload Folder | 上传文件夹 | 上傳資料夾 |
| new_folder | New Folder | 新建文件夹 | 新增資料夾 |
| search_placeholder | Search files | 搜索文件 | 搜尋檔案 |
| search_results_for | Results for “{q}” | “{q}”的搜索结果 | 「{q}」的搜尋結果 |
| search_found | {n} items found | 找到 {n} 个项目 | 找到 {n} 個項目 |
| clear | Clear | 清除 | 清除 |
| view_grid | Grid | 网格 | 格狀 |
| view_list | List | 列表 | 清單 |
| view_as | View as | 显示方式 | 檢視方式 |
| sort_by | Sort by | 排序方式 | 排序方式 |
| col.name | Name | 名称 | 名稱 |
| col.size | Size | 大小 | 大小 |
| col.modified | Date Modified | 修改日期 | 修改日期 |
| col.kind | Kind | 种类 | 種類 |
| folder | Folder | 文件夹 | 資料夾 |
| items_n | {n} items | {n} 个项目 | {n} 個項目 |
| n_selected | {n} selected | 已选择 {n} 项 | 已選取 {n} 個項目 |
| select_all | Select All | 全选 | 全選 |
| deselect | Deselect | 取消选择 | 取消選取 |
| open | Open | 打开 | 打開 |
| rename | Rename | 重命名 | 重新命名 |
| move | Move | 移动 | 移動 |
| copy | Copy | 复制 | 複製 |
| download | Download | 下载 | 下載 |
| share | Share | 分享 | 分享 |
| delete | Move to Trash | 移到回收站 | 移至垃圾桶 |
| moved_to_trash | {n} items moved to Trash | 已将 {n} 个项目移到回收站 | 已將 {n} 個項目移至垃圾桶 |
| undo | Undo | 撤销 | 復原 |
| empty_title | This folder is empty | 此文件夹为空 | 這個資料夾是空的 |
| empty_text | Drop files here, or use Upload to add photos, music, videos and documents. | 将文件拖到此处，或点“上传”添加照片、音乐、视频和文档。 | 將檔案拖曳到這裡，或使用「上傳」加入照片、音樂、影片與文件。 |
| drop_here | Drop files here to upload | 将文件拖到此处上传 | 將檔案拖曳到這裡上傳 |
| drop_into | They will be added to “{folder}” | 文件将添加到“{folder}” | 檔案將加入「{folder}」 |
| no_results | No results | 没有结果 | 沒有結果 |
| no_results_text | Nothing in your files matches “{q}”. Check the spelling or try a shorter word. | 你的文件中没有与“{q}”匹配的项目。请检查拼写，或换一个更短的词。 | 你的檔案中沒有符合「{q}」的項目。請檢查拼字，或試試較短的字詞。 |
| loading | Loading… | 正在载入… | 載入中… |
| error_load_title | Couldn’t load this folder | 无法载入此文件夹 | 無法載入這個資料夾 |
| error_load_text | The drive did not answer. Check that it is plugged in, then try again. | 硬盘没有响应。请确认已接好，然后重试。 | 硬碟沒有回應。請確認已接好，然後再試一次。 |
| error_drive_title | The drive is not connected | 硬盘未连接 | 硬碟未連接 |
| error_drive_text | Your files are safe. Plug the drive back in and Cloud Home will pick up where it left off. | 你的文件是安全的。重新接上硬盘后，Cloud Home 会自动继续。 | 你的檔案很安全。重新接上硬碟後，Cloud Home 會自動繼續。 |
| name_taken | An item with this name already exists here. Choose another name. | 此处已有同名项目，请换一个名称。 | 這裡已有同名項目，請換一個名稱。 |
| session_expired | Please sign in again | 请重新登录 | 請重新登入 |
| retry | Try Again | 重试 | 重試 |
| offline_title | You are offline | 当前无网络连接 | 目前沒有網路連線 |
| offline_text | You can keep browsing what is already loaded. Uploads wait and resume when you are back online. | 已载入的内容仍可浏览。上传会暂停，恢复连接后自动继续。 | 已載入的內容仍可瀏覽。上傳會暫停，恢復連線後自動繼續。 |
| offline_short | Offline. Changes will wait. | 无网络连接，更改将等待。 | 沒有網路連線，變更會等待。 |
| reconnected | Back online | 已恢复连接 | 已恢復連線 |
| cancel | Cancel | 取消 | 取消 |
| done | Done | 完成 | 完成 |
| close | Close | 关闭 | 關閉 |
| save | Save | 保存 | 儲存 |
| create | Create | 创建 | 建立 |
| folder_name | Folder name | 文件夹名称 | 資料夾名稱 |
| untitled_folder | Untitled Folder | 未命名文件夹 | 未命名資料夾 |
| uploading_n | Uploading {n} items | 正在上传 {n} 个项目 | 正在上傳 {n} 個項目 |
| upload_done | Upload complete | 上传完成 | 上傳完成 |
| upload_done_n | {n} items uploaded | 已上传 {n} 个项目 | 已上傳 {n} 個項目 |
| upload_cancel | Cancel upload | 取消上传 | 取消上傳 |
| upload_cancelled | Cancelled | 已取消 | 已取消 |
| upload_failed | Upload failed | 上传失败 | 上傳失敗 |
| upload_failed_hint | Connection lost. Try again. | 连接中断，请重试。 | 連線中斷，請再試一次。 |
| upload_waiting | Waiting | 等待中 | 等待中 |
| upload_paused_offline | Paused, waiting for connection | 已暂停，等待网络连接 | 已暫停，等待網路連線 |
| upload_minimize | Minimize | 收起 | 收合 |
| upload_expand | Show details | 展开详情 | 展開詳細資料 |
| upload_remaining | {t} left | 还剩 {t} | 剩下 {t} |
| upload_to | to {folder} | 上传到 {folder} | 上傳至 {folder} |
| renamed_as | Saved as “{name}” because that name was taken | 因名称已存在，已保存为“{name}” | 因名稱已存在，已儲存為「{name}」 |
| clear_finished | Clear Finished | 清除已完成项 | 清除已完成項目 |
| preview | Preview | 预览 | 預覽 |
| prev | Previous | 上一个 | 上一個 |
| next | Next | 下一个 | 下一個 |
| n_of_m | {n} of {m} | 第 {n} 项，共 {m} 项 | 第 {n} 項，共 {m} 項 |
| play | Play | 播放 | 播放 |
| pause | Pause | 暂停 | 暫停 |
| volume | Volume | 音量 | 音量 |
| position | Playback position | 播放位置 | 播放位置 |
| fullscreen | Full Screen | 全屏 | 全螢幕 |
| info | Info | 信息 | 資訊 |
| media_error_title | This file can’t be played here | 无法在此播放此文件 | 無法在這裡播放這個檔案 |
| media_error_text | Your browser doesn’t support this format. You can still download it. | 你的浏览器不支持此格式，但仍可下载。 | 你的瀏覽器不支援這個格式，但仍可下載。 |
| download_n | Download {n} items | 下载 {n} 个项目 | 下載 {n} 個項目 |
| download_as | Download as | 下载格式 | 下載格式 |
| download_zip | ZIP | ZIP | ZIP |
| download_zip_hint | Opens everywhere. Best for Windows and phones. | 通用，适合 Windows 与手机。 | 通用，適合 Windows 與手機。 |
| download_targz | tar.gz | tar.gz | tar.gz |
| download_targz_hint | Smaller for many text files. Best for Mac and Linux. | 大量文本文件时更小，适合 Mac 与 Linux。 | 大量文字檔時更小，適合 Mac 與 Linux。 |
| download_includes | Includes {n} folders and everything inside them | 包含 {n} 个文件夹及其中所有内容 | 包含 {n} 個資料夾及其中所有內容 |
| download_size | About {size} | 约 {size} | 約 {size} |
| download_start | Download | 下载 | 下載 |
| download_preparing | Preparing your download… | 正在准备下载… | 正在準備下載… |
| download_started | Your download has started. It streams straight to your device, so it can begin before the archive is finished. | 下载已开始。内容会直接流式传输到你的设备，因此无需等待打包完成。 | 下載已開始。內容會直接串流到你的裝置，因此不必等待封存完成。 |
| trash_title | Trash | 回收站 | 垃圾桶 |
| trash_note | Items are deleted for good after 7 days. | 项目将在 7 天后永久删除。 | 項目會在 7 天後永久刪除。 |
| trash_days_left | {n} days left | 还剩 {n} 天 | 剩下 {n} 天 |
| trash_day_left | 1 day left | 还剩 1 天 | 剩下 1 天 |
| trash_today | Deletes today | 今天将被删除 | 今天將被刪除 |
| empty_trash | Empty Trash | 清空回收站 | 清空垃圾桶 |
| restore | Restore | 恢复 | 還原 |
| delete_forever | Delete Forever | 永久删除 | 永久刪除 |
| trash_empty_title | Trash is empty | 回收站是空的 | 垃圾桶是空的 |
| trash_empty_text | Items you delete wait here for 7 days before they are removed for good. | 你删除的项目会在这里保留 7 天，之后才会永久移除。 | 你刪除的項目會在這裡保留 7 天，之後才會永久移除。 |
| confirm_empty_title | Empty the Trash? | 要清空回收站吗？ | 要清空垃圾桶嗎？ |
| confirm_empty_text | {n} items ({size}) will be deleted for good. This can’t be undone. | {n} 个项目（{size}）将被永久删除，此操作无法撤销。 | {n} 個項目（{size}）將被永久刪除，此操作無法復原。 |
| confirm_delete_title | Delete “{name}” forever? | 要永久删除“{name}”吗？ | 要永久刪除「{name}」嗎？ |
| confirm_delete_text | This item will be removed from the drive right away. This can’t be undone. | 此项目会立即从硬盘中移除，无法撤销。 | 這個項目會立即從硬碟中移除，無法復原。 |
| restored_to | Restored to {folder} | 已恢复到 {folder} | 已還原到 {folder} |
| restored_root | Original folder was deleted. Restored to Files. | 原文件夹已删除，已恢复到“文件”。 | 原本的資料夾已刪除，已還原到「檔案」。 |
| from_folder | from {folder} | 来自 {folder} | 來自 {folder} |
| shared_title | Shared by You | 你分享的内容 | 你分享的內容 |
| shared_empty_title | Nothing is shared | 还没有分享任何内容 | 尚未分享任何內容 |
| shared_empty_text | Choose a file or folder, then Share, to make a link anyone can open. | 选择文件或文件夹，再点“分享”，即可生成任何人都能打开的链接。 | 選取檔案或資料夾，再按「分享」，即可產生任何人都能打開的連結。 |
| share_link | Share link | 分享链接 | 分享連結 |
| share_create | Create Link | 创建链接 | 建立連結 |
| copy_link | Copy Link | 复制链接 | 複製連結 |
| link_copied | Link copied | 链接已复制 | 連結已複製 |
| link_expires | Link expires | 链接有效期 | 連結有效期限 |
| exp_1d | After 1 day | 1 天后 | 1 天後 |
| exp_7d | After 7 days | 7 天后 | 7 天後 |
| exp_30d | After 30 days | 30 天后 | 30 天後 |
| exp_never | Never | 永不过期 | 永不過期 |
| allow_download | Allow downloads | 允许下载 | 允許下載 |
| allow_download_hint | When off, people can view the file but not save it. | 关闭后，对方只能查看，不能保存文件。 | 關閉後，對方只能檢視，無法儲存檔案。 |
| anyone_with_link | Anyone with the link can open it | 任何拿到链接的人都能打开 | 任何拿到連結的人都能打開 |
| revoke_link | Stop Sharing | 停止分享 | 停止分享 |
| revoke_title | Stop sharing “{name}”? | 要停止分享“{name}”吗？ | 要停止分享「{name}」嗎？ |
| revoke_text | The link will stop working right away. | 该链接会立即失效。 | 該連結會立即失效。 |
| expires_on | Expires {date} | {date} 到期 | {date} 到期 |
| expired | Expired | 已过期 | 已過期 |
| never_expires | Never expires | 永不过期 | 永不過期 |
| opened_n | Opened {n} times | 已被打开 {n} 次 | 已被開啟 {n} 次 |
| download_on | Downloads on | 允许下载 | 允許下載 |
| download_off | View only | 仅查看 | 僅供檢視 |
| shared_by | Shared by {name} | {name} 分享的文件 | {name} 分享的檔案 |
| public_note | You don’t need an account to open this. | 无需账户即可打开。 | 不需要帳號即可打開。 |
| public_gone_title | This link isn’t available | 此链接已不可用 | 這個連結已無法使用 |
| public_gone_text | It may have expired or been turned off by the person who shared it. | 它可能已过期，或已被分享者关闭。 | 它可能已過期，或已被分享者關閉。 |
| storage_used | {used} of {total} used | 已使用 {used}（共 {total}） | 已使用 {used}（共 {total}） |
| storage_free | {free} available | 可用 {free} | 可用 {free} |
| storage_drive | Drive | 硬盘 | 硬碟 |
| storage_drive_name | Home SSD | 家用 SSD | 家用 SSD |
| storage_online | Connected | 已连接 | 已連接 |
| storage_yours | Your files | 你的文件 | 你的檔案 |
| storage_by_kind | By kind | 按种类 | 依種類 |
| kind.photos | Photos | 照片 | 照片 |
| kind.videos | Videos | 视频 | 影片 |
| kind.audio | Audio | 音频 | 音訊 |
| kind.documents | Documents | 文档 | 文件 |
| kind.other | Other | 其他 | 其他 |
| kind.trash | In Trash | 回收站中 | 垃圾桶中 |
| storage_low | Less than 10% of the drive is free | 硬盘可用空间不足 10% | 硬碟可用空間不足 10% |
| storage_manage | Review large files | 查看大文件 | 檢視大型檔案 |
| sign_in | Sign In | 登录 | 登入 |
| sign_in_title | Sign in to Cloud Home | 登录 Cloud Home | 登入 Cloud Home |
| sign_in_sub | Your family’s files, on your own drive. | 你们家的文件，存放在自己的硬盘上。 | 你們家的檔案，存放在自己的硬碟上。 |
| sign_out | Sign Out | 退出登录 | 登出 |
| register | Create Account | 创建账户 | 建立帳號 |
| register_title | Create your account | 创建你的账户 | 建立你的帳號 |
| register_sub | Use an email or a phone number. You can add the other later. | 使用邮箱或手机号。另一项以后可再添加。 | 使用電子郵件或手機號碼，另一項之後可再加入。 |
| email_or_phone | Email or phone number | 邮箱或手机号 | 電子郵件或手機號碼 |
| email | Email | 邮箱 | 電子郵件 |
| phone | Phone number | 手机号 | 手機號碼 |
| display_name | Display name | 显示名称 | 顯示名稱 |
| password | Password | 密码 | 密碼 |
| password_hint | At least 8 characters | 至少 8 个字符 | 至少 8 個字元 |
| show_password | Show password | 显示密码 | 顯示密碼 |
| hide_password | Hide password | 隐藏密码 | 隱藏密碼 |
| optional | Optional | 选填 | 選填 |
| no_account | New here? | 还没有账户？ | 還沒有帳號？ |
| have_account | Already have an account? | 已有账户？ | 已有帳號？ |
| wrong_credentials | The email, phone number or password is not right. | 邮箱、手机号或密码不正确。 | 電子郵件、手機號碼或密碼不正確。 |
| field_required | Enter your email or phone number. | 请输入邮箱或手机号。 | 請輸入電子郵件或手機號碼。 |
| approval.submitted_title | Request submitted | 请求已提交 | 申請已送出 |
| approval.submitted_body | Your request is waiting for the administrator’s approval. You will be able to sign in once it is approved. | 你的请求正在等待管理员批准。批准后即可登录。 | 你的申請正在等待管理員核准，核准後即可登入。 |
| approval.waiting | Waiting for approval | 等待批准 | 等待核准 |
| approval.pending_title | Your account is pending | 账户待审核 | 帳號審核中 |
| approval.pending_body | The administrator has not approved this account yet. Try signing in again later. | 管理员尚未批准此账户，请稍后再试。 | 管理員尚未核准這個帳號，請稍後再試。 |
| approval.rejected_title | Request not approved | 请求未获批准 | 申請未獲核准 |
| approval.rejected_body | The administrator did not approve this account. If you think this is a mistake, contact them directly. | 管理员没有批准此账户。如果你认为有误，请直接联系管理员。 | 管理員沒有核准這個帳號。如果你認為有誤，請直接聯絡管理員。 |
| approval.closed_title | Registration is closed | 暂不开放注册 | 目前不開放註冊 |
| approval.closed_body | This Cloud Home is not accepting new accounts. Ask the administrator for access. | 这台 Cloud Home 目前不接受新账户。请向管理员申请。 | 這台 Cloud Home 目前不接受新帳號，請向管理員申請。 |
| approval.busy_title | Too many requests right now | 当前申请过多 | 目前申請人數過多 |
| approval.busy_body | Please try again in a little while. | 请稍后再试。 | 請稍後再試。 |
| approval.back_to_sign_in | Back to Sign In | 返回登录 | 返回登入 |
| approval.step1 | Request sent | 请求已发送 | 申請已送出 |
| approval.step2 | Administrator reviews | 管理员审核 | 管理員審核 |
| approval.step3 | You can sign in | 你可以登录 | 你可以登入 |
| approval.pending_n | {n} accounts waiting for approval | {n} 个账户等待批准 | {n} 個帳號等待核准 |
| approval.approve | Approve | 批准 | 核准 |
| approval.reject | Reject | 拒绝 | 拒絕 |
| approval.registered | Registered {date} | {date} 注册 | {date} 註冊 |
| account | Account | 账户 | 帳號 |
| profile | Profile | 个人资料 | 個人資料 |
| language | Language | 语言 | 語言 |
| appearance | Appearance | 外观 | 外觀 |
| ap_auto | Automatic | 自动 | 自動 |
| ap_light | Light | 浅色 | 淺色 |
| ap_dark | Dark | 深色 | 深色 |
| sign_in_methods | Ways to sign in | 登录方式 | 登入方式 |
| verified | Verified | 已验证 | 已驗證 |
| not_verified | Not verified | 未验证 | 未驗證 |
| verify | Verify | 验证 | 驗證 |
| add | Add | 添加 | 加入 |
| change | Change | 更改 | 變更 |
| remove | Remove | 移除 | 移除 |
| change_password | Change Password | 修改密码 | 變更密碼 |
| current_password | Current password | 当前密码 | 目前的密碼 |
| new_password | New password | 新密码 | 新密碼 |
| one_time_code | One-time code | 验证码 | 驗證碼 |
| send_code | Send Code | 发送验证码 | 傳送驗證碼 |
| code_sent_to | We sent a 6-digit code to {to}. | 我们已向 {to} 发送 6 位验证码。 | 我們已將 6 位數驗證碼傳送到 {to}。 |
| code_resend_in | Send again in {s} s | {s} 秒后可重新发送 | {s} 秒後可重新傳送 |
| keep_one | Keep at least one way to sign in. | 至少保留一种登录方式。 | 至少要保留一種登入方式。 |
| saved | Saved | 已保存 | 已儲存 |
| danger_zone | Sessions | 登录会话 | 登入工作階段 |
| managed_by_admin | Managed by the administrator | 由管理员管理 | 由管理員管理 |
| admin_title | Administration | 管理 | 管理 |
| software_update | Software Update | 软件更新 | 軟體更新 |
| current_version | Current version | 当前版本 | 目前版本 |
| latest_version | Latest version | 最新版本 | 最新版本 |
| up_to_date | Cloud Home is up to date | Cloud Home 已是最新版本 | Cloud Home 已是最新版本 |
| checked_at | Last checked {t} | 上次检查：{t} | 上次檢查：{t} |
| update_available | Update available | 有可用更新 | 有可用的更新 |
| update_covers | One version covers the server and this web app. Both update together. | 服务器与此网页应用是同一个版本，会一起更新。 | 伺服器與這個網頁 App 是同一個版本，會一併更新。 |
| release_notes | Release notes | 发行说明 | 版本資訊 |
| install_update | Install Update | 安装更新 | 安裝更新 |
| check_now | Check Now | 立即检查 | 立即檢查 |
| checking | Checking for updates… | 正在检查更新… | 正在檢查更新… |
| phase.downloading | Downloading {pct}% | 正在下载 {pct}% | 正在下載 {pct}% |
| phase.installing | Verifying and installing | 正在验证并安装 | 正在驗證並安裝 |
| phase.restarting | Restarting | 正在重新启动 | 正在重新啟動 |
| phase.waiting | Waiting for the new version to come back… | 正在等待新版本恢复… | 正在等待新版本恢復… |
| phase.done | Updated to {v} | 已更新到 {v} | 已更新至 {v} |
| phase.failed | The update didn’t finish | 更新未能完成 | 更新未能完成 |
| phase.failed_text | Cloud Home went back to the version that worked. Nothing was lost. {err} | Cloud Home 已回到可正常运行的旧版本，没有丢失任何内容。{err} | Cloud Home 已回到可正常運作的舊版本，沒有遺失任何內容。{err} |
| busy_transfers | {n} transfers are running. Installing will interrupt them. | 有 {n} 个传输正在进行，安装会中断它们。 | 有 {n} 個傳輸正在進行，安裝會中斷它們。 |
| install_anyway | Install Anyway | 仍然安装 | 仍要安裝 |
| update_mode | Update mode | 更新方式 | 更新方式 |
| manual | Manual | 手动 | 手動 |
| automatic | Automatic | 自动 | 自動 |
| update_window | Update window | 更新时段 | 更新時段 |
| update_window_hint | Automatic updates only start in this window. | 自动更新只会在此时段开始。 | 自動更新只會在這個時段開始。 |
| channel | Channel | 渠道 | 發行管道 |
| ch_stable | Stable | 稳定版 | 穩定版 |
| ch_pre | Pre-release | 预发布版 | 預先發行版 |
| ch_hint | Pre-release builds arrive sooner and may have rough edges. | 预发布版更早到达，但可能不够完善。 | 預先發行版較早推出，但可能不夠完善。 |
| accounts | Accounts | 账户 | 帳號 |
| registration_mode | New accounts | 新账户 | 新帳號 |
| new_version | A new version is available | 有新版本可用 | 有新版本可用 |
| new_version_text | Reload to switch to {v}. Your uploads are not interrupted. | 重新载入即可切换到 {v}，不会中断你的上传。 | 重新載入即可切換至 {v}，不會中斷你的上傳。 |
| reload | Reload | 重新载入 | 重新載入 |
| later | Later | 稍后 | 稍後 |
| reload_after_upload | Will reload when your upload finishes | 上传完成后将自动重新载入 | 上傳完成後會自動重新載入 |
| updating | Updating, reconnecting… | 正在更新，正在重新连接… | 正在更新，重新連線中… |
| updating_text | Cloud Home is restarting with the new version. This usually takes under a minute. | Cloud Home 正在以新版本重新启动，通常不到一分钟。 | Cloud Home 正以新版本重新啟動，通常不到一分鐘。 |
| updating_retry | Trying again in {s} s | {s} 秒后重试 | {s} 秒後重試 |
| sse_reconnecting | Reconnecting… | 正在重新连接… | 重新連線中… |
| shared_with_title | Shared with You | 分享给你的内容 | 與你分享的內容 |
| shared_with_lede | Files and folders other people on this Cloud Home shared with you. | 这台 Cloud Home 上其他人分享给你的文件和文件夹。 | 這台 Cloud Home 上其他人與你分享的檔案與資料夾。 |
| shared_with_empty_title | Nothing has been shared with you | 还没有人分享内容给你 | 目前沒有人與你分享內容 |
| shared_with_empty_text | When someone shares a file or folder with you, it appears here. | 有人把文件或文件夹分享给你时，会显示在这里。 | 有人與你分享檔案或資料夾時，會顯示在這裡。 |
| shared_from | From {name} | 来自 {name} | 來自 {name} |
| shared_on | Shared {date} | {date} 分享 | {date} 分享 |
| shared_new | New | 新 | 新 |
| shared_tabs | Shared items | 分享的内容 | 分享的內容 |
| aud_label | Who can open it | 谁可以打开 | 誰可以打開 |
| aud_link | Anyone with the link | 任何拥有链接的人 | 任何擁有連結的人 |
| aud_people | Specific people | 指定的人 | 指定的對象 |
| aud_people_hint | They see it under Shared with You. No link is created. | 他们会在“分享给你的内容”中看到，不会生成链接。 | 他們會在「與你分享的內容」中看到，不會產生連結。 |
| shared_with_names | Shared with {names}. | 已分享给 {names}。 | 已與 {names} 分享。 |
| aud_none_chosen | Choose at least one person | 请至少选择一个人 | 請至少選擇一位 |
| phone_change_title | Change Phone Number | 更改手机号 | 變更手機號碼 |
| country_region | Country or Region | 国家或地区 | 國家或地區 |
| country_search | Search countries and regions | 搜索国家和地区 | 搜尋國家與地區 |
| country_suggested | Suggested | 推荐 | 建議 |
| country_all | All Countries and Regions | 所有国家和地区 | 所有國家與地區 |
| country_none | No matching country or region | 没有匹配的国家或地区 | 沒有符合的國家或地區 |
| phone_hint | We’ll send a code by text message to confirm it. | 我们会发送短信验证码来确认。 | 我們會傳送簡訊驗證碼來確認。 |
| phone_bad | This number doesn’t look right for {country}. Check the digits. | 这个号码与{country}的格式不符，请检查数字。 | 這個號碼與{country}的格式不符，請檢查數字。 |
| phone_exists | This number is already used by another account. | 这个号码已被其他账户使用。 | 這個號碼已被其他帳號使用。 |
| id_placeholder | name@example.com or +1 555 0134 | name@example.com 或 +86 138 0013 8000 | name@example.com 或 +886 912 345 678 |
| c.US | United States | 美国 | 美國 |
| c.CA | Canada | 加拿大 | 加拿大 |
| c.GB | United Kingdom | 英国 | 英國 |
| c.AU | Australia | 澳大利亚 | 澳洲 |
| c.CN | Mainland China | 中国大陆 | 中國大陸 |
| c.HK | Hong Kong | 中国香港 | 香港 |
| c.MO | Macao | 中国澳门 | 澳門 |
| c.TW | Taiwan | 中国台湾 | 臺灣 |
| c.JP | Japan | 日本 | 日本 |
| c.KR | South Korea | 韩国 | 南韓 |
| c.SG | Singapore | 新加坡 | 新加坡 |
| c.MY | Malaysia | 马来西亚 | 馬來西亞 |
| c.DE | Germany | 德国 | 德國 |
| c.FR | France | 法国 | 法國 |

Longest strings to test at 390 px in each language: `approval.rejected_body`, `approval.submitted_body`, `renamed_as`, `phase.failed_text`, `confirm_empty_text`, `download_targz_hint`, `ch_hint`, `upload_paused_offline`, `no_results_text`.
