# 榜单 API 协议文档

> **对应 TDS**: `doc/tds/server/T-00021.md`  
> **对应 Server 架构**: `doc/arch/server/index.md` → Ranking 模块  
> **最后更新**: 2026-04-22

---

## 1. GET /api/v1/ranking — 查询榜单

**请求方式**: GET  
**鉴权**: ✅ 需要 JWT Token（Bearer 或 Query 参数 `?token=xxx`）  
**协议版本**: v1

### 1.1 请求参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `type` | string | ✅ | - | 榜单类型：`charm`（魅力榜）、`wealth`（财富榜）；其他值返回 40003 |
| `period` | string | ❌ | `day` | 榜单周期：`day`（日榜）、`week`（周榜）；其他值返回 40003 |
| `limit` | number | ❌ | `50` | 返回数量，范围 1-100；超出范围返回 40003 |

### 1.2 请求示例

```http
GET /api/v1/ranking?type=charm&period=day&limit=50 HTTP/1.1
Authorization: Bearer <jwt_token>
Accept: application/json
```

或使用 Query Token：
```http
GET /api/v1/ranking?type=charm&period=day&limit=50&token=<jwt_token> HTTP/1.1
```

### 1.3 成功响应 (200 OK)

```json
{
  "code": 0,
  "request_id": "uuid",
  "data": {
    "type": "charm",
    "period": "day",
    "period_key": "2026-04-22",
    "items": [
      {
        "rank": 1,
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "nickname": "العم جمال",
        "avatar": "https://assets.example.com/avatars/user123.jpg",
        "score": 123456,
        "medal": "gold"
      },
      {
        "rank": 2,
        "user_id": "550e8400-e29b-41d4-a716-446655440001",
        "nickname": "أم فاطمة",
        "avatar": "https://assets.example.com/avatars/user456.jpg",
        "score": 89012,
        "medal": "silver"
      },
      {
        "rank": 3,
        "user_id": "550e8400-e29b-41d4-a716-446655440002",
        "nickname": "سامي",
        "avatar": "https://assets.example.com/avatars/user789.jpg",
        "score": 56789,
        "medal": "bronze"
      }
    ],
    "me": {
      "rank": 42,
      "score": 6800
    }
  }
}
```

#### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `type` | string | 所查询的榜单类型（charm/wealth） |
| `period` | string | 所查询的榜单周期（day/week） |
| `period_key` | string | 当前榜单的日期或周次，格式 YYYY-MM-DD；便于前端判断是否跨日/跨周 |
| `items[].rank` | number | 排名（1-based，从 1 开始） |
| `items[].user_id` | string | 用户 UUID |
| `items[].nickname` | string | 用户昵称 |
| `items[].avatar` | string | 用户头像 URL |
| `items[].score` | number | 榜单积分（日榜：魅力值；周榜：魅力值累计） |
| `items[].medal` | string | 奖牌标识：`gold`（第1名）、`silver`（第2名）、`bronze`（第3名）；其他名次为 `null` |
| `me.rank` | number \| null | 当前用户排名（1-based）；未入榜返回 `null` |
| `me.score` | number | 当前用户积分；未入榜返回 `0` |

### 1.4 错误响应

#### 40003 — 参数错误

```json
{
  "code": 40003,
  "request_id": "uuid",
  "message": "参数错误",
  "safe_message": "参数错误"
}
```

**触发条件**：
- `type` 不为 `charm`/`wealth`
- `period` 不为 `day`/`week`
- `limit` 不在 1-100 范围内

#### 401 — 未授权

```json
{
  "code": 401,
  "request_id": "uuid",
  "message": "Unauthorized",
  "safe_message": "未授权"
}
```

**触发条件**：
- 缺少 JWT Token
- JWT 签名无效或已过期

#### 400 — 其他业务错误

```json
{
  "code": 400,
  "request_id": "uuid",
  "message": "Internal error",
  "safe_message": "查询失败"
}
```

**触发条件**：
- Redis/数据库连接异常
- 用户信息查询失败

---

## 2. 时区与日期切换

### 2.1 时区规则

- **当前实现**：使用 UTC 时区生成榜单 key（MVP 简化，与 T-00020 SendGift 保持一致）
- **后续计划**：下一 milestone 切换为 `Asia/Riyadh` 时区（UTC+3），支持真正的沙特当地 00:00 日榜切换

### 2.2 日榜切换

- **触发时机**：每天 UTC 00:00（当前）；后续计划改为 Riyadh 03:00（即 Riyadh 本地 00:00）
- **旧榜归档**：昨日榜单自动归档到 `ranking_archive:charm:day:2026-04-21` 等 key，保留 7 天
- **补偿机制**：启动时自动检测落差，若有漏掉的日期则逐日补偿执行

### 2.3 周榜切换

- **触发时机**：每周日 UTC 00:00（当前）；后续计划改为 Riyadh 本地周日 00:00
- **旧榜归档**：上周榜单自动归档到 `ranking_archive:charm:week:2026-W17` 等 key，保留 7 天

---

## 3. Redis 数据结构

### 3.1 榜单 ZSet

```
Key: ranking:{type}:{period}:{date|week}
    ↓
Sorted Set
    member: {user_id}
    score:  {魅力值}
```

**示例**：
- `ranking:charm:day:2026-04-22` — 2026 年 4 月 22 日魅力日榜
- `ranking:wealth:week:2026-W17` — 2026 年第 17 周财富周榜

### 3.2 榜单归档

```
Key: ranking_archive:{type}:{period}:{date|week}
TTL: 7 days
```

**示例**：
- `ranking_archive:charm:day:2026-04-21` — 过期日榜
- `ranking_archive:wealth:week:2026-W16` — 过期周榜

---

## 4. 客户端集成建议

### 4.1 跨日刷新

前端可通过对比 `period_key` 字段判断是否跨日：

```javascript
const response = await fetch('/api/v1/ranking?type=charm&period=day');
const { data } = await response.json();

if (lastPeriodKey !== data.period_key) {
  // 跨日了，清空缓存重新加载
  cacheManager.clear('ranking:charm:day');
  lastPeriodKey = data.period_key;
}
```

### 4.2 当前用户排名

```javascript
const { me } = data;

if (me.rank === null) {
  console.log('未上榜，继续加油！');
} else {
  console.log(`您排在第 ${me.rank} 名，得分 ${me.score}`);
}
```

### 4.3 奖牌展示

```javascript
const medalIcons = {
  'gold': '🥇',
  'silver': '🥈',
  'bronze': '🥉',
  null: ''
};

items.forEach(item => {
  console.log(`${item.rank}. ${item.nickname} ${medalIcons[item.medal]} - ${item.score}`);
});
```

---

## 文档变更历史

- 2026-04-22: 初始版本，T-00021 DoD 同步创建；包含完整请求/响应示例、错误码映射、时区说明
