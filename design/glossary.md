# Cloud Home glossary

One source for UI wording, shared with the frontend engineer. Add a term here before it appears in a screen.

**Which table applies where**

| Table | Used by | Languages |
|---|---|---|
| A. Report site | `manager-site/` (public report site, R-001) | English, 简体中文 only |
| B, C, D | the web app in `ui/` (R-005, R-006 to R-013, R-062) | English, 简体中文, 繁體中文 |

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
