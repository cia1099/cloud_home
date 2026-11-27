#!/usr/bin/env bash
# Cloud Home 一体机安装脚本（Linux，需 root）：  sudo bash install.sh
# 创建 cloudhome 用户 → 取最新发布的清单并用 minisign 验签 → 下载匹配本机架构的安装包并校验 sha256
# → 布置 /opt/cloud_home（versions/<ver>/、current 符号链接）→ 安装 systemd 服务（Restart=always）。
# 布局与在线更新器完全一致（spec/update.md），装好后更新器自动启用。已安装（current 存在）则不覆盖。
#
# 选项：--no-system  只布置目录与文件，不建用户、不写 systemd（测试用，可非 root）
# 环境变量（测试用，生产勿设）：
#   UPDATE_ROOT            安装根目录，默认 /opt/cloud_home（与程序的同名配置一致）
#   CLOUD_HOME_BASE_URL    用本地镜像代替 GitHub 最新发布
#   CLOUD_HOME_PUBLIC_KEY  覆盖内置公钥
set -euo pipefail

# 必须与 src/update/mod.rs 的 PUBLIC_KEY 一致（有单元测试守护）。
PUBLIC_KEY="${CLOUD_HOME_PUBLIC_KEY:-RWR4IcEpkUB52UT0pqy7cHKuT1IKMRljRctf8rcOYT332Z29K23oGbp3}"
BASE_URL="${CLOUD_HOME_BASE_URL:-https://github.com/cia1099/cloud_home/releases/latest/download}"
ROOT="${UPDATE_ROOT:-/opt/cloud_home}"
SVC_USER="cloudhome"

NO_SYSTEM=0
for arg in "$@"; do
  case "$arg" in
    --no-system) NO_SYSTEM=1 ;;
    *) echo "未知选项: $arg" >&2; exit 2 ;;
  esac
done

die() { echo "错误: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "缺少命令 $1（$2）"; }

if [ "$NO_SYSTEM" = 0 ]; then
  [ "$(id -u)" = 0 ] || die "请用 root 运行（sudo bash install.sh）"
  [ "$(uname -s)" = Linux ] || die "本脚本只支持 Linux；macOS 见 packaging/macos/"
  need systemctl "需要 systemd"
fi
need curl "apt install curl"
need minisign "apt install minisign；验签必需，不会跳过"
need tar "apt install tar"

case "$(uname -m)" in
  x86_64 | amd64) TARGET=x86_64-unknown-linux-musl ;;
  aarch64 | arm64) TARGET=aarch64-unknown-linux-musl ;;
  *) die "不支持的架构: $(uname -m)" ;;
esac

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ -L "$ROOT/current" ]; then
  echo "已安装（current -> $(readlink "$ROOT/current")），升级请用在线更新；不覆盖。"
else
  # 1. 先验签再解析：清单与签名来自同一个发布。
  curl -fsSL "$BASE_URL/manifest.json" -o "$TMP/manifest.json"
  curl -fsSL "$BASE_URL/manifest.json.minisig" -o "$TMP/manifest.json.minisig"
  minisign -Vm "$TMP/manifest.json" -x "$TMP/manifest.json.minisig" -P "$PUBLIC_KEY" >/dev/null \
    || die "清单签名校验失败，已中止"

  # 2. 解析（格式由 scripts/make_manifest.py 的 json.dumps(indent=2) 固定；签名已保证内容可信）。
  VERSION="$(sed -n 's/^  "version": "\([^"]*\)",$/\1/p' "$TMP/manifest.json" | head -1)"
  printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$' \
    || die "清单版本无效: $VERSION"
  WANT_SHA="$(awk -v t="\"$TARGET\": {" 'index($0, t) {f=1} f && /"sha256":/ {gsub(/[",]/, "", $2); print $2; exit}' "$TMP/manifest.json")"
  printf '%s' "$WANT_SHA" | grep -Eq '^[0-9a-fA-F]{64}$' || die "清单里没有 $TARGET 的安装包"

  # 3. 下载、校验 sha256，只解出 cloud_home 一个文件并自检。
  PKG="cloud_home-$VERSION-$TARGET"
  curl -fSL "$BASE_URL/$PKG.tar.gz" -o "$TMP/$PKG.tar.gz"
  [ "$(sha256_of "$TMP/$PKG.tar.gz" | tr 'A-F' 'a-f')" = "$(printf '%s' "$WANT_SHA" | tr 'A-F' 'a-f')" ] \
    || die "安装包 sha256 不符，已中止"
  mkdir "$TMP/x"
  tar -xzf "$TMP/$PKG.tar.gz" -C "$TMP/x" "$PKG/cloud_home"
  chmod 755 "$TMP/x/$PKG/cloud_home"
  "$TMP/x/$PKG/cloud_home" --self-test | grep -q "^ok $VERSION " || die "新二进制自检失败"

  # 4. 布局：versions/<ver>/cloud_home + current 链接（更新器仅在 current 为符号链接时启用）。
  mkdir -p "$ROOT/versions/$VERSION" "$ROOT/update" "$ROOT/logs"
  mv "$TMP/x/$PKG/cloud_home" "$ROOT/versions/$VERSION/cloud_home"
  ln -sfn "versions/$VERSION" "$ROOT/current"

  # 5. 配置：首次生成 .env（JWT_SECRET 随机，绝不打印）；已有则保留。
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
  echo "完成（--no-system）：已布置 $ROOT，未建用户、未写服务。"
  exit 0
fi

# 6. 系统用户、属主、服务。更新器要在安装目录里写 versions/ 与 update/，所以整个目录归 cloudhome。
id "$SVC_USER" >/dev/null 2>&1 \
  || useradd --system --home-dir "$ROOT" --shell /usr/sbin/nologin "$SVC_USER"
chown -R "$SVC_USER:$SVC_USER" "$ROOT"
chmod 600 "$ROOT/.env"

cat > /etc/systemd/system/cloud_home.service <<UNIT_EOF
[Unit]
Description=Cloud Home
After=network-online.target local-fs.target
Wants=network-online.target

[Service]
User=$SVC_USER
WorkingDirectory=$ROOT
ExecStart=$ROOT/current/cloud_home
# 在线更新靠「进程退出后总是被拉起」切换版本，必须 always（含退出码 0）。
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
UNIT_EOF

systemctl daemon-reload
systemctl enable --now cloud_home.service
echo "完成：cloud_home 已安装并启动（systemctl status cloud_home），配置在 $ROOT/.env。"
echo "提示：服务以 $SVC_USER 运行，外接硬盘的挂载点须对该用户可写（如 exFAT 用 uid= 挂载选项）。"
