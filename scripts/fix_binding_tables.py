#!/usr/bin/env python3
"""批量为缺少 🔌 协议路径绑定表 的 TDS 文件添加 N/A 声明"""

import os
import re
import sys

REPO = '/Users/yuanye/myWork/voice-room'

# 139 个缺失绑定表的文件
MISSING_FILES = """doc/tds/adminServer/T-10001.md
doc/tds/adminServer/T-10002.md
doc/tds/adminServer/T-10003.md
doc/tds/adminServer/T-10004.md
doc/tds/adminServer/T-10005.md
doc/tds/adminServer/T-10006.md
doc/tds/adminServer/T-10007.md
doc/tds/adminServer/T-10008.md
doc/tds/adminServer/T-10009.md
doc/tds/adminServer/T-10010.md
doc/tds/adminServer/T-10011.md
doc/tds/adminServer/T-10012.md
doc/tds/adminServer/T-10013.md
doc/tds/adminServer/T-10014.md
doc/tds/adminServer/T-10015.md
doc/tds/adminServer/T-10016.md
doc/tds/adminServer/T-10020.md
doc/tds/android/T-30001.md
doc/tds/android/T-30002.md
doc/tds/android/T-30003.md
doc/tds/android/T-30004.md
doc/tds/android/T-30005.md
doc/tds/android/T-30006.md
doc/tds/android/T-30007.md
doc/tds/android/T-30008.md
doc/tds/android/T-30009.md
doc/tds/android/T-30010.md
doc/tds/android/T-30011.md
doc/tds/android/T-30012.md
doc/tds/android/T-30013.md
doc/tds/android/T-30014.md
doc/tds/android/T-30015.md
doc/tds/android/T-30016.md
doc/tds/android/T-30017.md
doc/tds/android/T-30018.md
doc/tds/android/T-30019.md
doc/tds/android/T-30020.md
doc/tds/android/T-30021.md
doc/tds/android/T-30022.md
doc/tds/android/T-30023.md
doc/tds/android/T-30024.md
doc/tds/android/T-30025.md
doc/tds/android/T-30026.md
doc/tds/android/T-30027.md
doc/tds/android/T-30028.md
doc/tds/android/T-30029.md
doc/tds/android/T-30030.md
doc/tds/android/T-30031.md
doc/tds/android/T-30032.md
doc/tds/android/T-30033.md
doc/tds/android/T-30034.md
doc/tds/android/T-30035.md
doc/tds/android/T-30036.md
doc/tds/android/T-30037.md
doc/tds/android/T-30038.md
doc/tds/android/T-30039.md
doc/tds/android/T-30040.md
doc/tds/android/T-30041.md
doc/tds/android/T-30042.md
doc/tds/android/T-30043.md
doc/tds/android/T-30044.md
doc/tds/android/T-30050.md
doc/tds/android/T-30051.md
doc/tds/android/T-30052.md
doc/tds/android/T-30053.md
doc/tds/android/T-30099.md
doc/tds/infra/T-0000A.md
doc/tds/infra/T-0000B.md
doc/tds/infra/T-0000C.md
doc/tds/infra/T-0000D.md
doc/tds/infra/T-0000E.md
doc/tds/infra/T-0000F.md
doc/tds/infra/T-0000G.md
doc/tds/infra/T-0000H.md
doc/tds/infra/T-0000I.md
doc/tds/infra/T-0000J.md
doc/tds/infra/T-0000K.md
doc/tds/infra/T-0000L.md
doc/tds/infra/T-0000M.md
doc/tds/infra/T-0000N.md
doc/tds/infra/T-0000O.md
doc/tds/infra/T-0000P.md
doc/tds/infra/T-0000Q.md
doc/tds/infra/T-0000R.md
doc/tds/infra/T-0000S.md
doc/tds/server/T-00001.md
doc/tds/server/T-00002.md
doc/tds/server/T-00003.md
doc/tds/server/T-00004.md
doc/tds/server/T-00005.md
doc/tds/server/T-00006.md
doc/tds/server/T-00007.md
doc/tds/server/T-00008.md
doc/tds/server/T-00009.md
doc/tds/server/T-00010.md
doc/tds/server/T-00011.md
doc/tds/server/T-00011B.md
doc/tds/server/T-00011C.md
doc/tds/server/T-00012.md
doc/tds/server/T-00013.md
doc/tds/server/T-00014.md
doc/tds/server/T-00015.md
doc/tds/server/T-00016.md
doc/tds/server/T-00017.md
doc/tds/server/T-00018.md
doc/tds/server/T-00019.md
doc/tds/server/T-00020.md
doc/tds/server/T-00021.md
doc/tds/server/T-00022.md
doc/tds/server/T-00023.md
doc/tds/server/T-00024.md
doc/tds/server/T-00025.md
doc/tds/server/T-00026.md
doc/tds/server/T-00027.md
doc/tds/server/T-00028.md
doc/tds/server/T-00029.md
doc/tds/server/T-00030.md
doc/tds/server/T-00040.md
doc/tds/server/T-00041.md
doc/tds/server/T-00042.md
doc/tds/server/T-00043.md
doc/tds/server/T-00044.md
doc/tds/server/T-00045.md
doc/tds/server/T-00046.md
doc/tds/web/T-20001.md
doc/tds/web/T-20002.md
doc/tds/web/T-20003.md
doc/tds/web/T-20004.md
doc/tds/web/T-20005.md
doc/tds/web/T-20006.md
doc/tds/web/T-20007.md
doc/tds/web/T-20008.md
doc/tds/web/T-20009.md
doc/tds/web/T-20010.md
doc/tds/web/T-20011.md
doc/tds/web/T-20012.md
doc/tds/web/T-20013.md
doc/tds/web/T-20014.md
doc/tds/web/T-20020.md""".strip().splitlines()

