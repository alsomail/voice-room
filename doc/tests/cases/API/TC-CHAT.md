# 测试套件：CHAT 公屏聊天（API）

> **需求模糊点 (Ambiguity Notes)**：
> - 敏感词库由外部服务（或本地列表）维护；测试以固定样本词 `fuck` / `政治敏感A` 为例。

覆盖 Task：T-00015（公屏聊天广播、敏感词过滤、msg_id 去重）。

---

## TC-CHAT-00001：SendMessage - 正常发送广播
**【元数据】**
- **归属模块**：`CHAT`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. R1 内 U1/U2/U3 活跃。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 发 `{"type":"SendMessage","payload":{"content":"hello"},"msg_id":"c1"}` | 收到 Ack code=0 |
| 2 | `AppServer` | U2/U3 | 收到 `{"type":"ChatMessage","payload":{"user_id":"{U1}","content":"hello","ts":...}}` |
| 3 | `AppServer` | U1 自身 | 也收到 ChatMessage（或通过 Ack 内携带等效字段，视实现） |

**【数据清理】**
- 无。

---

## TC-CHAT-00002：SendMessage - 内容长度边界值 (0/1/500/501)
**【元数据】**
- **归属模块**：`CHAT`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U1 在房间内。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | content=`""` | Ack code=40003 |
| 2 | `AppServer` | content=`"a"` | 成功广播 |
| 3 | `AppServer` | content=500 字符 | 成功广播 |
| 4 | `AppServer` | content=501 字符 | Ack code=40003 |

**【数据清理】**
- 无。

---

## TC-CHAT-00003：敏感词过滤 / XSS 尝试
**【元数据】**
- **归属模块**：`CHAT`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 敏感词库含 `fuck`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 发 content=`"go fuck you"` | Ack code=40003 或广播后 content 被替换为 `go **** you` |
| 2 | `AppServer` | U1 发 content=`"<script>alert(1)</script>"` | 广播原始字符串（Server 不额外转义），但字段值完整包含在 JSON 中，未造成 Server 异常（XSS 由客户端渲染层负责） |
| 3 | `AppServer` | U1 发 content=`"'; DROP TABLE users;--"` | 广播成功，DB 无异常；users 表仍存在 |

**【数据清理】**
- 无。

---

## TC-CHAT-00004：CHAT_MUTED 禁言用户无法发言
**【元数据】**
- **归属模块**：`CHAT`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. U2 被房主 MuteUser（包含 chat_muted=true）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发 SendMessage | Ack code=40305（CHAT_MUTED） |
| 2 | `AppServer` | 其他成员 | 未收到 ChatMessage |

**【数据清理】**
- 解除 mute。

---

## TC-CHAT-00005：msg_id 去重 - 重复发送同一 msg_id
**【元数据】**
- **归属模块**：`CHAT`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 在房间。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 发 SendMessage content="hi" msg_id="dup" | 广播 1 次 |
| 2 | `AppServer` | U1 3 秒内重发 msg_id="dup" content="hi" | 服务端去重，返回上次 Ack，不产生新 ChatMessage 广播 |
| 3 | `AppServer` | U1 使用新 msg_id="dup2" content="hi" | 正常广播（内容相同但 msg_id 不同不去重） |

**【数据清理】**
- 无。
