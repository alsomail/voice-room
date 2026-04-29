#!/usr/bin/env bash
# start-local.sh — 本地全栈智能启动（已运行则跳过）
#
# 与 e2e-up.sh 的差异：
#   - e2e-up.sh：端口被占用即 fail（适合干净 CI）
#   - start-local.sh：每端先 health-check，已健康则跳过，仅启动缺失部分（适合日常开发）
#
# 用法：
#   bash scripts/dev/start-local.sh                # 智能启动 5 端
#   bash scripts/dev/start-local.sh --skip web     # 跳过 web
#   bash scripts/dev/start-local.sh --force        # 强制重启（先 stop 再 up）
#   bash scripts/dev/start-local.sh --no-web       # 仅起后端三件套（PG/Redis/AppServer/AdminServer）
#
# 退出码：0 = 全绿；非 0 = wait-on 超时或启动失败。

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

PIDS_FILE="$REPO_ROOT/.e2e-up.pids"
LOG_DIR="$REPO_ROOT/.e2e-logs"
mkdir -p "$LOG_DIR"
touch "$PIDS_FILE"

FORCE=0
SKIP_WEB=0
SKIP_APP=0
SKIP_ADMIN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)   FORCE=1; shift ;;
    --no-web)  SKIP_WEB=1; shift ;;
    --skip)    case "${2:-}" in
                 web) SKIP_WEB=1 ;;
                 app|app-server) SKIP_APP=1 ;;
                 admin|admin-server) SKIP_ADMIN=1 ;;
                 *) echo "unknown skip target: $2" >&2; exit 2 ;;
               esac; shift 2 ;;
    -h|--help) sed -n '2,18p' "$0"; exit 0 ;;
    *)         echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [[ $FORCE -eq 1 ]]; then
  echo "[start-local] --force：先停服"
  bash "$REPO_ROOT/scripts/dev/e2e-down.sh" || true
fi

# 与 e2e-up.sh 对齐的环境变量默认值
export DATABASE_URL="${DATABASE_URL:-postgres://app_server_user:app_server_pass@localhost:5432/voiceroom}"
export ADMIN_DATABASE_URL="${ADMIN_DATABASE_URL:-postgres://admin_server_user:admin_server_pass@localhost:5432/voiceroom}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"
export JWT_SECRET="${JWT_SECRET:-e2e-up-local-dev-secret-do-not-use-in-prod-please}"
export APP_JWT_SECRET="${APP_JWT_SECRET:-$JWT_SECRET}"
export ADMIN_JWT_SECRET="${ADMIN_JWT_SECRET:-$JWT_SECRET}"
export AGORA_APP_CERT="${AGORA_APP_CERT:-e2e-stub-cert}"

# ---------- helpers ----------
http_ok() { # url, expected_regex
  local code
  code=$(curl -sS -o /dev/null --max-time 2 -w '%{http_code}' "$1" 2>/dev/null || true)
  [[ "$code" =~ $2 ]]
}

container_running() { # name
  [[ "$(docker inspect -f '{{.State.Status}}' "$1" 2>/dev/null || true)" == "running" ]]
}

container_healthy() { # name (无 healthcheck 的也算 ok)
  local s
  s=$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}healthy{{end}}' "$1" 2>/dev/null || true)
  [[ "$s" == "healthy" ]]
}

bg_record() { # cmd...
  "$@" &
  echo "$!" >> "$PIDS_FILE"
}

# ---------- 1. Docker (PG + Redis) ----------
echo "[start-local] 1/4 检查 docker postgres + redis"
need_compose_up=0
if container_running vr-postgres && container_running vr-redis; then
  echo "  ↪ 容器已运行，跳过 docker compose up"
else
  need_compose_up=1
fi

if [[ $need_compose_up -eq 1 ]]; then
  if ! docker info >/dev/null 2>&1; then
    echo "[start-local] ❌ docker daemon 未启动" >&2
    exit 10
  fi
  docker compose up -d postgres redis
fi

# 等 PG 就绪
echo "[start-local] 1.5/4 等待 PG 健康（最长 30s）"
for i in $(seq 1 30); do
  if docker exec vr-postgres pg_isready -U postgres >/dev/null 2>&1; then
    echo "  ↪ PG 就绪"
    break
  fi
  sleep 1
  if [[ $i -eq 30 ]]; then
    echo "[start-local] ❌ PG 在 30s 内未就绪" >&2
    exit 11
  fi
done

# ---------- 2. AppServer ----------
if [[ $SKIP_APP -eq 1 ]]; then
  echo "[start-local] 2/4 跳过 AppServer（--skip app）"
