# 测试套件：MIC 麦位（Android）

> **需求模糊点 (Ambiguity Notes)**：
> - MIC-05 中"麦位已满"的边界错误码未在协议文档 `TakeMicResult.schema.json` 中明确定义；暂以返回 `code ≠ 0` 且 `message` 包含"满"或"full"等语义为断言，具体错误码以实现为准。
> - MIC-03/MIC-04 中管理员强制上麦/下麦的触发路径（HTTP 接口 vs WS 信令 `ForceTakeMic`/`ForceLeaveMic`）文档标注为 WS 信令；若切换为 HTTP 触发需同步更新对应步骤。

覆盖 Task：T-30012（上下麦 UI）、T-30014（麦克风权限 + RTC 集成骨架）、T-00104（跨语言 E2E），字段名已修正为协议冻结后的 snake_case 规范（`mic_index`，非 `slot`/`slot_index`）。

---

## TC-MIC-00001：权限申请 - 拒绝后 Fallback 到系统设置
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. U1 全新安装 App，未授予 RECORD_AUDIO 权限。
2. 已进入 RoomScreen。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击底栏麦克风按钮（🎤） | 弹出系统权限对话框"允许语聊房使用录音？" |
| 2 | `Android` | 点击"拒绝" | App 内 SnackBar 显示"需要麦克风权限才能上麦"，按钮"去设置" |
| 3 | `Android` | 点击"去设置" | 跳转到系统设置 App 当前应用的权限页 |
| 4 | `Android` | 在设置中授予麦克风权限，返回 App | 回到 RoomScreen |
| 5 | `Android` | 再次点击麦克风按钮 | 不再弹权限框，直接进入上麦流程 |

**【数据清理】**
- 无。

---

## TC-MIC-00002：上麦 → RTC publish → 下麦 E2E
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 授予麦克风权限，在 R1 内。麦位 3 空闲。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击麦位 3 的空座位"+" | 弹出确认菜单，选择"上麦" |
| 2 | `AppServer` | WS 收到 `TakeMic slot_index=3` | 返回 Ack code=0 |
| 3 | `Android` | 麦位 3 UI | 显示 U1 头像，头像下方出现金色麦克风图标 |
| 4 | `Android` | RTC SDK 日志（logcat filter RtcAdapter） | 输出 `publish audio stream success` |
| 5 | `Android` | 长按底栏"静音"图标 | 切换为"取消静音" |
| 6 | `Android` | 点击底栏"下麦" | 弹出确认 Dialog，点击"确认下麦" |
| 7 | `AppServer` | WS 收到 `LeaveMic` | 广播 MicLeft |
| 8 | `Android` | 麦位 3 UI | 恢复为空状态 `+` 按钮 |
| 9 | `Android` | RTC 日志 | 输出 `unpublish audio success` |

**【数据清理】**
- 无。

---

## TC-MIC-00003：麦位被占 - 错误提示
**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 麦位 3 已被 U2 占用。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击麦位 3（含 U2 头像） | 弹出 U2 资料卡（非可上麦入口） |
| 2 | `Android` | 从房主账号强制让 U1 上麦位 3（通过菜单） | SnackBar 提示"该麦位已被占用" |

**【数据清理】**
- 无。

---

