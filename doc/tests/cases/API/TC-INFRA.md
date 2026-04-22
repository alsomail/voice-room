# 测试套件：INFRA 工程基建

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-0000A（Docker Compose）、T-0000B（shared crate）、T-0000C（DB 权限隔离）、T-0000D（CI 流水线）。

---

## TC-INFRA-00001：Docker Compose 一键启动 PG + Redis 并持久化数据
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 本机已安装 Docker Desktop 且守护进程已启动。
2. 仓库根目录存在 `docker-compose.yml`。
3. 本机 5432 与 6379 端口空闲。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 在仓库根目录执行 `docker compose up -d` | 进程退出码为 0，终端输出包含 `Started postgres` 和 `Started redis` |
| 2 | `Shell` | 执行 `docker compose ps --format json` | postgres 与 redis 两个服务 State 字段均为 `running` 或 `Up` |
| 3 | `DB` | 执行 `psql -h 127.0.0.1 -p 5432 -U postgres -c "SELECT 1"` | 标准输出返回 `1`，退出码 0 |
| 4 | `DB` | 执行 `redis-cli -h 127.0.0.1 -p 6379 ping` | 标准输出返回 `PONG` |
| 5 | `DB` | 以 psql 执行 `INSERT INTO users(phone) VALUES('+966500000999')` | INSERT 成功（rowcount=1） |
| 6 | `Shell` | 执行 `docker compose restart postgres`，等待服务就绪 | 容器重启成功，State=running |
| 7 | `DB` | 执行 `SELECT phone FROM users WHERE phone='+966500000999'` | 返回 1 行，数据未丢失（卷持久化） |

**【数据清理】**
- 执行 `DELETE FROM users WHERE phone='+966500000999'`。
- 如需重置：执行 `docker compose down -v`（删除卷）。

---

## TC-INFRA-00002：Docker Compose 启动失败 - 端口被占用时给出明确错误
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Integration`
- **回归级别**：`P2`

**【前置条件】**
1. 本机另一进程已占用 5432 端口（例如本地 PostgreSQL 服务）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `docker compose up postgres` | 退出码非 0，stderr 包含 `port is already allocated` 或 `bind: address already in use` |
| 2 | `Shell` | 执行 `docker compose ps` | postgres 服务 State 为 `Exited` 或不存在 |

**【数据清理】**
- 结束占用端口的进程。

---

## TC-INFRA-00003：shared crate 被 App Server 与 Admin Server 同时引用且整体编译通过
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 根目录 `Cargo.toml` workspace 成员已包含 `shared/`、`app/server`、`app/adminServer`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 在仓库根目录执行 `cargo build --workspace` | 退出码 0，stderr 不含 `error[` 行 |
| 2 | `Shell` | 执行 `cargo tree -p voice-room-server -i shared` | 输出包含 `shared v` 开头一行 |
| 3 | `Shell` | 执行 `cargo tree -p voice-room-admin-server -i shared` | 输出包含 `shared v` 开头一行 |

**【数据清理】**
- 无。

---

## TC-INFRA-00004：shared crate JWT 工具 - 编解码对称与边界错误
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. `shared/src/jwt.rs` 提供 `encode_token(sub, ttl_secs, iss, secret)` 与 `decode_token(token, iss, secret)`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 单元测试：`encode_token("U1", 60, "voiceroom", "k1")` 再 decode | decode 返回的 claims.sub 等于 `"U1"` |
| 2 | `AppServer` | 单元测试：用不同 secret `"k2"` decode 上条 token | 返回 `Err(InvalidSignature)` |
| 3 | `AppServer` | 单元测试：`encode_token("U1", -1, ...)` 后 decode | 返回 `Err(TokenExpired)` |
| 4 | `AppServer` | 单元测试：decode `""` / `"a.b"` / 随机 base64 | 返回 `Err(InvalidFormat)`，不 panic |
| 5 | `AppServer` | 单元测试：decode 指定 iss=`voiceroom`，但 token 的 iss=`voiceroom-admin` | 返回 `Err(InvalidIssuer)` |

**【数据清理】**
- 无。

---

## TC-INFRA-00005：shared crate bcrypt 密码哈希 - 正确性与随机盐
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. `shared/src/password.rs` 暴露 `hash_password` 与 `verify_password`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 单元测试：`hash_password("Passw0rd!")` 再 `verify_password("Passw0rd!", hash)` | 返回 true |
| 2 | `AppServer` | 单元测试：同一明文连续 hash 两次 | 两个 hash 字符串不相等（因随机 salt） |
| 3 | `AppServer` | 单元测试：`verify_password("wrong", hash)` | 返回 false |
| 4 | `AppServer` | 单元测试：hash 字符串形如 `$2b$12$...` | 匹配正则 `^\$2[aby]\$\d{2}\$` |

**【数据清理】**
- 无。

---

## TC-INFRA-00006：app_server_user 无权修改 admins 表（垂直越权防护）
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 已执行 `scripts/dev/init-db.sh` 创建 `app_server_user` 与 `admin_server_user`。
2. 数据库已应用迁移，存在 users/rooms/admins/admin_logs 表。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `DB` | 以 app_server_user 连接执行 `INSERT INTO users(phone) VALUES('+966500000998')` | 成功，rowcount=1 |
| 2 | `DB` | 以 app_server_user 连接执行 `INSERT INTO admins(username, password_hash) VALUES('x','y')` | 失败，SQLSTATE=42501（permission denied for table admins） |
| 3 | `DB` | 以 app_server_user 连接执行 `DROP TABLE users` | 失败，SQLSTATE=42501 |
| 4 | `DB` | 以 admin_server_user 连接执行 `SELECT count(*) FROM admins` | 返回整数，无权限错误 |
| 5 | `DB` | 以 admin_server_user 连接执行 `INSERT INTO admin_logs(admin_id, action, target_id) VALUES(gen_random_uuid(), 'test', gen_random_uuid())` | 成功 |

**【数据清理】**
- 删除步骤 1 插入的 users 记录；删除步骤 5 插入的 admin_logs 记录。

---

## TC-INFRA-00007：CI 流水线 - PR 触发 lint+test+build 绿色 / 有警告变红
**【元数据】**
- **归属模块**：`INFRA`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. `.github/workflows/ci.yml` 已配置，仓库 Actions 已启用。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 新建分支 `ci-smoke`，修改 README 后 push，通过 GitHub Web 开 PR | PR 页面 Checks 区域出现 workflow 任务进入 `In progress` |
| 2 | `Shell` | 等待 CI 完成 | 所有 Job 显示绿色 ✓，PR 底部显示 `All checks have passed` |
| 3 | `Shell` | 查看 `cargo clippy --workspace -- -D warnings` step 日志 | 退出码 0，无 `warning:` 行 |
| 4 | `Shell` | 查看 `cargo test --workspace` step 日志 | 日志包含 `test result: ok.` 且 `0 failed` |
| 5 | `Shell` | 查看 Web 端 `npm run lint` step 日志 | 退出码 0，无 error |
| 6 | `Shell` | 在分支加入一行未使用变量 `let _unused: i32 = 1;` 后 push | CI 变红，clippy step 失败，PR 显示 `Some checks were not successful` |

**【数据清理】**
- 关闭并删除 `ci-smoke` 分支及对应 PR。
