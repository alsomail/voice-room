-- T-00024: rooms 表扩字段 + 治理审计表迁移
-- 参见 doc/tds/server/T-00024.md
-- TDD 验收用例：
--   [x] S24-01 迁移可重入执行两次无报错（IF NOT EXISTS / DROP CONSTRAINT IF EXISTS）
--   [x] S24-02 category=invalid 被 CHECK 约束拒绝
--   [x] S24-03 存量房间迁移后 cover_url=空串、category=chat
--   [x] S24-04 idx_kick_records_room_ts 索引存在
--   [x] S24-05 room_mute_records.type=sms 被 CHECK 约束拒绝
--   [x] S24-06 admin_user_id 外键引用 users(id)，外键默认 RESTRICT（无 CASCADE）

-- ────────────────────────────────────────────────
-- 1. rooms 表扩展字段（幂等：ADD COLUMN IF NOT EXISTS）
-- ────────────────────────────────────────────────
ALTER TABLE rooms
    ADD COLUMN IF NOT EXISTS cover_url       TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS category        VARCHAR(32) NOT NULL DEFAULT 'chat',
    ADD COLUMN IF NOT EXISTS password_hash   VARCHAR(60),
    ADD COLUMN IF NOT EXISTS announcement    TEXT,
    ADD COLUMN IF NOT EXISTS admin_user_id   UUID REFERENCES users(id);

-- ────────────────────────────────────────────────
-- 2. category 枚举约束（先 DROP 再 ADD 保证幂等）
-- ────────────────────────────────────────────────
ALTER TABLE rooms
    DROP CONSTRAINT IF EXISTS chk_room_category,
    ADD CONSTRAINT chk_room_category
        CHECK (category IN ('chat','emotion','music','game','matchmaking','other'));

-- ────────────────────────────────────────────────
-- 3. 踢人审计表
-- ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS room_kick_records (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id          UUID NOT NULL REFERENCES rooms(id),
    target_user_id   UUID NOT NULL REFERENCES users(id),
    operator_user_id UUID NOT NULL REFERENCES users(id),
    reason           TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_kick_records_room_ts    ON room_kick_records(room_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_kick_records_target_ts  ON room_kick_records(target_user_id, created_at DESC);

-- ────────────────────────────────────────────────
-- 4. 禁言审计表
-- ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS room_mute_records (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id          UUID NOT NULL REFERENCES rooms(id),
    target_user_id   UUID NOT NULL REFERENCES users(id),
    operator_user_id UUID NOT NULL REFERENCES users(id),
    type             VARCHAR(8) NOT NULL CHECK (type IN ('mic','chat')),
    duration_sec     INT NOT NULL CHECK (duration_sec >= 0), -- 0 = 解除禁言
    reason           TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_mute_records_room_ts           ON room_mute_records(room_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_mute_records_target_type_ts    ON room_mute_records(target_user_id, type, created_at DESC);