## TC-MIC-00004：上麦完整链路，字段断言 MicTaken.payload.mic_index / user_id / forced_by=null

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile，临时 PG + Redis）。
2. U1（`E2E_VALID_TOKEN`）已加入房间 `E2E_ROOM_ID`，WS 连接有效，已授予 RECORD_AUDIO 权限。
3. 旁观者 U2（`E2E_VALID_TOKEN_B`）已加入同一房间，WS 连接有效（用于接收 `MicTaken` 广播）。
4. 麦位 0（`seat_index=0`）当前为空（`SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=0` 返回空）。
5. Schema 引用：`doc/protocol/schemas/ws/TakeMic.schema.json`、`doc/protocol/schemas/ws/TakeMicResult.schema.json`、`doc/protocol/schemas/ws/MicTaken.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                  | 预期结果 (Assertion)                                                                                                                                                                                                                                                   |
| :------: | :---------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | 点击 `mic_seat_0`（麦位 0 空座位图标"+"）                                                                                                                          | 弹出上麦确认菜单，显示"上麦"选项                                                                                                                                                                                                                                       |
|    2     | `Android`   | 点击"上麦"选项                                                                                                                                                     | 按钮短暂 Loading 状态                                                                                                                                                                                                                                                  |
|    3     | `AppServer` | 检查 AppServer WS 日志，确认收到 `TakeMic` 帧                                                                                                                      | 收到帧字段：`type="TakeMic"`；`payload.mic_index=0`（integer，snake_case，非 `micIndex` / `slot` / `slot_index`）；`msg_id` 为 UUID v4 格式；帧通过 AJV 校验 `TakeMic.schema.json`                                                                                    |
|    4     | `AppServer` | 等待 U1 接收 `TakeMicResult`（超时 3s）                                                                                                                            | `type="TakeMicResult"`，`code=0`，`payload.mic_index=0`                                                                                                                                                                                                               |
|    5     | `AppServer` | U2 等待接收 `MicTaken` 广播（超时 3s）                                                                                                                             | 字段断言（对照 `MicTaken.schema.json`）：`type="MicTaken"`；`payload.mic_index=0`（类型 integer，值 0）；`payload.user_id` = U1 的 UUID（snake_case，非 `userId`）；`payload.forced_by` 为 `null` **或** 字段缺省（非强制抱麦场景）；`timestamp` 整数 > 1,000,000,000,000 |
|    6     | `AppServer` | 对步骤 5 的 `MicTaken` 帧执行 AJV Schema 全量校验                                                                                                                  | AJV 校验 0 errors；`additionalProperties: false` 约束成立（无多余字段）                                                                                                                                                                                                |
|    7     | `Android`   | 观察 `mic_seat_0` 视觉区域                                                                                                                                         | 麦位 0 从空状态变为已占用状态：显示 U1 头像，底部出现声音波纹或麦克风图标指示 RTC 推流                                                                                                                                                                                  |
|    8     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=0`                                                                               | 返回 U1 的 UUID                                                                                                                                                                                                                                                        |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=0"`
- 关闭 WS 连接。

---

## TC-MIC-00005：下麦完整链路，字段断言 MicLeft.payload.mic_index / user_id / forced=false

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. U1 已加入房间 `E2E_ROOM_ID`，WS 连接有效，且 U1 已占据麦位 1（`seat_index=1`，DB 确认）。
3. 旁观者 U2 已加入同一房间，WS 连接有效（用于接收 `MicLeft` 广播）。
4. Schema 引用：`doc/protocol/schemas/ws/LeaveMic.schema.json`、`doc/protocol/schemas/ws/LeaveMicResult.schema.json`、`doc/protocol/schemas/ws/MicLeft.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                    | 预期结果 (Assertion)                                                                                                                                                                                                                                             |
| :------: | :---------- | :--------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | 点击底部操作栏的"下麦"按钮（或长按 `mic_seat_1` 自身麦位选择"下麦"）                                                                                 | 弹出确认对话框，显示"确认下麦？"                                                                                                                                                                                                                                  |
|    2     | `Android`   | 点击对话框"确认"按钮                                                                                                                                  | 按钮短暂 Loading 状态                                                                                                                                                                                                                                            |
|    3     | `AppServer` | 检查 AppServer WS 日志，确认收到 `LeaveMic` 帧                                                                                                        | 收到帧字段：`type="LeaveMic"`；`payload.mic_index=1`（integer，snake_case）；`msg_id` 为 UUID v4；帧通过 AJV 校验 `LeaveMic.schema.json`                                                                                                                          |
|    4     | `AppServer` | U1 接收 `LeaveMicResult`（超时 3s）                                                                                                                   | `type="LeaveMicResult"`，`code=0`                                                                                                                                                                                                                                |
|    5     | `AppServer` | U2 接收 `MicLeft` 广播（超时 3s）                                                                                                                     | 字段断言（对照 `MicLeft.schema.json`）：`type="MicLeft"`；`payload.mic_index=1`（类型 integer，值 1）；`payload.user_id` = U1 的 UUID（snake_case）；`payload.forced=false`（boolean，主动下麦）；`payload.forced_by` 为 `null` 或字段缺省；`timestamp` 整数 > 1,000,000,000,000 |
|    6     | `AppServer` | 对步骤 5 的 `MicLeft` 帧执行 AJV Schema 全量校验                                                                                                      | AJV 校验 0 errors                                                                                                                                                                                                                                                |
|    7     | `Android`   | 观察 `mic_seat_1` 视觉区域                                                                                                                             | 麦位 1 从 U1 头像恢复为空状态"+"图标，声音波纹消失                                                                                                                                                                                                               |
|    8     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=1`                                                                   | 返回空（user_id IS NULL）                                                                                                                                                                                                                                        |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=1"` （若用例失败未自动清理）
- 关闭 WS 连接。

