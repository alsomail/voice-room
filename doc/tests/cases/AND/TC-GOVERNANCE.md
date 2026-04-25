# 测试套件：GOVERNANCE 房间治理与用户操作（Android）

> **需求模糊点 (Ambiguity Notes)**：
> - 公告首次自动弹窗的"24h 内不再自动弹"存储位置（DataStore Key 命名未明示），本套件按 `announcement_last_shown_{room_id}` 断言，不同实现请反馈。

覆盖 Task：T-30036（创建房间升级）、T-30037（封面选择器）、T-30038（密码房进房弹窗）、T-30039（观众席 BottomSheet）、T-30040（用户操作菜单动态权限）、T-30041（踢人原因弹窗）、T-30042（被踢/被禁提示）、T-30043（公告栏 + 管理员徽章）、T-30044（禁麦/禁言 UI 反馈 + 抱麦集成）。

---

## TC-GOVERNANCE-00001：创建房间升级表单 - 四字段联动校验
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 用户 U1 已登录并进入大厅。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击大厅 FAB `+` 进入 CreateRoomScreen | 显示金色"创建房间"标题、房名输入框、封面缩略图（默认第一张）、分类下拉、公告 TextField、密码 Switch（默认关闭） |
| 2 | `Android` | 不填房名，观察底部按钮 `Key('btn_submit_create_room')` | 按钮置灰（enabled=false） |
| 3 | `Android` | 房名输入"情感夜谈"，公告输入 201 字中文 | 公告计数 `201/200` 变红；提交按钮置灰 |
| 4 | `Android` | 公告改为 200 字；打开密码 Switch；密码输入 `1234` | 密码框有 6 个分格，已输入 4 个；提交按钮置灰 |
| 5 | `Android` | 密码补齐 `123456` | 提交按钮点亮金色 |
| 6 | `Android` | 点击提交 | Loading → 网络请求 POST /rooms → 成功后自动 JoinRoom 进入 RoomScreen |
| 7 | `AppServer` | DB 查该房间 | password_hash 非空，announcement、category 均已保存 |

**【数据清理】**
- 关闭本次创建的房间。

---

## TC-GOVERNANCE-00002：封面选择器 - 8 张预设 + 选中视觉 + RTL
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 位于 CreateRoomScreen。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击封面缩略图区域 | 弹起 CoverPickerBottomSheet，3 列网格 8 张图片 |
| 2 | `Android` | 断言 testTag | `Key('cover_option_0')` ~ `Key('cover_option_7')` 均可见 |
| 3 | `Android` | 选中第 4 张 `cover_option_3` | 其卡片边框出现 2dp 金色边框，其他取消边框 |
| 4 | `Android` | 点击"确认" | BottomSheet 关闭，CreateRoomScreen 封面预览更新为 cover_03 |
| 5 | `Android` | 切换系统语言为阿语（ar）重新进入 | 3 列网格镜像，金色边框仍在选中项，滚动方向 RTL 正常 |

**【数据清理】**
- 恢复系统语言。

---

## TC-GOVERNANCE-00003：密码房进房弹窗 - 6 位自动提交 + 错误剧本 + 锁定
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 大厅存在密码房 R1（has_password=true）。
2. 后端 Redis `pwd_fail:U1:R1` 已被预置为 `4`（再错一次即锁）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击 R1 卡片 | 弹出 PasswordInputDialog；6 分格输入框聚焦；testTag `Key('password_input')` |
| 2 | `Android` | 输入 `000000` | 满 6 位自动触发提交，无需点按钮 |
| 3 | `AppServer` | 返回 PASSWORD_LOCKED locked_sec=1800 | 弹窗显示红字"账号已锁定，30 分钟后重试"；`Key('btn_submit_password')` 置灰 |
| 4 | `Android` | 返回键关闭弹窗 | 未进入房间；大厅停留 |
| 5 | `Android` | 5 分钟后（模拟）重试正确 `666666` | 仍显示锁定提示（未过 30min） |
| 6 | `Android` | 手动清 Redis 锁 Key 后重试 `666666` | 成功进房，跳转 RoomScreen |

