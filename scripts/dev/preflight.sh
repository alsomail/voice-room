#!/usr/bin/env bash
# T-0000G: Preflight 5 端健康检查
#
# 实现 doc/tds/infra/T-0000G.md §2.6 矩阵 + §2.7 globalSetup 接口契约。
#
# 用法：
#   bash scripts/dev/preflight.sh [--profile local|staging|prod]
# Profile 默认读 env E2E_PROFILE。
#
# 退出码：0 全绿；11 PG；12 Redis；13 AppServer；14 AdminServer；15 Web。
# 任一失败 fail-fast，后续项标记 ⏭ skipped。
# 总壁时上限 ≤ 10s（5 项 × 每项 2s 超时）。

set -uo pipefail

PROFILE="${E2E_PROFILE:-local}"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --profile) PROFILE="${2:-}"; shift 2 ;;
        -h|--help) sed -n '2,15p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *) shift ;;
    esac
done

# -------- 颜色（无 TTY 或 NO_COLOR=1 关闭） --------
if [[ -t 1 && "${NO_COLOR:-0}" != "1" && "${CI:-}" != "1" ]]; then
    C_OK=$'\033[32m'; C_FAIL=$'\033[31m'; C_SKIP=$'\033[33m'; C_HINT=$'\033[36m'; C_END=$'\033[0m'
else
    C_OK=""; C_FAIL=""; C_SKIP=""; C_HINT=""; C_END=""
fi

ICON_OK="${C_OK}✅${C_END}"
ICON_FAIL="${C_FAIL}❌${C_END}"
ICON_SKIP="${C_SKIP}⏭${C_END}"

START_TS=$(date +%s)

elapsed_total() {
    local now; now=$(date +%s)
    echo "$(( now - START_TS ))"
}

# 上行：失败行（含 hint）
fail_line() {
    local label="$1" target="$2" hint="$3"
    printf '%s %s  %s\n' "${label}" "${ICON_FAIL}" "${target}"
    printf '        ↳ %sHint:%s %s\n' "${C_HINT}" "${C_END}" "${hint}"
}

ok_line() {
    local label="$1" detail="$2" ms="$3"
    if [[ -n "${ms}" ]]; then
        printf '%s %s  %s  (%sms)\n' "${label}" "${ICON_OK}" "${detail}" "${ms}"
    else
        printf '%s %s  %s\n' "${label}" "${ICON_OK}" "${detail}"
    fi
}

skip_line() {
    local label="$1" reason="$2"
    printf '%s %s  skipped (%s)\n' "${label}" "${ICON_SKIP}" "${reason}"
}

# 计时（毫秒；无 nanosecond date 时退化为 0）
now_ms() {
    if date +%s%N 2>/dev/null | grep -q '^[0-9]*$'; then
        echo $(( $(date +%s%N) / 1000000 ))
    else
        # macOS 默认 date 不支持 %N → 用 perl 或退化
        if command -v perl >/dev/null 2>&1; then
            perl -MTime::HiRes=time -e 'printf "%d\n", time*1000'
        else
            echo $(( $(date +%s) * 1000 ))
        fi
    fi
}

# -------- 检查实现 --------
check_postgres() {
    local label="[1/5] Postgres"
    if [[ "${PROFILE}" != "local" ]]; then
        skip_line "${label}    " "remote profile"
        return 0
    fi
    local host="${PGHOST:-127.0.0.1}" port="${PGPORT:-5432}" user="${PGUSER:-postgres}" db="${PGDATABASE:-voiceroom}"
    if command -v pg_isready >/dev/null 2>&1; then
        if pg_isready -h "${host}" -p "${port}" -U "${user}" -d "${db}" -t 2 >/dev/null 2>&1; then
            ok_line "${label}   " "ready (${host}:${port})" ""
            return 0
        fi
    else
        # fallback: nc 端口探测
        if command -v nc >/dev/null 2>&1 && nc -z -w 2 "${host}" "${port}" >/dev/null 2>&1; then
            ok_line "${label}   " "port reachable (${host}:${port}) [pg_isready missing]" ""
            return 0
        fi
    fi
    fail_line "${label}   " "(${host}:${port})" "docker compose up -d postgres"
    return 11
}

