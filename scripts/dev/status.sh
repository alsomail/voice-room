#!/usr/bin/env bash
# status.sh — Voice Room 本地全栈状态总览（兼容 macOS bash 3.2）
#
# 用法：
#   bash scripts/dev/status.sh           # 普通输出
#   bash scripts/dev/status.sh --json    # JSON 输出
#
# 退出码（位掩码）：
#   0  全绿
#   1  docker
#   2  vr-postgres
#   4  vr-redis
#   8  AppServer
#   16 AdminServer
#   32 Web

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PIDS_FILE="$REPO_ROOT/.e2e-up.pids"

JSON_MODE=0
[[ "${1:-}" == "--json" ]] && JSON_MODE=1

if [[ -t 1 && "${NO_COLOR:-0}" != "1" && "${CI:-}" != "1" && $JSON_MODE -eq 0 ]]; then
  C_OK=$'\033[32m'; C_FAIL=$'\033[31m'; C_WARN=$'\033[33m'; C_DIM=$'\033[2m'; C_END=$'\033[0m'
else
  C_OK=""; C_FAIL=""; C_WARN=""; C_DIM=""; C_END=""
fi

# 服务列表与对应变量名（bash 3.2 无关联数组，用一一对应的位置变量）
SERVICES="docker pg_container redis_container pg_tcp pg_db redis_ping app_server admin_server web pids"

# 用 _STATUS_$name / _DETAIL_$name 形式存储
set_state() { # name, status, detail
  local name="$1" st="$2" de="$3"
  eval "_STATUS_$name=\"\$st\""
  eval "_DETAIL_$name=\"\$de\""
}
get_status()  { eval "echo \"\${_STATUS_$1:-unknown}\""; }
get_detail()  { eval "echo \"\${_DETAIL_$1:-}\""; }

ok()    { printf '  %s✅%s %-22s %s\n'  "$C_OK"   "$C_END" "$1" "$2"; }
fail()  { printf '  %s❌%s %-22s %s\n'  "$C_FAIL" "$C_END" "$1" "$2"; }
warn()  { printf '  %s⚠️ %s %-22s %s\n' "$C_WARN" "$C_END" "$1" "$2"; }

# ---------- 1. Docker ----------
if docker info >/dev/null 2>&1; then
  set_state docker ok "running"
else
  set_state docker down "daemon not running"
fi

# ---------- 2/3. 容器 ----------
container_state() { # name -> "running/healthy" | "missing" | ...
  local name="$1"
  if ! docker info >/dev/null 2>&1; then
    echo "docker-down"; return
  fi
  local s
  s=$(docker inspect -f '{{.State.Status}}/{{if .State.Health}}{{.State.Health.Status}}{{else}}n-a{{end}}' "$name" 2>/dev/null || true)
  if [[ -z "$s" ]]; then echo "missing"; else echo "$s"; fi
}

pg_state=$(container_state vr-postgres)
case "$pg_state" in
  running/healthy)   set_state pg_container ok       "vr-postgres ($pg_state)" ;;
  running/starting)  set_state pg_container degraded "vr-postgres ($pg_state)" ;;
  running/n-a)       set_state pg_container ok       "vr-postgres ($pg_state)" ;;
  missing)           set_state pg_container down     "vr-postgres 不存在（未启动 docker compose？）" ;;
  *)                 set_state pg_container down     "vr-postgres ($pg_state)" ;;
esac

redis_state=$(container_state vr-redis)
case "$redis_state" in
  running/healthy)   set_state redis_container ok       "vr-redis ($redis_state)" ;;
  running/starting)  set_state redis_container degraded "vr-redis ($redis_state)" ;;
  running/n-a)       set_state redis_container ok       "vr-redis ($redis_state)" ;;
  missing)           set_state redis_container down     "vr-redis 不存在" ;;
  *)                 set_state redis_container down     "vr-redis ($redis_state)" ;;
esac

# ---------- 4. PG TCP ----------
if command -v pg_isready >/dev/null 2>&1 && pg_isready -h 127.0.0.1 -p 5432 -U postgres -t 2 >/dev/null 2>&1; then
  set_state pg_tcp ok "127.0.0.1:5432 ready"
elif command -v nc >/dev/null 2>&1 && nc -z -w 2 127.0.0.1 5432 >/dev/null 2>&1; then
  set_state pg_tcp degraded "127.0.0.1:5432 端口在但 pg_isready 失败"
else
  set_state pg_tcp down "127.0.0.1:5432 不通"
fi

