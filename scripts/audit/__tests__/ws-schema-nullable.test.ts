/**
 * TDD 测试套件：WS Schema nullable 字段正确性验证
 * Task: T-00100 Review Round 1 P1-1
 *
 * RED → GREEN → REFACTOR
 *
 * 覆盖场景：
 *  - UserJoined.schema.json  avatar 必须接受 null（server Option<String>）
 *  - MicTaken.schema.json    avatar 必须接受 null（server MemberInfo.avatar Option<String>）
 *  - RoomMessage.schema.json avatar 必须接受 null（server MemberInfo.avatar Option<String>）
 *  - 所有三个 schema 仍接受合法 URI 字符串（非回归）
 *  - 所有三个 schema 仍拒绝非字符串非 null 类型（边界值）
 */

import { describe, test, expect } from '@jest/globals';
import * as fs from 'node:fs';
import * as path from 'node:path';
import Ajv2020 from 'ajv/dist/2020';
import addFormats from 'ajv-formats';

// ─── 路径常量 ────────────────────────────────────────────────────────────────
const REPO_ROOT = path.resolve(__dirname, '../../..');
const WS_SCHEMAS = path.join(REPO_ROOT, 'doc/protocol/schemas/ws');

// ─── 辅助：每次调用创建独立 AJV 实例，避免 $id 重复注册错误 ────────────────
function loadSchema(name: string) {
  const filePath = path.join(WS_SCHEMAS, `${name}.schema.json`);
  const raw = fs.readFileSync(filePath, 'utf8');
  const schema = JSON.parse(raw) as Record<string, unknown>;
  // 独立实例：每次编译互不干扰，避免 "schema with key already exists" 错误
  const ajv = new Ajv2020({ strict: false, allErrors: true });
  addFormats(ajv);
  return ajv.compile(schema);
}

// ─── 合法 UUID 常量 ───────────────────────────────────────────────────────────
const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_AVATAR = 'https://cdn.example.com/avatar.jpg';

