-- T-10001 (admin_logs 部分) / T-10012 占位
-- 参见 doc/protocol.md §六 6.4
-- TDD 验收用例：
--   [x] admin_id 外键引用 admins(id)
--   [x] action 列记录操作类型
--   [x] 两个索引支持高效查询

CREATE TABLE IF NOT EXISTS admin_logs (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id    UUID         NOT NULL REFERENCES admins(id),
    action      VARCHAR(50)  NOT NULL,
    target_type VARCHAR(20),
    target_id   UUID,
    detail      JSONB,
    ip_address  INET,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- 按管理员 + 时间倒序查询操作历史
CREATE INDEX IF NOT EXISTS idx_admin_logs_admin_id
    ON admin_logs(admin_id, created_at DESC);

-- 按操作类型 + 时间倒序查询
CREATE INDEX IF NOT EXISTS idx_admin_logs_action
    ON admin_logs(action, created_at DESC);