check_redis() {
    local label="[2/5] Redis"
    if [[ "${PROFILE}" != "local" ]]; then
        skip_line "${label}       " "remote profile"
        return 0
    fi
    local url="${REDIS_URL:-redis://127.0.0.1:6379}"
    if command -v redis-cli >/dev/null 2>&1; then
        local out
        out=$(redis-cli -u "${url}" -t 2 ping 2>/dev/null || true)
        if [[ "${out}" == "PONG" ]]; then
            ok_line "${label}      " "ready (${url})" ""
            return 0
        fi
    else
        # fallback: 解析 host:port 后用 nc
        local hp="${url#redis://}"; hp="${hp%%/*}"
        local host="${hp%%:*}" port="${hp##*:}"
        [[ "${host}" == "${port}" ]] && port=6379
        if command -v nc >/dev/null 2>&1 && nc -z -w 2 "${host}" "${port}" >/dev/null 2>&1; then
            ok_line "${label}      " "port reachable (${url}) [redis-cli missing]" ""
            return 0
        fi
    fi
    fail_line "${label}      " "(${url})" "docker compose up -d redis"
    return 12
}

check_http() {
    # $1 label  $2 url  $3 hint  $4 expected_code (regex)  $5 fail_code
    local label="$1" url="$2" hint="$3" code_regex="$4" fail_code="$5"
    if ! command -v curl >/dev/null 2>&1; then
        fail_line "${label}" "${url}" "install curl"
        return "${fail_code}"
    fi
    local t0 t1 ms code
    t0=$(now_ms)
    code=$(curl -sS -o /dev/null --max-time 2 -w '%{http_code}' "${url}" 2>/dev/null || true)
    [[ -z "${code}" ]] && code="000"
    t1=$(now_ms)
    ms=$(( t1 - t0 ))
    if [[ "${code}" =~ ${code_regex} ]]; then
        ok_line "${label}" "${code}  ${url}" "${ms}"
        return 0
    fi
    fail_line "${label}" "${url} (got ${code})" "${hint}"
    return "${fail_code}"
}

check_app() {
    local url="${APP_SERVER_BASE_URL:-http://localhost:3000}/health"
    check_http "[3/5] AppServer  " "${url}" "cargo run -p voice-room-server -- --profile local" '^200$' 13
}

check_admin() {
    local url="${ADMIN_SERVER_BASE_URL:-http://localhost:3001}/health"
    check_http "[4/5] AdminServer" "${url}" "cargo run -p voice-room-admin-server -- --profile local" '^200$' 14
}

check_web() {
    local url="${ADMIN_WEB_URL:-http://localhost:5173}"
    check_http "[5/5] Web        " "${url}" "cd app/web && npm run dev" '^(200|301|302)$' 15
}

# -------- 执行序列 + fail-fast --------
RC=0
for fn in check_postgres check_redis check_app check_admin check_web; do
    if [[ "${RC}" -eq 0 ]]; then
        "${fn}" || RC=$?
    else
        case "${fn}" in
            check_postgres)  skip_line "[1/5] Postgres   " "fail-fast" ;;
            check_redis)     skip_line "[2/5] Redis      " "fail-fast" ;;
            check_app)       skip_line "[3/5] AppServer  " "fail-fast" ;;
            check_admin)     skip_line "[4/5] AdminServer" "fail-fast" ;;
            check_web)       skip_line "[5/5] Web        " "fail-fast" ;;
        esac
    fi
done

if [[ "${RC}" -eq 0 ]]; then
    printf 'preflight: all 5 checks passed in %ss (profile=%s)\n' "$(elapsed_total)" "${PROFILE}"
else
    printf 'preflight: failed in %ss (rc=%s, profile=%s)\n' "$(elapsed_total)" "${RC}" "${PROFILE}" >&2
fi
exit "${RC}"
