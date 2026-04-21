-- T-00020: gift_records 表 + users.charm_balance 列
-- 幂等：所有语句使用 IF NOT EXISTS / ADD COLUMN IF NOT EXISTS

-- 1. users 增加 charm_balance（收礼魅力值）
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS charm_balance BIGINT NOT NULL DEFAULT 0 CHECK (charm_balance >= 0);

-- 2. gift_records 送礼历史表
CREATE TABLE IF NOT EXISTS gift_records (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id    UUID NOT NULL REFERENCES users(id),
    receiver_id  UUID NOT NULL REFERENCES users(id),
    room_id      UUID NOT NULL REFERENCES rooms(id),
    gift_id      UUID NOT NULL REFERENCES gifts(id),
    count        INT NOT NULL CHECK (count >= 1 AND count <= 9999),
    total_price  BIGINT NOT NULL CHECK (total_price >= 1),
    msg_id       VARCHAR(64) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (sender_id, msg_id)  -- 幂等约束
);

CREATE INDEX IF NOT EXISTS idx_gift_records_receiver_created
    ON gift_records(receiver_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_gift_records_room_created
    ON gift_records(room_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_gift_records_sender_created
    ON gift_records(sender_id, created_at DESC);
