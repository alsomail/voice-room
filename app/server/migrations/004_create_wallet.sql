-- T-00017: 钱包 Schema 与迁移
-- 参见 doc/tds/server/T-00017.md
-- TDD 验收用例：
--   [x] W01: 迁移可重入（IF NOT EXISTS + DEFAULT 0）
--   [x] W02: 新注册用户 diamond_balance 默认 0
--   [x] W03: diamond_balance CHECK >= 0，拒绝负值（PG 23514）
--   [x] W04: wallet_transactions.balance_after CHECK >= 0
--   [x] W05: 复合索引 (user_id, created_at DESC) 存在
--   [x] W06: 存量 users 迁移后 diamond_balance = 0

-- ─────────────────────────────────────────────────────────────
-- 1. users 表新增 diamond_balance 字段
--    ADD COLUMN IF NOT EXISTS 保证幂等：重复执行不报错
-- ─────────────────────────────────────────────────────────────
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS diamond_balance BIGINT NOT NULL DEFAULT 0
        CHECK (diamond_balance >= 0);

-- ─────────────────────────────────────────────────────────────
-- 2. 钱包流水表
--    CREATE TABLE IF NOT EXISTS 保证幂等
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID        NOT NULL REFERENCES users(id),
    type          VARCHAR(32) NOT NULL,   -- gift_send | gift_receive | admin_adjust | recharge | refund
    amount        BIGINT      NOT NULL,   -- 正数=加款，负数=扣款
    balance_after BIGINT      NOT NULL
                  CHECK (balance_after >= 0),
    ref_id        UUID,                   -- 关联 gift_record_id / admin_log_id 等
    reason        TEXT,
    operator_id   UUID        REFERENCES users(id),  -- 非空表示管理员操作
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────
-- 3. 索引
--    CREATE INDEX IF NOT EXISTS 保证幂等
-- ─────────────────────────────────────────────────────────────

-- 复合索引：按用户查流水（分页按时间倒序）
CREATE INDEX IF NOT EXISTS idx_wallet_txn_user_created
    ON wallet_transactions(user_id, created_at DESC);

-- 类型索引：按流水类型查询（对账 / Admin 统计）
CREATE INDEX IF NOT EXISTS idx_wallet_txn_type
    ON wallet_transactions(type, created_at DESC);
