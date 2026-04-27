-- T-00043: 聊天消息持久化表
-- 参见 doc/tds/server/T-00043.md
-- TDD 验收用例：
--   [x] R-1 迁移幂等可重复执行（IF NOT EXISTS / 索引名稳定）
--   [x] U-1 SendMessage 持久化插入一行
--   [x] U-3 历史查询按 created_at DESC 排序
--   [x] B-3 并发插入不丢失

CREATE TABLE IF NOT EXISTS chat_messages (
    id         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id    UUID         NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id    UUID         REFERENCES users(id) ON DELETE SET NULL,
    content    TEXT         NOT NULL CHECK (char_length(content) > 0 AND char_length(content) <= 500),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- 房间历史查询：最常见路径（room_id + 倒序时间）
CREATE INDEX IF NOT EXISTS idx_chat_messages_room_time
    ON chat_messages (room_id, created_at DESC);

-- 用户查询：举报 / 申诉场景
CREATE INDEX IF NOT EXISTS idx_chat_messages_user_time
    ON chat_messages (user_id, created_at DESC);
