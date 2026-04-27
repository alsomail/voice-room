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

# C 方案：内联 export 服务端启动必需 ENV（避免依赖缺失的 app/server/.env）
# 与 docker-compose（POSTGRES_DB=voiceroom）/ init-db.sh（app_server_user）严格对齐
export DATABASE_URL="${DATABASE_URL:-postgres://app_server_user:app_server_pass@localhost:5432/voiceroom}"
export ADMIN_DATABASE_URL="${ADMIN_DATABASE_URL:-postgres://admin_server_user:admin_server_pass@localhost:5432/voiceroom}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"
export JWT_SECRET="${JWT_SECRET:-e2e-up-local-dev-secret-do-not-use-in-prod-please}"
export APP_JWT_SECRET="${APP_JWT_SECRET:-$JWT_SECRET}"
export ADMIN_JWT_SECRET="${ADMIN_JWT_SECRET:-$JWT_SECRET}"
export AGORA_APP_CERT="${AGORA_APP_CERT:-e2e-stub-cert}"

echo "[e2e:up] 1/4 拉起 docker postgres + redis"
docker compose up -d postgres redis

# C 方案补强：等待 PG 健康后，幂等地补齐 app_server_user 的 schema CREATE 权限
# （local dev 必须；放行 sqlx migrate-on-startup 的 _sqlx_migrations 表 IF NOT EXISTS）
echo "[e2e:up] 1.5/4 等待 PG 健康并补齐 schema 权限"
for i in 1 2 3 4 5 6 7 8 9 10; do
  if docker exec vr-postgres pg_isready -U postgres >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec vr-postgres psql -U postgres -d voiceroom -v ON_ERROR_STOP=1 -c \
  "GRANT CREATE ON SCHEMA public TO app_server_user;" >/dev/null 2>&1 || true

echo "[e2e:up] 2/4 后台启动 AppServer (cargo run -p voice-room-server) → :3000"
APP_PROFILE="${APP_PROFILE:-dev}" cargo run -p voice-room-server >"$LOG_DIR/app-server.log" 2>&1 &
echo "$!" >> "$PIDS_FILE"

echo "[e2e:up] 3/4 后台启动 AdminServer (cargo run -p voice-room-admin-server) → :3001"
# AdminServer 必须以 admin_server_user 连库（拥有 schema public 全权 + 业务表全权）
ADMIN_PROFILE="${ADMIN_PROFILE:-dev}" DATABASE_URL="$ADMIN_DATABASE_URL" cargo run -p voice-room-admin-server >"$LOG_DIR/admin-server.log" 2>&1 &
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