# ---------- 5. PG 业务库 ----------
if [[ "$(get_status pg_tcp)" == "ok" ]]; then
  if docker exec vr-postgres psql -U postgres -d voiceroom -c 'SELECT 1' >/dev/null 2>&1; then
    set_state pg_db ok "voiceroom 库可连接"
  else
    set_state pg_db down "voiceroom 库连接失败"
  fi
else
  set_state pg_db down "skip（PG 未就绪）"
fi

# ---------- 6. Redis PONG ----------
redis_pong=""
if command -v redis-cli >/dev/null 2>&1; then
  redis_pong=$(redis-cli -h 127.0.0.1 -p 6379 -t 2 ping 2>/dev/null || true)
elif docker info >/dev/null 2>&1; then
  redis_pong=$(docker exec vr-redis redis-cli ping 2>/dev/null || true)
fi
if [[ "$redis_pong" == "PONG" ]]; then
  set_state redis_ping ok "PONG"
else
  set_state redis_ping down "无 PONG"
fi

# ---------- 7/8/9. HTTP ----------
http_check() { # name, url, regex
  local name="$1" url="$2" rx="$3" code
  if ! command -v curl >/dev/null 2>&1; then
    set_state "$name" down "curl 缺失"
    return
  fi
  code=$(curl -sS -o /dev/null --max-time 2 -w '%{http_code}' "$url" 2>/dev/null || true)
  [[ -z "$code" ]] && code="000"
  if [[ "$code" =~ $rx ]]; then
    set_state "$name" ok "HTTP $code  $url"
  else
    set_state "$name" down "HTTP $code  $url"
  fi
}
http_check app_server   "http://127.0.0.1:3000/health" '^200$'
http_check admin_server "http://127.0.0.1:3001/health" '^200$'
http_check web          "http://127.0.0.1:5173/"       '^(200|301|302)$'

# ---------- 10. PID 文件 ----------
if [[ -f "$PIDS_FILE" ]]; then
  alive=0; total=0
  while read -r pid; do
    [[ -z "$pid" ]] && continue
    total=$((total+1))
    if kill -0 "$pid" 2>/dev/null; then alive=$((alive+1)); fi
  done < "$PIDS_FILE"
  if [[ $total -eq 0 ]]; then
    set_state pids degraded ".e2e-up.pids 为空"
  elif [[ $alive -eq $total ]]; then
    set_state pids ok "$alive/$total 进程存活"
  else
    set_state pids degraded "$alive/$total 进程存活（部分已退出）"
  fi
else
  set_state pids degraded "无 .e2e-up.pids（未通过 e2e:up/start 启动？）"
fi

# ---------- 输出 ----------
print_line() { # name
  local n="$1" st de
  st=$(get_status "$n"); de=$(get_detail "$n")
  case "$st" in
    ok)       ok   "$n" "$de" ;;
    degraded) warn "$n" "$de" ;;
    *)        fail "$n" "$de" ;;
  esac
}

if [[ $JSON_MODE -eq 1 ]]; then
  echo -n '{'
  first=1
  for k in $SERVICES; do
    [[ $first -eq 0 ]] && echo -n ','
    first=0
    st=$(get_status "$k"); de=$(get_detail "$k")
    de_esc=$(printf '%s' "$de" | sed 's/\\/\\\\/g; s/"/\\"/g')
    printf '"%s":{"status":"%s","detail":"%s"}' "$k" "$st" "$de_esc"
  done
  echo '}'
else
  echo
  echo "============== Voice Room — 本地状态 =============="
  echo
  printf '  %s基础设施%s\n' "$C_DIM" "$C_END"
  for k in docker pg_container redis_container pg_tcp pg_db redis_ping; do print_line "$k"; done
  echo
  printf '  %s应用服务%s\n' "$C_DIM" "$C_END"
  for k in app_server admin_server web; do print_line "$k"; done
  echo
  printf '  %s进程跟踪%s\n' "$C_DIM" "$C_END"
  print_line pids
  echo
fi

# ---------- 退出码 ----------
rc=0
[[ "$(get_status docker)"          != "ok" ]] && rc=$((rc | 1))
[[ "$(get_status pg_container)"    != "ok" ]] && rc=$((rc | 2))
[[ "$(get_status redis_container)" != "ok" ]] && rc=$((rc | 4))
[[ "$(get_status app_server)"      != "ok" ]] && rc=$((rc | 8))
[[ "$(get_status admin_server)"    != "ok" ]] && rc=$((rc | 16))
[[ "$(get_status web)"             != "ok" ]] && rc=$((rc | 32))
exit $rc
