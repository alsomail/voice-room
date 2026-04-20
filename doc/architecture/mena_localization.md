# 13. 中东本土化与 RTL 规范

1. **语言底座**：最小支持集为阿拉伯语（ar）和英语（en），禁止硬编码文案。
2. **Android RTL**：
   - 禁止写死 `left/right`
   - 必须使用 `start/end`
   - 使用 `paddingStart/paddingEnd`
   - 独立维护 `values-ar/`
3. **Web RTL**：
   - 根节点根据语言切换 `dir="rtl"` 或 `dir="ltr"`
   - CSS 使用逻辑属性，如 `margin-inline-start`
   - 箭头、抽屉、分页方向需镜像
4. **时间与数字**：
   - 时间遵循用户地区时区
   - 金额遵循 locale 格式
   - 阿语字符长度需预留布局空间
