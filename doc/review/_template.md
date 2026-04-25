# 全局代码审查报告: [批次名称/模块名称]
> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [0/10]

---

## 0. 流转规则
- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由[GlobalReview]进行全局代码审查
- [GlobalReview]审查通过，则修改负责人 [-] 状态 [✅ Passed]
- [GlobalReview]审查未通过，则修改负责人 [TDD] 状态 [❌ Failed], 并将审查意见填入文档下方
- 处于负责人 [TDD] 状态 [❌ Failed]，则由[TDD]根据审查意见进行代码修复并自测
- [TDD]修复之后，将状态改为负责人 [GlobalReview] 状态 [⏳ In Review]

---

## 1. 审查上下文
- **包含任务**：[例如: - [模块 1: 用户认证系统 (User Authentication)](../tasks/模块1-用户认证系统%20(User%20Authentication).md), T-00007, T-00008]
- **关联 TDS**：[例如: [T-00001](../tds/server/T-00001.md)]
- **开始时间**：YYYY-MM-DD

---

## 2. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】
**@GlobalReview 审查意见：**
- [ ] **缺陷 1**：[级别 P0/P1/P2] [描述问题与涉及文件] 
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]
- [ ] **缺陷 2**：...

---