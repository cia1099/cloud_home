# Cloud Home — 在线更新（OTA）

设备自己检查 GitHub Release、验签下载、自检、切换版本并重启；新版本起不来则自动回滚二进制与数据库。

## 流程

```
git tag vX.Y.Z ─► GitHub Actions (.github/workflows/release.yml)
                  ├─ 构建 3 个目标：aarch64/x86_64 Linux(musl)、aarch64 macOS
                  ├─ 生成 manifest.json（每目标 url + sha256 + size）
                  └─ 用 minisign 私钥签名 → manifest.json.minisig → 发布 Release

设备：update::checker（每 UPDATE_CHECK_INTERVAL_HOURS 小时）
  拉 manifest.json + .minisig → 用内置公钥验签（先验再解析）→ semver 比较
  → 发现新版本：写入状态 + SSE `update` 事件
  → manual：等人安装；auto：时间窗内且无在途传输时自动安装
安装：下载(流式 sha256) → 只解出 cloud_home 一个文件 → --self-test
     → VACUUM INTO 备份数据库 → 写 pending 状态 → 原子切换 previous/current 链接
     → 触发优雅关停 → systemd/launchd 拉起 current/cloud_home
启动守卫（迁移之前）：pending==本版本则 attempts+1；>3 次未提交 → 回滚
提交：服务稳定运行 2 分钟后清除 pending
```

运行时可通过管理端点查看 / 触发 / 改设置，见 `spec/backend.md`「在线更新端点」（`/admin/update*`）。

## 安装目录

```
$UPDATE_ROOT (默认 /opt/cloud_home)
  versions/<ver>/cloud_home       只保留 current 与 previous 两个
  current  -> versions/<ver>      守护进程运行 current/cloud_home
  previous -> versions/<ver>
  update/state.json               pending / attempts / backup / last_failed
  update/settings.json            运行时覆盖 .env 的更新设置
  update/staging/                 下载中的安装包
```

**更新器仅在 `current` 是符号链接时启用**，所以开发机 `cargo run` 不会自更新。

首次安装由 `packaging/linux/install.sh`（systemd，`Restart=always`，用户 `cloudhome`）与
`packaging/macos/install.sh`（launchd LaunchDaemon，`KeepAlive`，以 `sudo` 的调用用户运行）完成：
先用脚本内嵌的公钥（须与 `update::PUBLIC_KEY` 一致，有单元测试守护）验 `manifest.json` 签名，再按清单里本机
目标的 sha256 校验 `cloud_home-<ver>-<target>.tar.gz`，只解出 `cloud_home`、跑 `--self-test`，然后布置上述目录并
生成 `.env`（随机 `JWT_SECRET`）。整个安装目录归服务用户所有，因为更新器要在其中写 `versions/` 与 `update/`。
脚本解析清单依赖 `scripts/make_manifest.py` 的 `json.dumps(indent=2)` 输出格式。已有 `current` 时脚本不覆盖。
测试用钩子：`--no-system`、`UPDATE_ROOT`、`CLOUD_HOME_BASE_URL`、`CLOUD_HOME_PUBLIC_KEY`，生产勿设。
数据库备份在数据盘 `backups/`（保留最近 2 份）。

## 配置（.env）

| 变量 | 默认 | 说明 |
|------|------|------|
| `UPDATE_MODE` | `manual` | `manual` 仅提示；`auto` 在时间窗内空闲时自动安装 |
| `UPDATE_WINDOW` | `03:00-05:00` | 自动安装时间窗（本地时间，可跨午夜） |
| `UPDATE_CHANNEL` | `stable` | `beta` 同时接收 `-rc` 预发布 |
| `UPDATE_CHECK_INTERVAL_HOURS` | `6` | 检查间隔（1..=720 小时，另加 0~10% 抖动） |
| `UPDATE_ROOT` | `/opt/cloud_home` | 安装根目录 |
| `UPDATE_BASE_URL` | 空 | 仅测试：用本地镜像代替 GitHub，生产勿设 |

## 安全

- 设备只信任**编进二进制的 minisign 公钥**（`update::PUBLIC_KEY`）。GitHub 账号或 CI 被攻破
  但私钥未泄露，则无法给设备推恶意版本。
- 先验签再解析清单；安装包还要匹配清单里的 sha256 与大小；解包只取一个固定路径的文件，
  不做通用解压（无路径穿越面）。
- 只升级、不降级：清单版本 `<=` 当前版本一律忽略（防回放旧版本）。
- 回滚过的版本记入 `last_failed`，检查器不再重复安装它（防「装→崩→回滚」死循环）。
- **更换签名密钥**：必须先发一个同时信任新旧两把公钥的过渡版本，否则已部署设备再也收不到更新。

## 界面（UI）随二进制更新

产品是一体机：`ui/` 的构建产物编译进 `cloud_home` 二进制（需求 R-026），与 API 同源提供。安装包里只有这一个
二进制，所以**更新二进制就是同时更新后端与 UI**——没有独立的 UI 制品，也就不会出现版本错位，回滚时两者一起回退。

- 发布构建必须先构建 `ui/`，且 UI 内置的版本号等于 `CARGO_PKG_VERSION`，否则编译失败（不可能发出过期或缺失的 UI）。
- `--self-test`（更新器在切换前对新二进制运行）同时校验内嵌 UI：`index.html` 与资源齐全、版本一致；不通过则拒绝安装。
- 已打开的浏览器标签页持有旧 UI：更新重启后，页面通过 `/health` 的版本号发现变化并提示/自动重新加载（R-027），
  上传/下载进行中绝不强制刷新。
- 端到端脚本（R-031）要证明：更新后 `GET /` 返回的是新 UI，回滚后是旧 UI。

## 发布规则

1. **迁移只增不改不删**（加表/加列）。数据库备份是安全网，不是计划。
2. 先打 `vX.Y.Z-rc.N` 标签，用 `UPDATE_CHANNEL=beta` 的自己的设备验证，再打正式标签。
3. 标签必须与 `Cargo.toml` 的 `version` 一致，否则工作流失败。
4. 签名私钥存在 GitHub Secret `MINISIGN_SECRET_KEY`（无口令密钥）并离线备份一份。

## 已知限制

- 回滚会还原更新前的数据库备份：新版本在被提交之前（2 分钟内）写入的数据会丢失。
  这只发生在新版本反复崩溃、无法稳定运行的情形。
- 需要守护进程在进程退出（含退出码 0）后总是重启：systemd `Restart=always` / launchd `KeepAlive`。
- 仅 Unix（符号链接切换）。