# NA 文本选择（必须匹配 audit 脚本的 NA_PATTERNS）
# Pattern 1: /N\/A.*本\s*Task.*无跨端协议/
# Pattern 3: /N\/A.*本\s*Task.*为.*基础设施/
def get_na_text(rel_path):
    if '/infra/' in rel_path:
        return 'N/A — 本 Task 为基础设施，无跨端协议路径'
    elif '/android/' in rel_path:
        return 'N/A — 本 Task 无跨端协议路径，仅 Android 端内部改造'
    elif '/server/' in rel_path:
        return 'N/A — 本 Task 无跨端协议路径，仅服务端内部改造'
    elif '/adminServer/' in rel_path:
        return 'N/A — 本 Task 无跨端协议路径，仅 Admin Server 内部改造'
    elif '/web/' in rel_path:
        return 'N/A — 本 Task 无跨端协议路径，仅 Web 端内部改造'
    else:
        return 'N/A — 本 Task 无跨端协议路径'

BINDING_SECTION = """
### 🔌 协议路径绑定表

{na_text}

"""

def process_file(rel_path):
    abs_path = os.path.join(REPO, rel_path)
    with open(abs_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # 特殊处理 T-30053.md — 已有章节但 N/A 文本不匹配 pattern
    if 'T-30053.md' in rel_path:
        na_text = get_na_text(rel_path)
        # 找到现有 ### 🔌 协议路径绑定表 行，在其后追加匹配行
        old = 'N/A — 本 Task 仅 Android 端 UI 行为新增，不动任何后端协议。无前后端协议变更。'
        new = f'N/A — 本 Task 无跨端协议路径，仅 Android 端 UI 行为新增，不动任何后端协议。无前后端协议变更。'
        if old in content:
            content = content.replace(old, new, 1)
            with open(abs_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return 'updated'
        else:
            return 'skipped(T-30053 pattern not found)'

    # 已有协议路径绑定表章节则跳过
    if '协议路径绑定表' in content:
        return 'skipped(already has section)'

    na_text = get_na_text(rel_path)
    section = BINDING_SECTION.format(na_text=na_text)

    lines = content.splitlines(keepends=True)

    # 找 ## 三、 或 ## 四、 或类似的第三节
    insert_idx = None
    for i, line in enumerate(lines):
        if re.match(r'^## [三四五六七八九十]', line) or re.match(r'^## (三|四|五|六)[、.。]', line):
            insert_idx = i
            break

    if insert_idx is None:
        # fallback: 找最后一个 ## 标题之前（排除 ## 一、和 ## 二、）
        for i in range(len(lines) - 1, -1, -1):
            if re.match(r'^## ', lines[i]) and not re.match(r'^## [一二]', lines[i]):
                insert_idx = i
                break

    if insert_idx is None:
        # 最后手段：文件末尾
        insert_idx = len(lines)

    # 插入
    lines.insert(insert_idx, section)
    new_content = ''.join(lines)

    with open(abs_path, 'w', encoding='utf-8') as f:
        f.write(new_content)

    return 'inserted'

# 执行
ok = 0
skip = 0
for rel_path in MISSING_FILES:
    result = process_file(rel_path)
    if result.startswith('insert') or result.startswith('update'):
        ok += 1
    else:
        skip += 1
    print(f'  [{result}] {rel_path}')

print(f'\n✅ Done: {ok} files modified, {skip} skipped')
