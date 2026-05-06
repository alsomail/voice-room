/**
 * schema-validator.ts — 用 AJV 8 验证 WS 信令是否符合 JSON Schema。
 *
 * PROTO-BINDING:
 *   Schema 事实源：doc/protocol/schemas/ws/<SchemaName>.schema.json
 *   T-00100: JSON Schema 冻结，34 WS 信令
 *   T-00103: Server deny_unknown_fields + schema_guard
 *
 * 使用方式：
 *   validateOrThrow('Pong', message);           // 校验失败抛 ValidationError
 *   const ok = validate('JoinRoomResult', msg); // 返回 boolean
 */

import Ajv, { type ValidateFunction } from 'ajv';
import addFormats from 'ajv-formats';
import * as fs from 'node:fs';
import * as path from 'node:path';

// ─────────────────────────────────────────────────────────────────────────────
// AJV 实例（全局单例，schema 缓存在内部）
// ─────────────────────────────────────────────────────────────────────────────

const ajv = new Ajv({
  allErrors: true,
  strict: false, // 允许 schema 中 $schema / $id 字段
});
addFormats(ajv);

/** Schema 文件所在目录（相对项目根） */
const SCHEMA_DIR = path.resolve(
  __dirname,
  '../../../../doc/protocol/schemas/ws',
);

/** 已编译 schema 的缓存 Map<schemaName, ValidateFunction> */
const schemaCache: Map<string, ValidateFunction> = new Map();

// ─────────────────────────────────────────────────────────────────────────────
// 公开接口
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 从磁盘加载并编译指定 schema（首次加载后缓存）。
 * @param schemaName  schema 文件名（不含 .schema.json 后缀），例如 "Pong"
 * @throws 如果文件不存在或 JSON 格式非法
 */
export function loadSchema(schemaName: string): ValidateFunction {
  if (schemaCache.has(schemaName)) {
    return schemaCache.get(schemaName)!;
  }
  const filePath = path.join(SCHEMA_DIR, `${schemaName}.schema.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(
      `[schema-validator] Schema file not found: ${filePath}\n` +
        `  Known schemas are in: ${SCHEMA_DIR}\n` +
        `  DISCREPANCY NOTE: If this schema is listed in protocol docs but not on disk, ` +
        `  record it in T-00104.md §四.`,
    );
  }
  const raw = fs.readFileSync(filePath, 'utf8');
  const schemaDef = JSON.parse(raw) as object;
  const validate = ajv.compile(schemaDef);
  schemaCache.set(schemaName, validate);
  return validate;
}

/**
 * 验证数据是否符合指定 schema。
 * @returns true = 合法；false = 不合法（错误详情在 validate.errors 中）
 */
export function validate(schemaName: string, data: unknown): boolean {
  const fn = loadSchema(schemaName);
  return fn(data) as boolean;
}

/**
 * 验证数据是否符合指定 schema；不合法时抛出带详细信息的 ValidationError。
 *
 * 每个 WS 消息收到后必须调用此函数（T-00104 DoD §5）。
 */
export function validateOrThrow(schemaName: string, data: unknown): void {
  const fn = loadSchema(schemaName);
  const valid = fn(data) as boolean;
  if (!valid) {
    const errors = ajv.errorsText(fn.errors, { separator: '\n  ', dataVar: 'msg' });
    throw new ValidationError(
      `[schema-validator] "${schemaName}" validation FAILED:\n  ${errors}\n` +
        `  Received: ${JSON.stringify(data, null, 2)}`,
      fn.errors ?? [],
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 错误类型
// ─────────────────────────────────────────────────────────────────────────────

export class ValidationError extends Error {
  public readonly validationErrors: object[];
  constructor(message: string, errors: object[]) {
    super(message);
    this.name = 'ValidationError';
    this.validationErrors = errors;
  }
}

/**
 * 检查指定 schema 文件是否存在（不抛异常，用于 CROSS-6 GiftReceived 缺失检测）。
 */
export function schemaExists(schemaName: string): boolean {
  const filePath = path.join(SCHEMA_DIR, `${schemaName}.schema.json`);
  return fs.existsSync(filePath);
}
