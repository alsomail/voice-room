import { describe, test, expect } from '@jest/globals';
import { extractRustJsonMacroFields, extractRustSerdeFields } from '../lib/rust-ast-extractor';
import { extractKotlinSerializedNameFields } from '../lib/kotlin-ast-extractor';
import { extractZodObjectFields } from '../lib/zod-extractor';
import { compareWithSchema, isCamelCase, loadSchemaFields } from '../lib/schema-comparator';

describe('FIELD-1: Rust camelCase 字段检测', () => {
  test('json! 宏中含 camelCase 字段时应被检测为违规', () => {
    const source = `
      let payload = json!({
        "micIndex": 0,
        "room_id": "abc123",
        "userId": 999
      });
    `;
    const fields = extractRustJsonMacroFields(source);
    expect(fields).toContain('micIndex');
    expect(fields).toContain('userId');
    expect(fields).toContain('room_id');

    const camelFields = fields.filter(isCamelCase);
    expect(camelFields).toContain('micIndex');
    expect(camelFields).toContain('userId');
    expect(camelFields).not.toContain('room_id');
  });

  test('compareWithSchema 对 camelCase 字段报 P0', () => {
    const extractedFields = new Map([
      ['micIndex', ['app/server/src/handler.rs:42']],
      ['room_id', ['app/server/src/handler.rs:43']],
    ]);
    const schemaFields = new Set(['mic_index', 'room_id', 'user_id']);
    const mismatches = compareWithSchema(extractedFields, schemaFields, 'test.schema.json');

    const p0 = mismatches.filter(m => m.severity === 'P0');
    expect(p0.length).toBeGreaterThanOrEqual(1);
    expect(p0.some(m => m.fieldName === 'micIndex' && m.issue === 'camelCase')).toBe(true);
  });
});

describe('FIELD-2: Android SerializedName 缺失检测', () => {
  test('应正确提取 @SerializedName 中的字段名', () => {
    const source = `
      data class UserDto(
        @SerializedName("user_id")
        val userId: Long,
        @SerializedName("room_id")
        val roomId: String,
        val rawField: String
      )
    `;
    const fields = extractKotlinSerializedNameFields(source);
    expect(fields).toContain('user_id');
    expect(fields).toContain('room_id');
    expect(fields).not.toContain('rawField');
  });
});

describe('FIELD-3: Zod schema 字段提取', () => {
  test('应正确提取 z.object() 中的字段名', () => {
    const source = `
      const MicSlotSchema = z.object({
        mic_index: z.number(),
        user_id: z.string(),
        is_muted: z.boolean(),
      });
    `;
    const fields = extractZodObjectFields(source);
    expect(fields).toContain('mic_index');
    expect(fields).toContain('user_id');
    expect(fields).toContain('is_muted');
  });

  test('Zod 中无 camelCase 时 compareWithSchema 应返回 0 个 P0', () => {
    const extractedFields = new Map([
      ['mic_index', ['static/src/schemas.ts:10']],
      ['user_id', ['static/src/schemas.ts:11']],
    ]);
    const schemaFields = new Set(['mic_index', 'user_id', 'is_muted']);
    const mismatches = compareWithSchema(extractedFields, schemaFields, 'test.schema.json');
    const p0 = mismatches.filter(m => m.severity === 'P0');
    expect(p0.length).toBe(0);
  });
});

describe('isCamelCase 工具函数', () => {
  test('识别 camelCase', () => {
    expect(isCamelCase('micIndex')).toBe(true);
    expect(isCamelCase('userId')).toBe(true);
    expect(isCamelCase('roomId')).toBe(true);
  });

  test('识别 snake_case 为合规', () => {
    expect(isCamelCase('mic_index')).toBe(false);
    expect(isCamelCase('user_id')).toBe(false);
    expect(isCamelCase('room_id')).toBe(false);
  });

  test('识别单词（无下划线无大写）为合规', () => {
    expect(isCamelCase('type')).toBe(false);
    expect(isCamelCase('code')).toBe(false);
  });
});

describe('REGRESSION: T-0000T 原有测试 0 回归', () => {
  test('protocol-binding-audit 核心导出仍存在', async () => {
    const mod = await import('../protocol-binding-audit');
    expect(typeof mod.parseBindingTable).toBe('function');
    expect(typeof mod.auditBindings).toBe('function');
    expect(typeof mod.generateReport).toBe('function');
  });
});
