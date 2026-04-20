# 七、数据模型（模块1相关）

## 7.1 users 表

```sql
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    phone       VARCHAR(20) NOT NULL UNIQUE,
    nickname    VARCHAR(50) NOT NULL,
    avatar      TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    coin_balance BIGINT NOT NULL DEFAULT 0,
    vip_level    SMALLINT NOT NULL DEFAULT 0,
    deleted_at  TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_users_phone ON users(phone) WHERE deleted_at IS NULL;
```

## 7.2 验证码存储 (Redis)

> 验证码使用 Redis 存储，不使用 PostgreSQL 表。

**Redis Key 设计**:

| Key 模式 | 类型 | TTL | 说明 |
|----------|------|-----|------|
| `sms:code:{phone}` | Hash | 300s | 验证码内容 + 尝试次数 |
| `sms:cooldown:{phone}` | String | 60s | 发送冷却标记 |
| `sms:daily:{phone}:{date}` | String (INCR) | 86400s | 每日发送计数 |

**`sms:code:{phone}` Hash 结构**:
```
HSET sms:code:+966512345678 code "123456" attempts 0 max_attempts 5
EXPIRE sms:code:+966512345678 300
```

**验证流程**:
1. 发送验证码前：检查 `sms:cooldown:{phone}` 是否存在（冷却中）；检查 `sms:daily:{phone}:{date}` 是否超限
2. 发送成功后：写入 `sms:code:{phone}` (TTL 300s) + `sms:cooldown:{phone}` (TTL 60s) + INCR `sms:daily:{phone}:{date}`
3. 登录校验时：HGET `sms:code:{phone}` 取 code 比对，HINCRBY attempts 1，超过 max_attempts 返回 40105
4. 校验成功后：DEL `sms:code:{phone}` 使验证码一次性作废

## 7.3 admins 表

```sql
CREATE TABLE admins (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username     VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(200) NOT NULL,
    role         VARCHAR(20) NOT NULL DEFAULT 'operator',
    display_name VARCHAR(100),
    is_active    BOOLEAN NOT NULL DEFAULT true,
    last_login_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 角色枚举约束
ALTER TABLE admins ADD CONSTRAINT chk_admin_role
    CHECK (role IN ('super_admin', 'operator', 'cs', 'finance'));
```

**初始数据**: 部署时通过 migration seed 插入默认 super_admin 账号。

## 7.4 admin_logs 表

```sql
CREATE TABLE admin_logs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id    UUID NOT NULL REFERENCES admins(id),
    action      VARCHAR(50) NOT NULL,
    target_type VARCHAR(20),
    target_id   UUID,
    detail      JSONB,
    ip_address  INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_admin_logs_admin_id ON admin_logs(admin_id, created_at DESC);
CREATE INDEX idx_admin_logs_action ON admin_logs(action, created_at DESC);
```

**action 枚举**: `admin_login`, `ban_user`, `unban_user`, `close_room`, `broadcast_notice`, `create_admin`, `update_admin`
