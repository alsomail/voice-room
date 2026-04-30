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
--   :user_a_id :user_b_id :user_muted_id :room_id :admin_super_id :admin_op_id :admin_cs_id :admin_fin_id
--   T-0000S 新增：:user_muted_id（与 chat_muted:{room}:{user} Redis key 对接）

\set ON_ERROR_STOP on

BEGIN;

-- ============================================================================
-- 1) users — E2E User A / B / Muted / C ~ L（T-0000R Round1：扩充到 12 个房主）
-- ============================================================================
INSERT INTO users (id, phone, nickname, is_banned, coin_balance, diamond_balance, vip_level, created_at, updated_at)
VALUES
    (:'user_a_id'::uuid,     '+966500000900', 'E2E User A',     FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    (:'user_b_id'::uuid,     '+966500000901', 'E2E User B',     FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    (:'user_muted_id'::uuid, '+966500000902', 'E2E User Muted', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000011'::uuid, '+966500000903', 'E2E User C', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000012'::uuid, '+966500000904', 'E2E User D', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000013'::uuid, '+966500000905', 'E2E User E', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000014'::uuid, '+966500000906', 'E2E User F', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000015'::uuid, '+966500000907', 'E2E User G', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000016'::uuid, '+966500000908', 'E2E User H', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000017'::uuid, '+966500000909', 'E2E User I', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000018'::uuid, '+966500000910', 'E2E User J', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-000000000019'::uuid, '+966500000911', 'E2E User K', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now()),
    ('10000000-0000-4000-8000-00000000001A'::uuid, '+966500000912', 'E2E User L', FALSE, 100000, 100000, 0, '2026-01-01 00:00:00+00', now())
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
-- 3) rooms — 12 个测试房间（T-0000R Round1：9 active + 3 closed，满足分页验证 ≥12 条）
--    每个用户只能有一个 active 房间（idx_rooms_owner_active 约束），故分配给不同用户
-- ============================================================================
-- 先关闭所有 E2E 用户的其他活跃房间，防止 idx_rooms_owner_active 约束冲突
UPDATE rooms SET status='closed', updated_at=now()
WHERE owner_id IN (
    :'user_a_id'::uuid,
    :'user_b_id'::uuid,
    '10000000-0000-4000-8000-000000000011'::uuid,
    '10000000-0000-4000-8000-000000000012'::uuid,
    '10000000-0000-4000-8000-000000000013'::uuid,
    '10000000-0000-4000-8000-000000000014'::uuid,
    '10000000-0000-4000-8000-000000000015'::uuid,
    '10000000-0000-4000-8000-000000000016'::uuid,
    '10000000-0000-4000-8000-000000000017'::uuid,
    '10000000-0000-4000-8000-000000000018'::uuid,
    '10000000-0000-4000-8000-000000000019'::uuid,
    '10000000-0000-4000-8000-00000000001A'::uuid
) AND status = 'active' AND id NOT IN (
    :'room_id'::uuid,
    '10000000-0000-4000-8000-000000000002'::uuid,
    '10000000-0000-4000-8000-000000000003'::uuid,
    '10000000-0000-4000-8000-000000000004'::uuid,
    '10000000-0000-4000-8000-000000000005'::uuid,
    '10000000-0000-4000-8000-000000000006'::uuid,
    '10000000-0000-4000-8000-000000000007'::uuid,
    '10000000-0000-4000-8000-000000000008'::uuid,
    '10000000-0000-4000-8000-000000000009'::uuid
);

-- ──────────────────────────────────────────────────────────────────────────
-- 9 个 active 房间（分页测试需要 ≥12 条数据时能显示第 2 页）
-- ──────────────────────────────────────────────────────────────────────────
-- Room 1: 主测试房间（User A 房主）
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

-- Room 2-9: 活跃房间（User B ~ I 房主）
INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000002'::uuid, :'user_b_id'::uuid, 'E2E Active Room 2', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000003'::uuid, '10000000-0000-4000-8000-000000000011'::uuid, 'E2E Active Room 3', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000004'::uuid, '10000000-0000-4000-8000-000000000012'::uuid, 'E2E Active Room 4', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000005'::uuid, '10000000-0000-4000-8000-000000000013'::uuid, 'E2E Active Room 5', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000006'::uuid, '10000000-0000-4000-8000-000000000014'::uuid, 'E2E Active Room 6', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000007'::uuid, '10000000-0000-4000-8000-000000000015'::uuid, 'E2E Active Room 7', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000008'::uuid, '10000000-0000-4000-8000-000000000016'::uuid, 'E2E Active Room 8', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-000000000009'::uuid, '10000000-0000-4000-8000-000000000017'::uuid, 'E2E Active Room 9', 'normal', 0, 'active', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'active',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

-- ──────────────────────────────────────────────────────────────────────────
-- 3 个 closed 房间（User J ~ L 房主，用于分页筛选测试）
-- ──────────────────────────────────────────────────────────────────────────
INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-00000000000A'::uuid, '10000000-0000-4000-8000-000000000018'::uuid, 'E2E Closed Room 1', 'normal', 0, 'closed', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'closed',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-00000000000B'::uuid, '10000000-0000-4000-8000-000000000019'::uuid, 'E2E Closed Room 2', 'normal', 0, 'closed', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'closed',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

INSERT INTO rooms (id, owner_id, title, room_type, member_count, status, password_hash, max_members, cover_url, category, created_at, updated_at)
VALUES
    ('10000000-0000-4000-8000-00000000000C'::uuid, '10000000-0000-4000-8000-00000000001A'::uuid, 'E2E Closed Room 3', 'normal', 0, 'closed', NULL, 8, '', 'chat', '2026-01-01 00:00:00+00', now())
ON CONFLICT (id) DO UPDATE SET
    owner_id     = EXCLUDED.owner_id,
    title        = EXCLUDED.title,
    room_type    = EXCLUDED.room_type,
    status       = 'closed',
    password_hash= NULL,
    max_members  = EXCLUDED.max_members,
    deleted_at   = NULL,
    updated_at   = now();

-- ============================================================================
-- gifts — E2E 礼物 seed（与 005_create_gifts.sql 保持一致，幂等）
-- ============================================================================
INSERT INTO gifts (code, name_en, name_ar, icon_url, price, tier, effect_level, sort_order)
VALUES
  ('rose_01',      'Rose',           'وردة',            '/assets/gifts/rose.png',       1,    1, 1, 10),
  ('coffee_01',    'Arabic Coffee',  'قهوة عربية',      '/assets/gifts/coffee.png',    10,    2, 2, 20),
  ('kaaba_01',     'Kaaba Candle',   'شمعة الكعبة',     '/assets/gifts/kaaba.png',     10,    2, 2, 21),
  ('camel_01',     'Desert Camel',   'جمل',             '/assets/gifts/camel.png',     66,    3, 3, 30),
  ('falcon_01',    'Golden Falcon',  'صقر ذهبي',        '/assets/gifts/falcon.png',    88,    3, 3, 31),
  ('moon_786',     'Bismillah Moon', 'هلال بسم الله',   '/assets/gifts/moon786.png',  786,    4, 4, 40),
  ('castle_01',    'Royal Castle',   'قصر ملكي',        '/assets/gifts/castle.png',   520,    4, 4, 41),
  ('diamond_ring', 'Diamond Ring',   'خاتم الماس',      '/assets/gifts/diamond.png', 1314,    5, 5, 50)
ON CONFLICT (code) DO NOTHING;

COMMIT;

-- 行数断言（machine-readable，wrapper 校验幂等）
\echo '--seed-counts--'
SELECT 'users:'   AS k, COUNT(*) AS n FROM users  WHERE id IN (
    :'user_a_id'::uuid,
    :'user_b_id'::uuid,
    :'user_muted_id'::uuid,
    '10000000-0000-4000-8000-000000000011'::uuid,
    '10000000-0000-4000-8000-000000000012'::uuid,
    '10000000-0000-4000-8000-000000000013'::uuid,
    '10000000-0000-4000-8000-000000000014'::uuid,
    '10000000-0000-4000-8000-000000000015'::uuid,
    '10000000-0000-4000-8000-000000000016'::uuid,
    '10000000-0000-4000-8000-000000000017'::uuid,
    '10000000-0000-4000-8000-000000000018'::uuid,
    '10000000-0000-4000-8000-000000000019'::uuid,
    '10000000-0000-4000-8000-00000000001A'::uuid
) AND deleted_at IS NULL
UNION ALL
SELECT 'admins:'  AS k, COUNT(*) AS n FROM admins WHERE username IN ('e2e_admin','e2e_op','e2e_cs','e2e_fin')
UNION ALL
SELECT 'rooms:'   AS k, COUNT(*) AS n FROM rooms  WHERE id IN (
    :'room_id'::uuid,
    '10000000-0000-4000-8000-000000000002'::uuid,
    '10000000-0000-4000-8000-000000000003'::uuid,
    '10000000-0000-4000-8000-000000000004'::uuid,
    '10000000-0000-4000-8000-000000000005'::uuid,
    '10000000-0000-4000-8000-000000000006'::uuid,
    '10000000-0000-4000-8000-000000000007'::uuid,
    '10000000-0000-4000-8000-000000000008'::uuid,
    '10000000-0000-4000-8000-000000000009'::uuid,
    '10000000-0000-4000-8000-00000000000A'::uuid,
    '10000000-0000-4000-8000-00000000000B'::uuid,
    '10000000-0000-4000-8000-00000000000C'::uuid
) AND deleted_at IS NULL;
