-- T-00006: rooms 表初始化（App Server 第二个迁移）
-- 参见 doc/tds/server/T-00006.md
-- TDD 验收用例：
--   [x] UUID 主键，gen_random_uuid() 自动生成
--   [x] owner_id 外键关联 users(id) ON DELETE RESTRICT
--   [x] title 长度 1-30 字符约束
--   [x] room_type 枚举约束 (normal/password/paid)
--   [x] status 枚举约束 (active/closed)
--   [x] member_count 非负约束
--   [x] member_count <= max_members 上界约束
--   [x] max_members 正数约束
--   [x] deleted_at 支持软删除
--   [x] 热度/列表索引过滤软删除行 (WHERE deleted_at IS NULL)

CREATE TABLE IF NOT EXISTS rooms (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id      UUID         NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    title         VARCHAR(30)  NOT NULL,
    room_type     VARCHAR(20)  NOT NULL DEFAULT 'normal',
    member_count  INT          NOT NULL DEFAULT 0,
    status        VARCHAR(20)  NOT NULL DEFAULT 'active',
    password_hash VARCHAR(255),
    max_members   INT          NOT NULL DEFAULT 50,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ,

    CONSTRAINT chk_rooms_title_length CHECK (char_length(title) BETWEEN 1 AND 30),
    CONSTRAINT chk_rooms_room_type CHECK (room_type IN ('normal', 'password', 'paid')),
    CONSTRAINT chk_rooms_status CHECK (status IN ('active', 'closed')),
    CONSTRAINT chk_rooms_member_count_non_negative CHECK (member_count >= 0),
    CONSTRAINT chk_rooms_member_count_le_max CHECK (member_count <= max_members),
    CONSTRAINT chk_rooms_max_members_positive CHECK (max_members > 0)
);

CREATE INDEX IF NOT EXISTS idx_rooms_status_created_at ON rooms(status, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_rooms_owner_id ON rooms(owner_id);
CREATE INDEX IF NOT EXISTS idx_rooms_member_count ON rooms(member_count DESC) WHERE deleted_at IS NULL;