**【数据清理】**
- DEL pwd_fail/pwd_lock Key；退出房间。

---

## TC-GOVERNANCE-00004：观众席 BottomSheet - 分组 + 滚动 + 实时增删
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 有 100 人在线，麦上 4 人，房主 1 人，管理员 1 人，其余 95 人为观众。
2. U1 在房间内。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击顶部成员计数器（如"观众 100"） | 弹出 AudienceBottomSheet（占屏 70%） |
| 2 | `Android` | 断言分组标头 | 可见"麦上 (4)"与"观众 (95)" |
| 3 | `Android` | 上下滑动列表 | 测试帧率 ≥50fps；`Key('audience_item_$userId')` 可定位 |
| 4 | `AppServer` | 外部用户 U_NEW 发 JoinRoom | 500ms 内"观众 (96)"计数更新，列表顶部出现新观众 |
| 5 | `AppServer` | 麦上用户 U3 发 LeaveMic | "麦上 (3)"计数更新；U3 移到"观众"分组 |
| 6 | `Android` | 房间内无人（除自己）场景 | 显示空状态插画 + 文案"房间暂时没有其他观众" |

**【数据清理】**
- 退出房间。

---

## TC-GOVERNANCE-00005：用户操作菜单 - 角色权限矩阵（9 组合）
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. R1 房主 O、管理员 A、普通成员 M。
2. 三个设备分别登录 O/A/M 进入 R1。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` (O 设备) | 观众席点 A 用户 | 菜单显示 [抱上麦/禁麦/禁言/踢出/卸任管理员/查看资料/举报] 7 项 |
| 2 | `Android` (O 设备) | 点 M 用户 | 菜单显示 [抱上麦/禁麦/禁言/踢出/任命管理员/查看资料/举报] 7 项 |
| 3 | `Android` (O 设备) | 点 O 自己 | 菜单仅显示 [查看资料] |
| 4 | `Android` (A 设备) | 点 O | 菜单仅 [查看资料/举报] |
| 5 | `Android` (A 设备) | 点 M | [抱上麦/禁麦/禁言/踢出/查看资料/举报]（无任命） |
| 6 | `Android` (A 设备) | 点 A 自己 | [查看资料] |
| 7 | `Android` (M 设备) | 点 O | [查看资料/举报] |
| 8 | `Android` (M 设备) | 点 A | [查看资料/举报] |
| 9 | `Android` (M 设备) | 点 M 另一个普通用户 | [查看资料/举报] |
| 10 | `Android` (O 设备) | 点"卸任管理员" | 弹二次确认 Dialog，"确定"后发 TransferAdmin(revoke) → 广播 AdminChanged → 该用户徽章消失 |

**【数据清理】**
- 恢复 R1 无管理员。

---

## TC-GOVERNANCE-00006：踢人原因弹窗 - 单选 + 其他必填 + JSON 安全
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. O 作为房主选中踢出 M（已通过用户操作菜单触发）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 弹出 KickReasonDialog | 4 个单选按钮（骚扰/刷屏/辱骂/其他），默认选中"骚扰"；`Key('kick_reason_0')`~`kick_reason_3` |
| 2 | `Android` | 点击外部空白 | 弹窗不关闭（dismissOnClickOutside=false） |
| 3 | `Android` | 选择"其他"但 `Key('kick_reason_custom_input')` 留空 | `Key('btn_confirm_kick')` 置灰 |
| 4 | `Android` | 输入"引入"双引号 + 反斜杠：`"恶意\"广告\\链接"` | 无崩溃；点击确定 |
| 5 | `AppServer` | 后端收到 KickUser WS 消息 | reason 字段 JSON 合法，特殊字符已转义（双引号→全角或 `\"`） |
| 6 | `Android` | 并发点击"确定"3 次 | 仅 1 次 WS 请求（isSubmitting 防抖） |
| 7 | `Android` | 成功后 | 弹窗自动关闭，Toast "已踢出"；M 从观众席消失 |

**【数据清理】**
- 无。

---