---

## TC-MIC-00006：管理员强制上麦，MicTaken.forced_by=adminUserId

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. U1（普通用户）和管理员 ADMIN（`E2E_ADMIN_TOKEN`，角色 `owner` 或 `admin`）均已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. 旁观者 U2 已加入同一房间（用于接收广播）。
4. 麦位 2（`seat_index=2`）当前为空。
5. Schema 引用：`doc/protocol/schemas/ws/ForceTakeMic.schema.json`、`doc/protocol/schemas/ws/ForceTakeMicResult.schema.json`、`doc/protocol/schemas/ws/MicTaken.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                                | 预期结果 (Assertion)                                                                                                                                                                                                                          |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | 管理员账号点击 U1 的头像（或用户列表中 U1 条目）                                                                                                                                  | 弹出 U1 操作菜单                                                                                                                                                                                                                              |
|    2     | `Android`   | 点击菜单中"强制上麦"选项，在弹窗中选择麦位 2                                                                                                                                      | 弹窗显示麦位选择界面，选择麦位 2 后点击"确认"                                                                                                                                                                                                 |
|    3     | `AppServer` | 检查 AppServer WS 日志，确认收到 `ForceTakeMic` 帧                                                                                                                               | 收到帧字段：`type="ForceTakeMic"`；`payload.target_user_id` = U1 的 UUID；`payload.mic_index=2`（snake_case）；`msg_id` 为 UUID v4；帧通过 AJV 校验 `ForceTakeMic.schema.json`                                                               |
|    4     | `AppServer` | 管理员接收 `ForceTakeMicResult`（超时 3s）                                                                                                                                        | `type="ForceTakeMicResult"`，`code=0`                                                                                                                                                                                                         |
|    5     | `AppServer` | U2 接收 `MicTaken` 广播（超时 3s）                                                                                                                                                | 字段断言（对照 `MicTaken.schema.json`）：`type="MicTaken"`；`payload.mic_index=2`（integer）；`payload.user_id` = U1 的 UUID；`payload.forced_by` = **ADMIN 的 UUID**（非 `null`，非缺省，具体为管理员 UUID 字符串）；`timestamp` 整数 > 1,000,000,000,000 |
|    6     | `AppServer` | 对步骤 5 的 `MicTaken` 帧执行 AJV Schema 全量校验                                                                                                                                 | AJV 校验 0 errors；`forced_by` 字段符合 `format: uuid` 约束                                                                                                                                                                                   |
|    7     | `Android`   | 观察 `mic_seat_2` 视觉区域（U1 的设备和 U2 的设备）                                                                                                                               | 麦位 2 显示 U1 头像；管理员设备上显示"已将 U1 抱上麦位 2"或类似 Toast                                                                                                                                                                         |
|    8     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=2`                                                                                              | 返回 U1 的 UUID                                                                                                                                                                                                                               |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=2"`
- 关闭 WS 连接。

---