elif http_ok "http://127.0.0.1:3000/health" '^200$'; then
  echo "[start-local] 2/4 AppServer 已健康（http://127.0.0.1:3000） — 跳过"
else
  echo "[start-local] 2/4 启动 AppServer → :3000"
  ( APP_PROFILE="${APP_PROFILE:-dev}" cargo run -p voice-room-server >"$LOG_DIR/app-server.log" 2>&1 ) &
  echo "$!" >> "$PIDS_FILE"
fi

# ---------- 3. AdminServer ----------
if [[ $SKIP_ADMIN -eq 1 ]]; then
  echo "[start-local] 3/4 跳过 AdminServer"
elif http_ok "http://127.0.0.1:3001/health" '^200$'; then
  echo "[start-local] 3/4 AdminServer 已健康（http://127.0.0.1:3001） — 跳过"
else
  echo "[start-local] 3/4 启动 AdminServer → :3001"
  ( ADMIN_PROFILE="${ADMIN_PROFILE:-dev}" DATABASE_URL="$ADMIN_DATABASE_URL" \
      cargo run -p voice-room-admin-server >"$LOG_DIR/admin-server.log" 2>&1 ) &
  echo "$!" >> "$PIDS_FILE"
fi

# ---------- 4. Web ----------
if [[ $SKIP_WEB -eq 1 ]]; then
  echo "[start-local] 4/4 跳过 Web（--skip web 或 --no-web）"
elif http_ok "http://127.0.0.1:5173/" '^(200|301|302)$'; then
  echo "[start-local] 4/4 Web 已健康（http://127.0.0.1:5173） — 跳过"
else
  echo "[start-local] 4/4 启动 Web (vite) → :5173"
  ( cd "$REPO_ROOT/app/web" && npm run dev >"$LOG_DIR/web.log" 2>&1 ) &
  echo "$!" >> "$PIDS_FILE"
fi

# ---------- 等待健康 ----------
echo "[start-local] 等待全部端点就绪..."

wait_http() { # name, url, regex, timeout_sec
  local name="$1" url="$2" rx="$3" timeout="$4"
  local code start now
  start=$(date +%s)
  while :; do
    code=$(curl -sS -o /dev/null --max-time 2 -w '%{http_code}' "$url" 2>/dev/null || true)
    [[ "$code" =~ $rx ]] && { echo "  ↪ $name ✅ ($url HTTP $code)"; return 0; }
    now=$(date +%s)
    if (( now - start >= timeout )); then
      echo "  ↪ $name ❌ 超时 ${timeout}s (最后 HTTP=$code, url=$url)" >&2
      return 1
    fi
    sleep 2
  done
}

wait_tcp() { # name, host, port, timeout_sec
  local name="$1" host="$2" port="$3" timeout="$4"
  local start now
  start=$(date +%s)
  while :; do
    if (echo > "/dev/tcp/$host/$port") 2>/dev/null; then
      echo "  ↪ $name ✅ ($host:$port)"
      return 0
    fi
    now=$(date +%s)
    if (( now - start >= timeout )); then
      echo "  ↪ $name ❌ 超时 ${timeout}s（$host:$port 不通）" >&2
      return 1
    fi
    sleep 1
  done
}

rc=0
wait_tcp  "PG"           127.0.0.1 5432 30  || rc=$?
wait_tcp  "Redis"        127.0.0.1 6379 15  || rc=$?
[[ $SKIP_APP   -eq 0 ]] && { wait_http "AppServer"   "http://127.0.0.1:3000/health" '^200$'        180 || rc=$?; }
[[ $SKIP_ADMIN -eq 0 ]] && { wait_http "AdminServer" "http://127.0.0.1:3001/health" '^200$'        180 || rc=$?; }
[[ $SKIP_WEB   -eq 0 ]] && { wait_http "Web"         "http://127.0.0.1:5173/"       '^(200|301|302)$' 240 || rc=$?; }

if [[ $rc -ne 0 ]]; then
  echo "[start-local] ❌ 部分端点未就绪；日志：$LOG_DIR/" >&2
  echo "[start-local] 提示：再跑一次 'npm run start' 通常能闭环（cargo/vite 冷启已完成）" >&2
  exit 1
fi

echo
echo "[start-local] ✅ 本地全栈就绪"
echo "  ↪ 状态：bash scripts/dev/status.sh"
echo "  ↪ 停服：bash scripts/dev/e2e-down.sh"
echo "  ↪ 日志：$LOG_DIR/"
echo "  ↪ PID ：$PIDS_FILE"
