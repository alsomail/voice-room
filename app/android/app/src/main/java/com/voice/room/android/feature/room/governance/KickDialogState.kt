package com.voice.room.android.feature.room.governance

/**
 * 踢人原因枚举（T-30041）
 *
 * 与服务端协议字段对应：
 * - [Harassment] → "harassment"
 * - [Spam]       → "spam"
 * - [Abuse]      → "abuse"
 * - [Other]      → 用户自定义文本（key 仅作兜底占位）
 *
 * @param key 上报给服务端的字段值（预设原因直接使用；Other 时使用 customText）
 */
enum class KickReason(val key: String) {
    Harassment("harassment"),
    Spam("spam"),
    Abuse("abuse"),
    Other("other"),
}

/**
 * KickReasonDialog 对话框状态（T-30041）
 *
 * ### canSubmit 规则
 * - [submitting] = true → false（防止重复提交）
 * - [selected] ≠ [KickReason.Other] → true（预设原因直接可提交）
 * - [selected] = [KickReason.Other] 且 [customText].isBlank() → false
 * - [selected] = [KickReason.Other] 且 [customText].isNotBlank() → true
 *
 * @param selected    当前选中的踢出原因，默认 [KickReason.Harassment]
 * @param customText  "其他"原因的自定义输入文本（max 100 字符由 UI 层限制）
 * @param submitting  正在提交中（发送 WS 信令期间），防止重复点击
 */
data class KickDialogState(
    val selected: KickReason = KickReason.Harassment,
    val customText: String = "",
    val submitting: Boolean = false,
) {
    /**
     * 是否允许点击确认按钮。
     *
     * - submitting 期间始终返回 false
     * - 选择"其他"且 customText 为空/空白时返回 false
     * - 其余情况返回 true
     */
    val canSubmit: Boolean
        get() = !submitting && (selected != KickReason.Other || customText.isNotBlank())
}
