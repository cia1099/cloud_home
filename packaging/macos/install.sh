#!/usr/bin/env bash
# Cloud Home 一体机安装脚本（macOS，Apple 芯片）：  sudo bash install.sh
# 流程与 packaging/linux/install.sh 相同（minisign 验签 → sha256 → 布置 /opt/cloud_home 与 current 链接），
# 服务用 launchd LaunchDaemon（KeepAlive），以执行 sudo 的登录用户运行（不另建系统用户）。
# 需要 minisign：brew install minisign。已安装（current 存在）则不覆盖。
#
# 选项：--no-system  只布置目录与文件，不写 launchd（测试用，可非 root）
# 环境变量（测试用，生产勿设）：UPDATE_ROOT、CLOUD_HOME_BASE_URL、CLOUD_HOME_PUBLIC_KEY（含义同 Linux 版）
set -euo pipefail

# 必须与 src/update/mod.rs 的 PUBLIC_KEY 一致（有单元测试守护）。
PUBLIC_KEY="${CLOUD_HOME_PUBLIC_KEY:-RWR4IcEpkUB52UT0pqy7cHKuT1IKMRljRctf8rcOYT332Z29K23oGbp3}"
BASE_URL="${CLOUD_HOME_BASE_URL:-https://github.com/cia1099/cloud_home/releases/latest/download}"
ROOT="${UPDATE_ROOT:-/opt/cloud_home}"
LABEL="com.cloudhome.server"
PLIST="/Library/LaunchDaemons/$LABEL.plist"
TARGET=aarch64-apple-darwin

NO_SYSTEM=0
for arg in "$@"; do
  case "$arg" in
    --no-system) NO_SYSTEM=1 ;;
    *) echo "未知选项: $arg" >&2; exit 2 ;;
  esac
done

die() { echo "错误: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "缺少命令 $1（$2）"; }

SVC_USER="${SUDO_USER:-$(id -un)}"
if [ "$NO_SYSTEM" = 0 ]; then
  [ "$(id -u)" = 0 ] || die "请用 root 运行（sudo bash install.sh）"
  [ "$(uname -s)" = Darwin ] || die "本脚本只支持 macOS；Linux 见 packaging/linux/"
  [ "$SVC_USER" != root ] || die "请用 sudo 从普通用户运行，服务不以 root 身份运行"
fi
need curl "系统自带"
need minisign "brew install minisign；验签必需，不会跳过"
[ "$(uname -m)" = arm64 ] || die "只发布 Apple 芯片（arm64）安装包，本机是 $(uname -m)"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ -L "$ROOT/current" ]; then
  echo "已安装（current -> $(readlink "$ROOT/current")），升级请用在线更新；不覆盖。"
else
  # 1. 先验签再解析。
  curl -fsSL "$BASE_URL/manifest.json" -o "$TMP/manifest.json"
  curl -fsSL "$BASE_URL/manifest.json.minisig" -o "$TMP/manifest.json.minisig"
  minisign -Vm "$TMP/manifest.json" -x "$TMP/manifest.json.minisig" -P "$PUBLIC_KEY" >/dev/null \
    || die "清单签名校验失败，已中止"

  VERSION="$(sed -n 's/^  "version": "\([^"]*\)",$/\1/p' "$TMP/manifest.json" | head -1)"
  printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$' \
    || die "清单版本无效: $VERSION"
  WANT_SHA="$(awk -v t="\"$TARGET\": {" 'index($0, t) {f=1} f && /"sha256":/ {gsub(/[",]/, "", $2); print $2; exit}' "$TMP/manifest.json")"
  printf '%s' "$WANT_SHA" | grep -Eq '^[0-9a-fA-F]{64}$' || die "清单里没有 $TARGET 的安装包"

  # 2. 下载、校验 sha256，只解出 cloud_home 一个文件并自检。
  PKG="cloud_home-$VERSION-$TARGET"
  curl -fSL "$BASE_URL/$PKG.tar.gz" -o "$TMP/$PKG.tar.gz"
  GOT_SHA="$(shasum -a 256 "$TMP/$PKG.tar.gz" | cut -d' ' -f1)"
  [ "$(printf '%s' "$GOT_SHA" | tr 'A-F' 'a-f')" = "$(printf '%s' "$WANT_SHA" | tr 'A-F' 'a-f')" ] \
    || die "安装包 sha256 不符，已中止"
  mkdir "$TMP/x"
  tar -xzf "$TMP/$PKG.tar.gz" -C "$TMP/x" "$PKG/cloud_home"
  chmod 755 "$TMP/x/$PKG/cloud_home"
  "$TMP/x/$PKG/cloud_home" --self-test | grep -q "^ok $VERSION " || die "新二进制自检失败"

  # 3. 布局（与更新器一致）。
  mkdir -p "$ROOT/versions/$VERSION" "$ROOT/update" "$ROOT/logs"
  mv "$TMP/x/$PKG/cloud_home" "$ROOT/versions/$VERSION/cloud_home"
  ln -sfn "versions/$VERSION" "$ROOT/current"

  # 4. 首次生成 .env（JWT_SECRET 随机，绝不打印）；已有则保留。
  if [ ! -e "$ROOT/.env" ]; then
    (
      umask 077
      {
        echo "# 由 install.sh 生成；完整配置项见仓库 .env.example"
        echo "JWT_SECRET=$(head -c 48 /dev/urandom | base64 | tr -d '\n')"
        echo "# 局域网访问请改为 SERVER_HOST=0.0.0.0（默认只监听本机，建议放在反向代理之后）"
        echo "SERVER_HOST=127.0.0.1"
        echo "LOG_DIR=$ROOT/logs"
      } > "$ROOT/.env"
    )
  fi
fi

if [ "$NO_SYSTEM" = 1 ]; then
  echo "完成（--no-system）：已布置 $ROOT，未写 launchd。"
  exit 0
fi

# 5. 属主与 LaunchDaemon。更新器要在安装目录里写 versions/ 与 update/，所以整个目录归服务用户。
chown -R "$SVC_USER" "$ROOT"
chmod 600 "$ROOT/.env"

cat > "$PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>UserName</key><string>$SVC_USER</string>
  <key>WorkingDirectory</key><string>$ROOT</string>
  <key>ProgramArguments</key>
  <array><string>$ROOT/current/cloud_home</string></array>
  <key>RunAtLoad</key><true/>
  <!-- 在线更新靠「进程退出后总是被拉起」切换版本；KeepAlive=true 含退出码 0。 -->
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>2</integer>
  <key>StandardOutPath</key><string>$ROOT/logs/launchd.out.log</string>
  <key>StandardErrorPath</key><string>$ROOT/logs/launchd.err.log</string>
</dict>
</plist>
PLIST_EOF
chown root:wheel "$PLIST"
chmod 644 "$PLIST"

launchctl bootout "system/$LABEL" >/dev/null 2>&1 || true
launchctl bootstrap system "$PLIST"
echo "完成：cloud_home 已安装并启动（launchctl print system/$LABEL），配置在 $ROOT/.env。"
echo "提示：首次访问外接硬盘时 macOS 可能弹出「可移除卷宗」权限询问，需允许。"
