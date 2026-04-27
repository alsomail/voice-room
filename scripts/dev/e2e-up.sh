#!/usr/bin/env bash
# 缺陷 6 修复（batch-e2e-foundation-01 第 1 轮）：
#   一键起全栈聚合 — docker postgres+redis → cargo server → cargo admin-server → vite web
#   → npx wait-on 5 端健康。子进程后台运行，PID 写 .e2e-up.pids 便于停服。
#
# 用法：
#   $ npm run e2e:up        # 起服务并等待健康
#   $ bash scripts/dev/e2e-down.sh   # （可选）按 PID 停服
#
# 退出码：0 OK / 11~15 同 preflight / 78 envLoader CONFIG / 其他 = wait-on 超时
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

PIDS_FILE="$REPO_ROOT/.e2e-up.pids"
LOG_DIR="$REPO_ROOT/.e2e-logs"
mkdir -p "$LOG_DIR"
: > "$PIDS_FILE"

echo "[e2e:up] 1/4 拉起 docker postgres + redis"
docker compose up -d postgres redis

echo "[e2e:up] 2/4 后台启动 AppServer (cargo run -p server) → :3000"
APP_PROFILE="${APP_PROFILE:-dev}" cargo run -p server >"$LOG_DIR/app-server.log" 2>&1 &
echo "$!" >> "$PIDS_FILE"

echo "[e2e:up] 3/4 后台启动 AdminServer (cargo run -p admin-server) → :3001"
ADMIN_PROFILE="${ADMIN_PROFILE:-dev}" cargo run -p admin-server >"$LOG_DIR/admin-server.log" 2>&1 &
echo "$!" >> "$PIDS_FILE"

echo "[e2e:up] 4/4 后台启动 Web (vite) → :5173"
( cd "$REPO_ROOT/app/web" && npm run dev >"$LOG_DIR/web.log" 2>&1 ) &
echo "$!" >> "$PIDS_FILE"

echo "[e2e:up] 等待 5 端健康（最长 180s）..."
npx -y wait-on@^7 -t 180000 \
  "http-get://127.0.0.1:3000/health" \
  "http-get://127.0.0.1:3001/health" \
  "http-get://127.0.0.1:5173/" \
  "tcp:127.0.0.1:5432" \
  "tcp:127.0.0.1:6379" \
  || { echo "[e2e:up] wait-on 超时；查看日志：$LOG_DIR/" >&2; exit 1; }

echo "[e2e:up] OK — 5 端就绪。运行 'bash scripts/dev/e2e-down.sh' 停服。"
echo "[e2e:up] 服务 PID 已记录到 $PIDS_FILE"
