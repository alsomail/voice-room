-- T-0000G: E2E Seed SQL（幂等）
-- 仅在 E2E_PROFILE=local 由 scripts/dev/seed-e2e.sh wrapper 执行。
--
-- 数据契约：doc/tds/infra/T-0000G.md §2.4
-- 真值字段：以 app/server/migrations/00{1,2}_*.sql + app/adminServer/migrations/001_*.sql 为准
--
-- ⚠️  schema 偏差（已记录于 §四【实现结果】）：
--   - users 表无 role 列 → 仅写入 id/phone/nickname；role 由 JWT claim 表达
--   - rooms 表使用 title/room_type/max_members（非 TDS 描述的 name/capacity/is_locked）
--   - admins.role 合法枚举 = super_admin|operator|cs|finance（非 TDS 描述的 admin/op/cs/fin 简写）
--
-- 入参变量（由 wrapper 通过 psql -v 注入）：
--   :user_a_id :user_b_id :room_id :admin_super_id :admin_op_id :admin_cs_id :admin_fin_id

\set ON_ERROR_STOP on

BEGIN;

-- ============================================================================
-- 1) users — E2E User A / B
-- ============================================================================
INSERT INTO users (id, phone, nickname, is_banned, coin_balance, diamond_balance, vip_level, created_at, updated_at)
VALUES
    (:'user_a_id'::uuid, '+966500000900', 'E2E User A', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    (:'user_b_id'::uuid, '+966500000901', 'E2E User B', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    phone      = EXCLUDED.phone,
    nickname   = EXCLUDED.nickname,
    is_banned  = EXCLUDED.is_banned,
    deleted_at = NULL,
    updated_at = now();

-- ============================================================================
-- 2) admins — 5 角色（super_admin / operator / cs / finance / disabled）
--    password_hash 复用 003_seed_super_admin.sql 中已审核的 bcrypt(cost=12) hash，
--    明文 admin_password_change_me（仅本地 E2E 使用）。
-- ============================================================================
-- 先清理可能残留的不同 id 但相同 username 的旧记录（幂等保证）
DELETE FROM admin_logs WHERE admin_id IN (
  SELECT id FROM admins WHERE username IN ('e2e_admin','e2e_op','e2e_cs','e2e_fin','e2e_disabled')
);
DELETE FROM admins WHERE username IN ('e2e_admin','e2e_op','e2e_cs','e2e_fin','e2e_disabled');
INSERT INTO admins (id, username, password_hash, role, display_name, is_active, created_at, updated_at)
VALUES
    (:'admin_super_id'::uuid,    'e2e_admin',    '$2b$12$FQg0m0lriSYWkWBvArC14utVu.2nNEthominTTYS7Syjm6dwtu.Qm', 'super_admin', 'E2E Super Admin', TRUE,  '2026-01-01 00:00:00+00', now()),
    (:'admin_op_id'::uuid,       'e2e_op',       '$2b$12$FQg0m0lriSYWkWBvArC14utVu.2nNEthominTTYS7Syjm6dwtu.Qm', 'operator',    'E2E Operator',    TRUE,  '2026-01-01 00:00:00+00', now()),
    (:'admin_cs_id'::uuid,       'e2e_cs',       '$2b$12$FQg0m0lriSYWkWBvArC14utVu.2nNEthominTTYS7Syjm6dwtu.Qm', 'cs',          'E2E CS',          TRUE,  '2026-01-01 00:00:00+00', now()),
    (:'admin_fin_id'::uuid,      'e2e_fin',      '$2b$12$FQg0m0lriSYWkWBvArC14utVu.2nNEthominTTYS7Syjm6dwtu.Qm', 'finance',     'E2E Finance',     TRUE,  '2026-01-01 00:00:00+00', now()),
    (:'admin_disabled_id'::uuid, 'e2e_disabled', '$2b$12$FQg0m0lriSYWkWBvArC14utVu.2nNEthominTTYS7Syjm6dwtu.Qm', 'operator',    'E2E Disabled',    FALSE, '2026-01-01 00:00:00+00', now());

-- ============================================================================
-- 3) rooms — 唯一测试房间，房主 = User A
-- ============================================================================
-- 先关闭 User A 其他活跃房间，防止 idx_rooms_owner_active 约束冲突
UPDATE rooms SET status='closed', updated_at=now()
WHERE owner_id = :'user_a_id'::uuid AND status = 'active' AND id != :'room_id'::uuid;

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    (:'room_id'::uuid, :'user_a_id'::uuid, 'E2E Test Room', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

COMMIT;

-- 行数断言（machine-readable，wrapper 校验幂等）
\echo '--seed-counts--'
SELECT 'users:'   AS k, COUNT(*) AS n FROM users  WHERE id IN (:'user_a_id'::uuid, :'user_b_id'::uuid) AND deleted_at IS NULL
UNION ALL
SELECT 'admins:'  AS k, COUNT(*) AS n FROM admins WHERE username IN ('e2e_admin','e2e_op','e2e_cs','e2e_fin')
UNION ALL
SELECT 'rooms:'   AS k, COUNT(*) AS n FROM rooms  WHERE id = :'room_id'::uuid AND deleted_at IS NULL;
