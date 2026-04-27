#!/usr/bin/env bash
# T-0000G: E2E Reset 脚本（幂等）
#
# 仅在 E2E_PROFILE=local 时允许执行（profile≠local → 退码 21）。
# 清理范围：见 doc/tds/infra/T-0000G.md §2.5
#   - users.phone LIKE '+96650000090%' 或 id ∈ {E2E A/B}
#   - admins.username ∈ {e2e_admin,e2e_op,e2e_cs,e2e_fin}
#   - rooms.id = E2E_ROOM_ID 及其关联子表
#   - Redis 业务键前缀 room:{ROOM_ID}:*  user:{USER_*_ID}:*  kicked:{ROOM_ID}:*
#   - 删除 scripts/dev/.seed-output.env
# 不动业务表 schema、不动 schema_migrations。
#
# 退出码（与 TDS §2.7 globalSetup 契约一致）：
#   0 成功 / 21 profile 拒绝 / 24 无连接

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_FILE="${REPO_ROOT}/scripts/dev/.seed-output.env"

# CLI args
ASSUME_YES=0
for arg in "$@"; do
    case "$arg" in
        --yes) ASSUME_YES=1 ;;
        -h|--help) sed -n '2,17p' "${BASH_SOURCE[0]}"; exit 0 ;;
    esac
done

# -------- profile 红线（双 env 校验） --------
profile="${E2E_PROFILE:-}"
if [[ "${profile}" != "local" ]]; then
    echo "[reset-e2e] refuse: profile=${profile:-<unset>}, only 'local' is allowed" >&2
    exit 21
fi

# 交互确认（CI 用 --yes 跳过）
if [[ "${ASSUME_YES}" -ne 1 && -t 0 ]]; then
    read -r -p "[reset-e2e] Will DELETE E2E test data on profile=local. Continue? [y/N] " ans
    [[ "${ans}" =~ ^[Yy]$ ]] || { echo "[reset-e2e] aborted by user"; exit 0; }
fi

if ! command -v psql >/dev/null 2>&1; then
    echo "[reset-e2e] psql not found in PATH" >&2
    exit 24
fi

# -------- 计算确定性 ID（与 seed 一致） --------
SIGN_JWT="${SIGN_JWT_BIN:-}"
if [[ -z "${SIGN_JWT}" ]]; then
    if [[ -x "${REPO_ROOT}/target/debug/sign-jwt" ]]; then
        SIGN_JWT="${REPO_ROOT}/target/debug/sign-jwt"
    elif [[ -x "${REPO_ROOT}/target/release/sign-jwt" ]]; then
        SIGN_JWT="${REPO_ROOT}/target/release/sign-jwt"
    else
        ( cd "${REPO_ROOT}" && cargo build -q -p voice-room-shared --bin sign-jwt ) || {
            echo "[reset-e2e] sign-jwt build failed" >&2; exit 24; }
        SIGN_JWT="${REPO_ROOT}/target/debug/sign-jwt"
    fi
fi

USER_A_ID=$("${SIGN_JWT}" --uuid5 user_a)
USER_B_ID=$("${SIGN_JWT}" --uuid5 user_b)
ROOM_ID=$("${SIGN_JWT}"   --uuid5 room_main)

# psql 包装：用 DATABASE_URL 优先
PSQL=(psql -v ON_ERROR_STOP=1 -X -A -t)
if [[ -n "${DATABASE_URL:-}" ]]; then
    PSQL+=("${DATABASE_URL}")
fi

run_psql() {
    if ! "${PSQL[@]}" -c "$1"; then
        echo "[reset-e2e] psql failed on: $1" >&2
        exit 24
    fi
}

count_psql() {
    "${PSQL[@]}" -c "$1" 2>/dev/null | tr -d '[:space:]' || echo "?"
}

