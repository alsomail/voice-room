# 测试套件：GIFT 礼物管理（Web）

> **需求模糊点 (Ambiguity Notes)**：
> - 图片上传大小 ≤500KB，格式 png/jpg/webp（常识值，若 TDS 不同以 TDS 为准）。

覆盖 Task：T-20013（礼物管理页 CRUD）。

---

## TC-GIFT-00008：礼物管理 - 列表 + 筛选
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. gifts 表 10 个礼物（8 active + 2 inactive）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/gifts` | Table 渲染，列：图标 / 名称 / 价格 / 等级 / 状态 / 操作 |
| 2 | `AdminWeb` | 图标列 | 显示 48x48 图片缩略图 |
| 3 | `AdminWeb` | 筛选"等级"选 L3 | 列表仅显示 L3 礼物 |
| 4 | `AdminWeb` | 筛选"状态"选"已下架" | 显示 2 个 inactive 礼物 |
| 5 | `AdminWeb` | 右上角"+ 新增礼物"按钮 | 仅 operator+ 角色可见，CS 不可见 |

**【数据清理】**
- 无。

---

## TC-GIFT-00009：新增礼物 - 图片上传白名单 + CRUD
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. operator 登录。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击"+ 新增礼物" | 弹出 GiftFormModal，含名称/价格/等级/图片上传/描述 |
| 2 | `AdminWeb` | 上传 `evil.exe` | 前端校验失败，显示"只支持 png/jpg/webp" |
| 3 | `AdminWeb` | 上传 600KB 的 png | 校验失败"文件大小超过 500KB" |
| 4 | `AdminWeb` | 上传合法 100KB png | 预览区显示缩略图 |
| 5 | `AdminWeb` | 名称="玫瑰" 价格=0 | 价格字段下方红字"价格必须 ≥1" |
| 6 | `AdminWeb` | 价格=10 等级=L1 → 提交 | 成功；列表新增行 |
| 7 | `AdminWeb` | 编辑该礼物 → 价格改为 20 → 保存 | 列表价格更新为 20 |
| 8 | `AdminWeb` | 点击"下架"开关 | 状态切换为 inactive，DB gifts.is_active=false |
| 9 | `AdminWeb` | 点击"删除" → Modal.confirm → 确认 | DB gifts.is_deleted=true（软删），列表该行消失 |
| 10 | `AppServer` | C 端 GET /gifts/list | 已删除礼物不再返回 |

**【数据清理】**
- 清理测试礼物。