## TC-MIC-00007：管理员强制下麦，MicLeft.forced=true

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. U1 已加入房间 `E2E_ROOM_ID` 并占据麦位 3（`seat_index=3`，DB 确认）。
3. 管理员 ADMIN（`E2E_ADMIN_TOKEN`）已加入同一房间，角色为 `owner` 或 `admin`。
4. 旁观者 U2 已加入同一房间（用于接收广播）。
5. Schema 引用：`doc/protocol/schemas/ws/ForceLeaveMic.schema.json`、`doc/protocol/schemas/ws/ForceLeaveMicResult.schema.json`、`doc/protocol/schemas/ws/MicLeft.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                  | 预期结果 (Assertion)                                                                                                                                                                                                                                              |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | 管理员账号点击 `mic_seat_3`（U1 占据的麦位头像）                                                                                                   | 弹出麦位操作菜单，显示"强制下麦"选项                                                                                                                                                                                                                              |
|    2     | `Android`   | 点击"强制下麦"选项                                                                                                                                  | 弹出确认对话框                                                                                                                                                                                                                                                    |
|    3     | `Android`   | 点击对话框"确认"按钮                                                                                                                                | 按钮 Loading                                                                                                                                                                                                                                                      |
|    4     | `AppServer` | 检查 AppServer WS 日志，确认收到 `ForceLeaveMic` 帧                                                                                                | 收到帧字段：`type="ForceLeaveMic"`；`payload.target_user_id` = U1 的 UUID；`payload.mic_index=3`（snake_case）；`msg_id` 为 UUID v4；帧通过 AJV 校验 `ForceLeaveMic.schema.json`                                                                                   |
|    5     | `AppServer` | 管理员接收 `ForceLeaveMicResult`（超时 3s）                                                                                                         | `type="ForceLeaveMicResult"`，`code=0`                                                                                                                                                                                                                            |
|    6     | `AppServer` | U2 接收 `MicLeft` 广播（超时 3s）                                                                                                                   | 字段断言（对照 `MicLeft.schema.json`）：`type="MicLeft"`；`payload.mic_index=3`（integer）；`payload.user_id` = U1 的 UUID；`payload.forced=true`（boolean，强制下麦）；`payload.forced_by` = ADMIN 的 UUID（非 `null`）；`timestamp` 整数 > 1,000,000,000,000     |
|    7     | `AppServer` | 对步骤 6 的 `MicLeft` 帧执行 AJV Schema 全量校验                                                                                                    | AJV 校验 0 errors；`forced` 字段类型为 boolean；`forced_by` 字段符合 `format: uuid` 约束                                                                                                                                                                          |
|    8     | `Android`   | 观察 `mic_seat_3` 视觉区域（U1 设备视角和 U2 设备视角）                                                                                             | 麦位 3 从 U1 头像恢复为空状态"+"图标；U1 设备上显示 Toast 或 SnackBar"您已被管理员下麦"或类似提示                                                                                                                                                                  |
|    9     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=3`                                                                 | 返回空（user_id IS NULL）                                                                                                                                                                                                                                         |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=3"` （若用例失败未自动清理）
- 关闭 WS 连接。

---

## TC-MIC-00009：点击自己已占麦位图标 → 触发下麦（onMicSlotClick 调用链）

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Integration`
- **回归级别**：`P0`

> **关联 Bug**：T-30055（BUG-MIC-ONCLICK）——RoomScreen/AppNavGraph 未传递 `onMicSlotClick` 回调，导致点击自己麦位无任何反应。

**【前置条件】**
1. AppServer 已启动（test profile）。
2. U1（`E2E_VALID_TOKEN`）已加入房间 `E2E_ROOM_ID`，WS 连接有效，已授予 RECORD_AUDIO 权限。
3. U1 **已占据麦位 0**（`seat_index=0`，DB 确认：`SELECT user_id FROM mic_seats WHERE seat_index=0` 返回 U1 的 UUID）。
4. 旁观者 U2（`E2E_VALID_TOKEN_B`）已加入同一房间，WS 连接有效（用于接收 `MicLeft` 广播）。
5. Schema 引用：`doc/protocol/schemas/ws/LeaveMic.schema.json`、`doc/protocol/schemas/ws/LeaveMicResult.schema.json`、`doc/protocol/schemas/ws/MicLeft.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                      | 预期结果 (Assertion)                                                                                                                                               |
| :------: | :---------- | :----------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `Android`   | U1 设备上观察麦位 0（`mic_seat_0`）视觉状态                                                            | 显示 U1 的头像，非空座"+"图标（U1 已在此麦位）                                                                                                                     |
|    2     | `Android`   | U1 **单击** `mic_seat_0`（自己当前所在的麦位图标）                                                     | 弹出下麦确认菜单或 Dialog，显示"下麦"/"离开麦位"选项（**不应无反应，不应弹权限框**）                                                                               |
|    3     | `Android`   | 点击"下麦"/"确认"按钮                                                                                   | 按钮短暂 Loading 状态                                                                                                                                              |
|    4     | `AppServer` | 检查 AppServer WS 日志，确认收到 `LeaveMic` 帧（超时 3s）                                              | 收到帧：`type="LeaveMic"`；`payload.mic_index=0`（integer，snake_case）；`msg_id` 为 UUID v4；帧通过 AJV 校验 `LeaveMic.schema.json`                               |
|    5     | `AppServer` | U1 接收 `LeaveMicResult`（超时 3s）                                                                    | `type="LeaveMicResult"`，`code=0`                                                                                                                                  |
|    6     | `AppServer` | U2 接收 `MicLeft` 广播（超时 3s）                                                                      | `type="MicLeft"`；`payload.mic_index=0`；`payload.user_id` = U1 UUID；`payload.forced=false`；`timestamp` > 1,000,000,000,000；AJV 校验 0 errors                    |
|    7     | `Android`   | 观察 `mic_seat_0` 视觉状态                                                                              | 麦位 0 恢复为空状态"+"图标，U1 头像消失                                                                                                                            |
|    8     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=0`                   | 返回空（user_id IS NULL）                                                                                                                                          |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>' AND seat_index=0"` （若用例失败未自动清理）
- 关闭 WS 连接。

