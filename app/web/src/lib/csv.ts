/**
 * CSV 导出工具（T-20013）
 *
 * 轻量 CSV 生成与下载，不依赖 papaparse。
 * 支持：
 *   - 对象数组 → CSV 字符串
 *   - 触发浏览器下载
 *   - 文件名生成：user_{userId}_events_{ts}.csv
 */

/** 生成事件 CSV 文件名 */
export function generateEventCsvFilename(userId: string, ts?: number): string {
  const timestamp = ts ?? Date.now();
  return `user_${userId}_events_${timestamp}.csv`;
}

/**
 * 将对象数组转换为 CSV 字符串
 * - 第一行为字段名（keys）
 * - 字段值中含逗号/引号/换行时加双引号包裹，内部引号转义为 ""
 */
export function objectsToCsv(rows: Record<string, unknown>[]): string {
  if (rows.length === 0) return '';

  const headers = Object.keys(rows[0]);
  const escape = (val: unknown): string => {
    const str = val == null
      ? ''
      : typeof val === 'object'
        ? JSON.stringify(val)
        : String(val);
    // 含特殊字符时用引号包裹
    if (str.includes(',') || str.includes('"') || str.includes('\n') || str.includes('\r')) {
      return `"${str.replace(/"/g, '""')}"`;
    }
    return str;
  };

  const lines = [
    headers.join(','),
    ...rows.map((row) => headers.map((h) => escape(row[h])).join(',')),
  ];

  return lines.join('\r\n');
}

/**
 * 触发浏览器下载 CSV 文件
 * @param csvContent  CSV 字符串
 * @param filename    下载文件名
 */
export function downloadCsv(csvContent: string, filename: string): void {
  const bom = '\uFEFF'; // UTF-8 BOM（兼容 Excel 中文）
  const blob = new Blob([bom + csvContent], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);

  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);

  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
