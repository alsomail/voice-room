# 测试套件：GIFT 礼物（API）

> **需求模糊点 (Ambiguity Notes)**：
> - 礼物图片上传白名单：png/jpg/webp；大小 ≤500KB（按常识取值，若产品文档不同请以 TDS 为准）。

覆盖 Task：T-00017（礼物列表）、T-00018（SendGift 原子事务）、T-10013（Admin 礼物管理）。

---

## TC-GIFT-00001：GET /gifts/list - 排序 + 缓存 + Accept-Language
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. gifts 表存在 8 个 active、2 个 is_active=false 的礼物。
2. 部分礼物包含 name_i18n JSON 字段，含 ar/en/zh。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/gifts/list` Bearer TOKEN_U1 | 200，items 长度=8（下架不返回） |
| 2 | `AppServer` | items 顺序 | 先按 tier 升序，再按 sort_order 升序 |
| 3 | `AppServer` | 携带 `Accept-Language: ar` | items[i].name 返回阿拉伯语文本 |
| 4 | `AppServer` | 携带 `Accept-Language: en` | 返回英文；无对应翻译则 fallback 至默认语 |
| 5 | `AppServer` | 连续 10 次请求同一 URL | 平均响应时间 ≤50ms（命中缓存） |

**【数据清理】**
- 无。

---

## TC-GIFT-00002：SendGift - 原子事务闭环 + WS 推送
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 余额=1000；U2 在麦位 3；礼物 G1 price=100。
2. R1 内 U1/U2/U3 活跃。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 WS 发 `{"type":"SendGift","payload":{"gift_id":"{G1}","receiver_ids":["{U2}"],"count":1},"msg_id":"g1"}` | U1 收 Ack code=0 |
| 2 | `DB` | users.coin_balance WHERE id=U1 | =900 |
| 3 | `DB` | users.charm_value WHERE id=U2 | 增加 100 |
| 4 | `DB` | wallet_transactions 新增 2 行：U1 expense 100（ref_type=`gift_send`），U2 income 100（ref_type=`gift_receive`） | 存在且 ref_id 相同 |
| 5 | `DB` | gift_records 新增 1 行 | sender=U1，receiver=U2，gift_id=G1，count=1，amount=100 |
| 6 | `Redis` | ZSCORE `ranking:charm:day:{today}` U2 | +100 |
| 7 | `Redis` | ZSCORE `ranking:wealth:day:{today}` U1 | +100 |
| 8 | `AppServer` | U1 WS | 收 BalanceUpdated payload.coin_balance=900 |
| 9 | `AppServer` | R1 内 U1/U2/U3 | 都收 `{"type":"GiftReceived","payload":{"sender":"{U1}","receivers":["{U2}"],"gift_id":"{G1}","count":1,"effect_level":"L1"}}` |

**【数据清理】**
- 回滚测试数据。

---

## TC-GIFT-00003：SendGift - 余额不足 40290 + 完整回滚
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 余额=50；礼物 G1 price=100。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 发 SendGift count=1 | Ack code=40290，message 含 "INSUFFICIENT_BALANCE" |
| 2 | `DB` | users.coin_balance=50 | 不变 |
| 3 | `DB` | users.charm_value（U2） | 不变 |
| 4 | `DB` | wallet_transactions/gift_records | 无新增 |
| 5 | `Redis` | ranking ZSet | 无变动 |
| 6 | `AppServer` | 房间广播 | 无 GiftReceived |

**【数据清理】**
- 无。

---

## TC-GIFT-00004：SendGift - 接收者离麦/不存在 40403
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U2 不在麦位，UZ 用户不存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 SendGift receiver_ids=[U2]（不在麦） | Ack code=40403（RECEIVER_UNAVAILABLE） |
| 2 | `AppServer` | U1 SendGift receiver_ids=[UZ] | Ack code=40403 或 40400 |
| 3 | `DB` | 余额/流水 | 无变动 |

**【数据清理】**
- 无。

---

## TC-GIFT-00005：SendGift - msg_id 幂等 + 并发不超卖
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 余额=100；G1 price=100。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 首次 SendGift msg_id="x" count=1 | code=0，余额=0 |
| 2 | `AppServer` | U1 3 秒内重发 msg_id="x" | 返回首次结果，余额仍=0（不再扣） |
| 3 | `AppServer` | 恢复余额=100；U1 20 QPS 并发 SendGift count=1（不同 msg_id） | 恰好 1 次成功，其余失败 code=40290；余额最终=0；wallet_transactions 新增 1 对行（expense+income） |

**【数据清理】**
- 回滚。

---

## TC-GIFT-00006：SendGift - count 边界 (0/1/99/100)
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U1 余额充足。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | count=0 | code=40003 |
| 2 | `AppServer` | count=1 | code=0 |
| 3 | `AppServer` | count 合法最大档 (1314) | code=0（若在允许范围内），扣款=price*count |
| 4 | `AppServer` | count=9999（超限） | code=40003 |
| 5 | `AppServer` | count=-1 | code=40003 |

**【数据清理】**
- 回滚。

---

## TC-GIFT-00007：Admin 礼物 CRUD + 软删 + 审计
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. OP_TOKEN 具备 GiftManage 权限。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | POST `/api/v1/admin/gifts` Body `{"name":"玫瑰","price":10,"tier":"L1","icon_url":"https://cdn/r.png","is_active":true}` | 201，data.id 为 UUID |
| 2 | `AdminServer` | POST Body price=0 | 400（price>=1） |
| 3 | `AdminServer` | POST Body icon_url=`https://cdn/r.exe` | 400（文件类型非白名单） |
| 4 | `AdminServer` | PUT `/api/v1/admin/gifts/{id}` Body `{"is_active":false}` | 200 |
| 5 | `DB` | gifts.is_active=false | 符合 |
| 6 | `AdminServer` | DELETE `/api/v1/admin/gifts/{id}` | 200；gifts.is_deleted=true（软删） |
| 7 | `DB` | admin_logs | 每次均新增对应 action |
| 8 | `AppServer` | C 端 GET gifts/list | 已删除的礼物不再出现 |

**【数据清理】**
- 硬删测试数据或留档。
