package com.voice.room.android.util

import android.content.Context
import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.ui.res.stringResource

/**
 * 国际化文案占位类型（缺陷 #4 修复）。
 *
 * ViewModel 层禁止持有任何特定语言的字符串字面量；改为持有
 * [StringResource]（@StringRes + 可选 format 参数），由 UI 层在 Composable
 * 中通过 [asString] 真正解析为目标 locale 的字符串。
 *
 * 这同时避免：
 *  1. 中文字面量硬编码 → 阿语 / 英语用户看到非目标语言
 *  2. ViewModel 测试无法在不引入 Android 框架时验证文本（现在断言 resId）
 *  3. 切换 locale 时旧 ViewModel state 文案不刷新
 *
 * 用法：
 *   ViewModel 端: `UiText.of(R.string.hall_network_error)`
 *   UI 端:       `Text(error.asString())`
 */
sealed class UiText {

    /** [@StringRes] 资源 + 可选 format 参数（按 [String.format] 顺序传入）。 */
    data class StringResource(
        @StringRes val resId: Int,
        val args: List<Any> = emptyList(),
    ) : UiText()

    /** 在 Composable 中解析为字符串（推荐）。 */
    @Composable
    @ReadOnlyComposable
    fun asString(): String = when (this) {
        is StringResource -> stringResource(resId, *args.toTypedArray())
    }

    /** 非 Composable（如 Toast 回调）解析。 */
    fun asString(context: Context): String = when (this) {
        is StringResource -> context.getString(resId, *args.toTypedArray())
    }

    companion object {
        /** 便捷构造：无参字符串资源。 */
        fun of(@StringRes resId: Int): UiText = StringResource(resId)

        /** 便捷构造：带 format 参数的字符串资源。 */
        fun of(@StringRes resId: Int, vararg args: Any): UiText =
            StringResource(resId, args.toList())
    }
}
