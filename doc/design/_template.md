<!--
[AI 写入规约]
1. 本文件由 PM Agent 在阶段二（业务流程与 UI 设计定义）创建，作为前端/客户端 UI Task 的设计描述。
2. 文件命名：`doc/design/<端>/T-XXX.md`（端 = `android` | `adminWeb`）。
3. 创建后必须在 doc/tasks/index.md 对应 Task 的「UI设计文档」列填入相对链接。
4. 实现端（TDD Agent）只读本文件，禁止修改；如需调整设计走 PM 阶段回炉。
-->

# UI 设计：[页面/组件名称] (Task ID: T-xxx，端：android | adminWeb)

## 一、线框与版式（Wireframe）
（用文字 / ASCII 图 / 引用外链图片描述布局结构。中东市场必须明确 RTL 镜像规则。）

- 屏幕/页面：xxx
- 视口断点（Web）/ 设备适配（Android）：xxx
- RTL 镜像：✅ 启用 / ❌ 不启用（说明原因）

## 二、组件清单（Component Inventory）
列出本设计涉及的所有 UI 组件，区分「新增组件」与「复用既有组件」。

| 组件名 | 类型 | 来源 | 备注 |
|--------|------|------|------|
| `MicSeatAvatar` | 新增通用组件 | 本 Task 引入 | 头像 + 麦位状态徽标 |
| `Button` (Primary) | 复用既有 | design system | - |

⚠️ **通用组件提取铁律**：本 Task 中任何重复出现 ≥ 2 次的视觉单元必须封装为独立组件，禁止局部内联。

## 三、交互状态（Interaction States）
逐组件列出全部状态，**禁止只画 happy-path**。

| 组件 | 状态 | 触发条件 | 视觉表现 | 关联状态机锚点 |
|------|------|---------|---------|---------------|
| `MicSeatAvatar` | Idle | 座位空 | 灰色占位 + "+"号 | [`state_machines.md#mic-seat`](../../product/state_machines.md#mic-seat) |
| `MicSeatAvatar` | Occupied | 有用户 | 真实头像 + 在线圈 | 同上 |
| `MicSeatAvatar` | Muted | 房主禁麦 | 头像 + 红色禁麦图标 | 同上 |
| `MicSeatAvatar` | Loading | 抢麦中 | 头像 + 转圈 | 同上 |
| `MicSeatAvatar` | Error | 抢麦失败 | toast + 抖动动画 | 同上 |

## 四、文案（Copy）
中东出海必须给出 `ar` / `en` 双语文案，禁止仅给中文。

| Key | 中文（仅参考） | English | العربية | 出现位置 |
|-----|---------------|---------|---------|---------|
| `mic_seat_join_cta` | 上麦 | Take Seat | اعتلاء المنصة | 麦位按钮 |
| `mic_seat_taken_toast` | 座位已被占用 | Seat already taken | المقعد محجوز بالفعل | 抢麦失败提示 |

## 五、自动化测试锚点（Test Anchors）
**强制要求**：每个可交互元素必须有稳定测试 ID，供 e2e-runner / unit 测试断言。

### Android（Compose / View）
```kotlin
Modifier.testTag("mic_seat_${seatIndex}")
Modifier.testTag("btn_take_seat")
```

### Web（React / Refine）
```tsx
<button data-testid="btn-take-seat" />
<div data-testid={`mic-seat-${seatIndex}`} />
```

### 命名规范
- Android：`snake_case`，前缀按类型 `btn_` / `txt_` / `img_` / `mic_seat_<n>`。
- Web：`kebab-case`，前缀同上 `btn-` / `txt-` / `img-`。
- **禁止**使用 i18n 文案、定位器选择 DOM（如 `text=上麦`），否则文案变更必碎测试。

## 六、关联约束
- 字数上限：引用 [`business_constraints.md#text-length`](../../product/business_constraints.md#text-length)
- 频率限制：引用 [`business_constraints.md#rate-limit`](../../product/business_constraints.md#rate-limit)

## 七、变更记录
| 版本 | 日期 | 变更摘要 |
|------|------|---------|
| v1.0 | YYYY-MM-DD | 初版 |