## TC-GOVERNANCE-00007：被踢/被禁弹窗 + 倒计时 Chip
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U_TARGET 在 R1 内。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 房主踢出 U_TARGET，reason="骚扰" | U_TARGET 收 UserKicked，客户端弹全屏 Dialog `Key('dialog_kicked')` 显示"你已被移出房间，原因：骚扰，10 分钟后可再次进入" |
| 2 | `Android` | 点"知道了" | 导航回大厅；R1 卡片"进入"按钮灰色倒计时 600s |
| 3 | `Android` | 600s 内点该按钮 | Toast "冷却中 xx 秒"，不发起请求 |
| 4 | `Android` | 模拟时间快进 600s 后点击 | 正常进房请求（仍会被后端 KICKED_COOLDOWN 拦截，以 Toast 提示） |
| 5 | `AppServer` | 房主禁麦 U_TARGET 300s | U_TARGET Toast "你已被禁麦 5 分钟"；底部 Chip `Key('mute_countdown')` 显示 5:00 倒计时 |
| 6 | `AppServer` | 禁麦期内房主再禁言 60s | Chip 切换为最新（禁言 1:00），旧禁麦倒计时被覆盖 |
| 7 | `AppServer` | 房主 UnmuteUser | Chip 500ms 内消失 |

**【数据清理】**
- 无。

---

## TC-GOVERNANCE-00008：禁麦/禁言 UI 反馈 + 抱麦集成
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. U1 在 R1 中，未授权麦克风；U1 被设 mic_muted；UI 有空闲麦位 slot=3。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | U1 点击空麦位 "+" | 麦位 "+" 呈灰色禁用态；点击无网络请求；底部 Toast "你已被禁麦" |
| 2 | `AppServer` | 房主 ForceTakeMic target=U1 slot=3 | U1 客户端弹出麦克风权限系统对话框 |
| 3 | `Android` | 用户拒绝权限 | 客户端自动发 MicLeave；麦位 slot=3 保持空；Toast "需要麦克风权限" |
| 4 | `AppServer` | 下一次 ForceTakeMic 目标改为已授权用户 U5 | U5 自动上麦推流，广播 MicTaken |
| 5 | `AppServer` | 房主 ForceLeaveMic target=U5 | U5 RTC 推流停止；本地麦元状态变离线；Toast "你已被抱下麦" |
| 6 | `AppServer` | 房主设 chat_muted U1 | U1 聊天输入框 `Key('chat_input')` disabled，占位文本"你已被禁言 N 分钟" |
| 7 | `Android` | U1 尝试长按输入框粘贴 | 无法粘贴，提交按钮置灰 |

**【数据清理】**
- UnmuteUser 解除状态。

---

## TC-GOVERNANCE-00009：公告栏 + 管理员徽章 + 实时同步
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. R1 公告="欢迎新朋友"，无管理员；U1 首次进入。
2. DataStore 清空 `announcement_last_shown_R1`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | U1 JoinRoom R1 | 500ms 内弹出 AnnouncementPopup `Key('announcement_popup')` 内容为"欢迎新朋友" |
| 2 | `Android` | 关闭弹窗 | DataStore 写入 `announcement_last_shown_R1 = now` |
| 3 | `Android` | 5 分钟后再次 JoinRoom R1 | 不再自动弹；房间顶部保留 📄 图标 `Key('btn_show_announcement')` |
| 4 | `Android` | 点该图标 | 弹出同一公告 |
| 5 | `AppServer` | 房主 PATCH 公告为"新规则" | WS 广播 RoomInfoUpdated；客户端顶部 Toast 提示"公告已更新"（可选）；再次点图标显示"新规则" |
| 6 | `AppServer` | 房主 TransferAdmin assign target=U2 | U2 昵称旁 500ms 内出现 🛡️ 金色盾牌徽章 |
| 7 | `AppServer` | TransferAdmin revoke target=U2 | 500ms 内徽章消失 |
| 8 | `Android` | 房主 U_OWNER 昵称旁 | 持续显示 👑 金色王冠徽章 |

**【数据清理】**
- 清 DataStore Key；恢复 R1 公告/管理员状态。