# -------- 关联子表（best-effort，存在才删） --------
# 通过 information_schema 查表名后再 DELETE，避免缺表时报错。
RELATED_SQL=$(cat <<SQL
DO \$\$
DECLARE
    e2e_room uuid := '${ROOM_ID}';
    e2e_a    uuid := '${USER_A_ID}';
    e2e_b    uuid := '${USER_B_ID}';
BEGIN
    -- 房间相关子表
    IF to_regclass('public.room_members')      IS NOT NULL THEN EXECUTE 'DELETE FROM room_members      WHERE room_id = \$1 OR user_id IN (\$2,\$3)' USING e2e_room, e2e_a, e2e_b; END IF;
    IF to_regclass('public.room_governance')   IS NOT NULL THEN EXECUTE 'DELETE FROM room_governance   WHERE room_id = \$1' USING e2e_room; END IF;
    IF to_regclass('public.room_mute')         IS NOT NULL THEN EXECUTE 'DELETE FROM room_mute         WHERE room_id = \$1' USING e2e_room; END IF;
    IF to_regclass('public.room_kick')         IS NOT NULL THEN EXECUTE 'DELETE FROM room_kick         WHERE room_id = \$1' USING e2e_room; END IF;
    IF to_regclass('public.gift_records')      IS NOT NULL THEN EXECUTE 'DELETE FROM gift_records      WHERE room_id = \$1 OR sender_id IN (\$2,\$3) OR receiver_id IN (\$2,\$3)' USING e2e_room, e2e_a, e2e_b; END IF;
    IF to_regclass('public.gift_orders')       IS NOT NULL THEN EXECUTE 'DELETE FROM gift_orders       WHERE room_id = \$1 OR sender_id IN (\$2,\$3) OR receiver_id IN (\$2,\$3)' USING e2e_room, e2e_a, e2e_b; END IF;
    IF to_regclass('public.wallet_tx')         IS NOT NULL THEN EXECUTE 'DELETE FROM wallet_tx         WHERE user_id IN (\$1,\$2)' USING e2e_a, e2e_b; END IF;
    IF to_regclass('public.events')            IS NOT NULL THEN EXECUTE 'DELETE FROM events            WHERE user_id IN (\$1,\$2)' USING e2e_a, e2e_b; END IF;
END
\$\$;
SQL
)

echo "[reset] applying related-table cleanup"
"${PSQL[@]}" -c "${RELATED_SQL}" >/dev/null || { echo "[reset-e2e] related cleanup failed" >&2; exit 24; }

# 主三表 — 输出 -N 删除行数
USERS_DEL=$(count_psql "WITH d AS (DELETE FROM users  WHERE phone LIKE '+96650000090%' OR id IN ('${USER_A_ID}','${USER_B_ID}') RETURNING 1) SELECT COUNT(*) FROM d;")
ADMINS_DEL=$(count_psql "WITH d AS (DELETE FROM admins WHERE username IN ('e2e_admin','e2e_op','e2e_cs','e2e_fin') RETURNING 1) SELECT COUNT(*) FROM d;")
ROOMS_DEL=$(count_psql "WITH d AS (DELETE FROM rooms  WHERE id = '${ROOM_ID}' RETURNING 1) SELECT COUNT(*) FROM d;")

echo "[reset] users -${USERS_DEL} admins -${ADMINS_DEL} rooms -${ROOMS_DEL}"

# -------- Redis（best-effort） --------
if command -v redis-cli >/dev/null 2>&1 && [[ -n "${REDIS_URL:-}" ]]; then
    for pattern in "room:${ROOM_ID}:*" "user:${USER_A_ID}:*" "user:${USER_B_ID}:*" "kicked:${ROOM_ID}:*" "mic_muted:${ROOM_ID}:*" "chat_muted:${ROOM_ID}:*"; do
        keys=$(redis-cli -u "${REDIS_URL}" --scan --pattern "${pattern}" 2>/dev/null || true)
        if [[ -n "${keys}" ]]; then
            # shellcheck disable=SC2086
            echo "${keys}" | xargs -I{} redis-cli -u "${REDIS_URL}" DEL {} >/dev/null
        fi
    done
    echo "[reset] redis: cleared E2E key prefixes"
else
    echo "[reset] redis: skipped (no redis-cli or REDIS_URL)"
fi

# -------- 回填文件 --------
rm -f "${OUT_FILE}"
echo "[reset] removed ${OUT_FILE#${REPO_ROOT}/}"

echo "[reset] done"
exit 0