**【元数据】**
- **归属模块**：`MIC`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. AppServer 已启动（test profile）。
2. 用户 U1（`E2E_VALID_TOKEN`）已加入房间 `E2E_ROOM_ID`，WS 连接有效。
3. 房间 `E2E_ROOM_ID` 的所有 9 个麦位（`seat_index` 0-8）**全部被占满**：
   - `SELECT count(*) FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND user_id IS NOT NULL` 返回 `9`。
   - （测试前通过 DB seed 填充 8 个占位用户 + 1 个现有用户，确保无空余麦位）
4. U1 当前**不在任何麦位上**（U1 的 UUID 不在上述 9 个用户中）。
5. Schema 引用：`doc/protocol/schemas/ws/TakeMic.schema.json`、`doc/protocol/schemas/ws/TakeMicResult.schema.json`。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                   | 预期结果 (Assertion)                                                                                                                                                                  |
| :------: | :---------- | :-------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
|    1     | `Android`   | 观察麦位区域视觉（9 个麦位均显示已占用头像）                                                                                                         | 所有 9 个麦位均显示用户头像，无空余"+"图标                                                                                                                                            |
|    2     | `AppServer` | 通过测试 WS 客户端以 U1 身份发送 `TakeMic` 帧，尝试占据已满的麦位（选取任一已满麦位，如 `mic_index=0`）：`{"type":"TakeMic","msg_id":"<uuid-v4>","payload":{"mic_index":0}}` | 帧发出成功（即便 Android UI 不显示入口，直接通过 WS 注入）                                                                                                                            |
|    3     | `AppServer` | U1 接收 `TakeMicResult`（超时 3s）                                                                                                                   | `type="TakeMicResult"`；`code` **≠ 0**（错误码，具体值以实现为准，建议 `40900` 冲突错误或专属麦位错误码）；`message` 字段包含"occupied"、"full"、"已占用" 等语义关键词之一；无 `payload.mic_index` 字段（或有但不代表成功） |
|    4     | `AppServer` | 对步骤 3 的 `TakeMicResult` 帧执行 AJV Schema 校验，引用 `TakeMicResult.schema.json`                                                                | AJV 校验通过（错误响应也应符合 schema 格式）                                                                                                                                          |
|    5     | `DB`        | 执行 `SELECT user_id FROM mic_seats WHERE room_id='<E2E_ROOM_ID>' AND seat_index=0`                                                                  | 返回**原占座用户 UUID**（非 U1，U1 上麦请求被拒绝，DB 状态未变化）                                                                                                                   |

**【数据清理】**
- `psql -c "UPDATE mic_seats SET user_id=NULL WHERE room_id='<E2E_ROOM_ID>'"` （清空所有 seed 麦位数据）
- 关闭 WS 连接。