// =============================================================================
// TC-NULLABLE-01: UserJoined — avatar 字段可空性
// =============================================================================
describe('TC-NULLABLE-01: UserJoined.schema.json — avatar 字段可空性', () => {
  // --- 正向：avatar = null（server Option<String> 序列化结果）---
  test('🔴→🟢 avatar=null 应通过 schema 校验（Option<String> 序列化为 null）', () => {
    const validate = loadSchema('UserJoined');
    const msg = {
      type: 'UserJoined',
      payload: {
        user_id: VALID_UUID,
        nickname: 'Alice',
        avatar: null,          // ← 这是 P1-1 的核心修复点
        member_count: 1,
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 正向：avatar = 合法 URI（非回归） ---
  test('avatar 为合法 URI 时仍通过 schema 校验', () => {
    const validate = loadSchema('UserJoined');
    const msg = {
      type: 'UserJoined',
      payload: {
        user_id: VALID_UUID,
        nickname: 'Bob',
        avatar: VALID_AVATAR,
        member_count: 2,
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 正向：avatar 字段缺失（not required 时无 avatar key）---
  // 注意：avatar 在 required 列表内，所以这个场景测试 required 约束
  test('avatar 缺失时 schema 应拒绝（avatar 在 required 中）', () => {
    const validate = loadSchema('UserJoined');
    const msg = {
      type: 'UserJoined',
      payload: {
        user_id: VALID_UUID,
        nickname: 'Charlie',
        // avatar 故意不传
        member_count: 0,
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    // avatar 在 payload.required 中，缺失应校验失败
    expect(valid).toBe(false);
  });

  // --- 负向：avatar = 数字（非法类型）---
  test('avatar=42（数字）应被 schema 拒绝', () => {
    const validate = loadSchema('UserJoined');
    const msg = {
      type: 'UserJoined',
      payload: {
        user_id: VALID_UUID,
        nickname: 'Dave',
        avatar: 42,             // ← 非法：不是 string 也不是 null
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });

  // --- 负向：avatar = false（布尔）---
  test('avatar=false（布尔）应被 schema 拒绝', () => {
    const validate = loadSchema('UserJoined');
    const msg = {
      type: 'UserJoined',
      payload: {
        user_id: VALID_UUID,
        nickname: 'Eve',
        avatar: false,          // ← 非法
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });

  // --- 边界：空字符串 avatar --- 
  test('avatar=""（空字符串）通过类型检查（format: uri 验证由实现决定）', () => {
    const validate = loadSchema('UserJoined');
    const msg = {
      type: 'UserJoined',
      payload: {
        user_id: VALID_UUID,
        nickname: 'Frank',
        avatar: '',             // 空字符串，类型合法，但可能不符合 uri format
      },
      timestamp: 1700000000000,
    };
    // 此处只验证类型层面（string | null），format 检查由 ajv strict 模式控制
    // 测试目标：schema 不抛异常，行为可预期
    expect(() => validate(msg)).not.toThrow();
  });
});

// =============================================================================
// TC-NULLABLE-02: MicTaken — avatar 字段可空性（server MemberInfo.avatar）
// =============================================================================
describe('TC-NULLABLE-02: MicTaken.schema.json — avatar 字段可空性', () => {
  // --- 正向：包含 avatar=null ---
  test('🔴→🟢 avatar=null 时 MicTaken 应通过 schema 校验', () => {
    const validate = loadSchema('MicTaken');
    const msg = {
      type: 'MicTaken',
      payload: {
        mic_index: 2,
        user_id: VALID_UUID,
        nickname: 'Alice',
        avatar: null,           // ← Option<String> 为 None 时的序列化
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 正向：avatar 为有效 URI（非回归）---
  test('avatar 为有效 URI 时 MicTaken 应通过 schema 校验', () => {
    const validate = loadSchema('MicTaken');
    const msg = {
      type: 'MicTaken',
      payload: {
        mic_index: 0,
        user_id: VALID_UUID,
        nickname: 'Bob',
        avatar: VALID_AVATAR,
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 正向：avatar 字段不存在（可选字段）---
  test('avatar 字段缺失时 MicTaken 应通过 schema 校验（avatar 非 required）', () => {
    const validate = loadSchema('MicTaken');
    const msg = {
      type: 'MicTaken',
      payload: {
        mic_index: 1,
        user_id: VALID_UUID,
        // avatar 不传 — server 当前实际广播就没有 avatar
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 负向：avatar = 数字---
  test('avatar=123 应被 MicTaken schema 拒绝', () => {
    const validate = loadSchema('MicTaken');
    const msg = {
      type: 'MicTaken',
      payload: {
        mic_index: 3,
        user_id: VALID_UUID,
        avatar: 123,            // ← 非法类型
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });

  // --- 边界：mic_index 超出范围（maximum: 8）---
  test('mic_index=9 应被 MicTaken schema 拒绝（maximum: 8）', () => {
    const validate = loadSchema('MicTaken');
    const msg = {
      type: 'MicTaken',
      payload: {
        mic_index: 9,           // ← 超出 maximum: 8
        user_id: VALID_UUID,
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });
});

// =============================================================================
// TC-NULLABLE-03: RoomMessage — avatar 字段可空性
// =============================================================================
describe('TC-NULLABLE-03: RoomMessage.schema.json — avatar 字段可空性', () => {
  // --- 正向：avatar=null ---
  test('🔴→🟢 avatar=null 时 RoomMessage 应通过 schema 校验', () => {
    const validate = loadSchema('RoomMessage');
    const msg = {
      type: 'RoomMessage',
      payload: {
        msg_id: VALID_UUID,
        user_id: VALID_UUID,
        nickname: 'Alice',
        avatar: null,           // ← Option<String> 为 None
        content: 'Hello!',
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 正向：avatar 为有效 URI（非回归）---
  test('avatar 为有效 URI 时 RoomMessage 应通过 schema 校验', () => {
    const validate = loadSchema('RoomMessage');
    const msg = {
      type: 'RoomMessage',
      payload: {
        msg_id: VALID_UUID,
        user_id: VALID_UUID,
        nickname: 'Bob',
        avatar: VALID_AVATAR,
        content: 'World!',
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 正向：avatar 字段不存在（可选字段 — server 当前不发 avatar）---
  test('avatar 字段缺失时 RoomMessage 应通过 schema 校验（可选字段）', () => {
    const validate = loadSchema('RoomMessage');
    const msg = {
      type: 'RoomMessage',
      payload: {
        msg_id: VALID_UUID,
        user_id: VALID_UUID,
        content: 'Hi everyone',
        // nickname/avatar 不传 — 当前 server 实际广播就没有这两个字段
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(true);
    expect(validate.errors).toBeNull();
  });

  // --- 负向：content 超过 maxLength: 500 ---
  test('content 超过 500 字符应被 RoomMessage schema 拒绝', () => {
    const validate = loadSchema('RoomMessage');
    const msg = {
      type: 'RoomMessage',
      payload: {
        msg_id: VALID_UUID,
        user_id: VALID_UUID,
        content: 'x'.repeat(501),  // ← 超出 maxLength: 500
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });

  // --- 负向：content 为空字符串（minLength: 1）---
  test('content="" 应被 RoomMessage schema 拒绝（minLength: 1）', () => {
    const validate = loadSchema('RoomMessage');
    const msg = {
      type: 'RoomMessage',
      payload: {
        msg_id: VALID_UUID,
        user_id: VALID_UUID,
        content: '',            // ← 违反 minLength: 1
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });

  // --- 负向：avatar = 数字---
  test('avatar=true（布尔）应被 RoomMessage schema 拒绝', () => {
    const validate = loadSchema('RoomMessage');
    const msg = {
      type: 'RoomMessage',
      payload: {
        msg_id: VALID_UUID,
        user_id: VALID_UUID,
        content: 'test',
        avatar: true,           // ← 非法：不是 string 也不是 null
      },
      timestamp: 1700000000000,
    };
    const valid = validate(msg);
    expect(valid).toBe(false);
  });
});

// =============================================================================
// TC-NULLABLE-04: schema 文件 JSON 结构自查 — 验证 type 定义正确性
// =============================================================================
describe('TC-NULLABLE-04: schema JSON 结构自查', () => {
  // 验证 avatar 字段的 type 定义包含 null（结构层面）
  test.each([
    ['UserJoined', 'payload.properties.avatar'],
    ['MicTaken', 'payload.properties.avatar'],
    ['RoomMessage', 'payload.properties.avatar'],
  ])('%s: %s 的 type 定义必须包含 "null"', (schemaName, _fieldPath) => {
    const filePath = path.join(WS_SCHEMAS, `${schemaName}.schema.json`);
    const raw = fs.readFileSync(filePath, 'utf8');
    const schema = JSON.parse(raw) as {
      properties: {
        payload: {
          properties: {
            avatar: { type: string | string[] };
          };
        };
      };
    };

    const avatarType = schema.properties.payload.properties.avatar.type;

    // type 必须是数组（["string","null"]）且包含 "null"
    expect(Array.isArray(avatarType)).toBe(true);
    expect(avatarType).toContain('null');
    expect(avatarType).toContain('string');
  });

  // UserMuted 已有正确 nullable 示例 — 非回归检查
  test('UserMuted.schema.json: duration_sec 已正确定义为 ["integer","null"]（非回归）', () => {
    const filePath = path.join(WS_SCHEMAS, 'UserMuted.schema.json');
    const raw = fs.readFileSync(filePath, 'utf8');
    const schema = JSON.parse(raw) as {
      properties: {
        payload: {
          properties: {
            duration_sec: { type: string | string[] };
          };
        };
      };
    };
    const t = schema.properties.payload.properties.duration_sec.type;
    expect(Array.isArray(t)).toBe(true);
    expect(t).toContain('null');
    expect(t).toContain('integer');
  });
});
