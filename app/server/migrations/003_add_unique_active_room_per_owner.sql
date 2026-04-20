-- T-00007: 每个用户同时只能拥有一个 active 房间
-- 通过部分唯一索引强制约束（仅对未删除的 active 行）
CREATE UNIQUE INDEX IF NOT EXISTS idx_rooms_owner_active
    ON rooms (owner_id)
    WHERE status = 'active' AND deleted_at IS NULL;
