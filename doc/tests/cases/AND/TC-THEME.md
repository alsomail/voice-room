# 测试套件：THEME 黑金主题（Android）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-30018（MenaTheme）、T-30019（公共视觉组件）。

---

## TC-THEME-00001：MenaTheme 色值与 Typography
**【元数据】**
- **归属模块**：`THEME`
- **测试类型**：`Compatibility`
- **回归级别**：`P1`

**【前置条件】**
1. App 已升级到黑金主题版本；Debug 版本带有 ThemeInspector。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 启动 App | 所有页面背景色为 `#0F0F1E`（Background Primary 深黑紫），非白色 |
| 2 | `Android` | 观察主按钮（"获取验证码"、"创建"、"送出"） | 金色渐变填充，圆角 24dp，点击时水波纹为浅金 |
| 3 | `Android` | 打开开发者工具 ThemeInspector | 读取当前主色：`primary=#D4AF37`（金），`background=#0F0F1E`，`onBackground=#F5F5F7` |
| 4 | `Android` | 标题字号 | titleLarge=22sp，粗体；bodyMedium=16sp |

**【数据清理】**
- 无。

---

## TC-THEME-00002：GoldButton / GoldOutlinedTextField / AvatarWithFrame
**【元数据】**
- **归属模块**：`THEME`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 打开 Design System Demo 页面（Debug 菜单入口）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 观察 GoldButton 区块 | 启用态：金色渐变；禁用态：灰度 40% 透明；Loading：中央金色圆环 |
| 2 | `Android` | 观察 GoldOutlinedTextField | 未聚焦：淡金边框；聚焦：金色 2dp 边框；错误：红色边框 + 底部红字 |
| 3 | `Android` | 观察 AvatarWithFrame size=80dp | 头像外侧一圈 2dp 金色环，背景深色中清晰可辨 |
| 4 | `Android` | size 参数 40/64/80/120 | 均等比缩放，金环宽度按比例调整 |

**【数据清理】**
- 无。

---

## TC-THEME-00003：RTL 阿语下主题自动镜像
**【元数据】**
- **归属模块**：`THEME`
- **测试类型**：`Compatibility`
- **回归级别**：`P1`

**【前置条件】**
1. 系统语言切换到阿语。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 任意含返回按钮的页面 | 返回箭头位于右上角（镜像） |
| 2 | `Android` | 图标带方向（如"去充值 >"） | 箭头图标自动翻转为 `<` |
| 3 | `Android` | 文字段落 | 阿语文字从右至左对齐 |
| 4 | `Android` | 数字（金币数量） | 保持从左至右显示 |

**【数据清理】**
- 无。
