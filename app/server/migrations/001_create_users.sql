-- T-00001: users 表初始化
-- 参见 doc/protocol.md §六 6.1
-- TDD 验收用例：
--   [x] phone 有条件唯一索引（仅非软删除行）
--   [x] coin_balance BIGINT DEFAULT 0
--   [x] vip_level SMALLINT DEFAULT 0
--   [x] deleted_at 支持软删除
--   [x] is_banned 支持封禁

CREATE TABLE IF NOT EXISTS users (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    phone        VARCHAR(20) NOT NULL,
    nickname     VARCHAR(50) NOT NULL,
    avatar       TEXT,
    is_banned    BOOLEAN     NOT NULL DEFAULT FALSE,
    coin_balance BIGINT      NOT NULL DEFAULT 0,
    vip_level    SMALLINT    NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ
);

-- 条件唯一索引：仅对未软删除用户的手机号保证唯一
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_phone_active
    ON users(phone)
    WHERE deleted_at IS NULL;
