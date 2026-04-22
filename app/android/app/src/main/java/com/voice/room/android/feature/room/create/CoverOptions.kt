package com.voice.room.android.feature.room.create

import androidx.annotation.DrawableRes
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.voice.room.android.R

// ─────────────────────────────────────────────────────────────────────────────
// 设计常量
// ─────────────────────────────────────────────────────────────────────────────

/** 封面选中态金色边框 ARGB 值（0xFFD4AF37） */
const val COVER_GOLD_BORDER_COLOR: Long = 0xFFD4AF37L

/** 封面选中态边框宽度（单位 dp） */
const val COVER_GOLD_BORDER_WIDTH_DP: Int = 2

// ─────────────────────────────────────────────────────────────────────────────
// 数据模型
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 单个预设封面选项
 *
 * @param url   与服务端一致的封面路径（如 /assets/covers/desert.webp）
 * @param resId 本地占位 Drawable 资源 ID（@DrawableRes 保证类型安全）
 */
data class CoverOption(
    val url: String,
    @DrawableRes val resId: Int,
)

// ─────────────────────────────────────────────────────────────────────────────
// 预设封面列表（8 张中东风格）
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 8 张预设封面，顺序与设计稿一致。
 * 对应 TDS T-30037 §二。
 */
val COVER_OPTIONS: List<CoverOption> = listOf(
    CoverOption("/assets/covers/desert.webp",      R.drawable.cover_desert),
    CoverOption("/assets/covers/mosque.webp",      R.drawable.cover_mosque),
    CoverOption("/assets/covers/lantern.webp",     R.drawable.cover_lantern),
    CoverOption("/assets/covers/eagle.webp",       R.drawable.cover_eagle),
    CoverOption("/assets/covers/rose.webp",        R.drawable.cover_rose),
    CoverOption("/assets/covers/yacht.webp",       R.drawable.cover_yacht),
    CoverOption("/assets/covers/sunset.webp",      R.drawable.cover_sunset),
    CoverOption("/assets/covers/calligraphy.webp", R.drawable.cover_calligraphy),
)

// ─────────────────────────────────────────────────────────────────────────────
// 状态持有者（State Holder）
// ─────────────────────────────────────────────────────────────────────────────

/**
 * CoverPickerState — 封面选择器的纯状态逻辑（可在 JVM 单测中直接测试）
 *
 * 职责：
 * - 维护当前选中封面 URL（[selectedUrl]）
 * - 暴露当前选中索引（[selectedIndex]）
 * - [selectCover] 更新选中项
 * - [confirmSelection] 触发 [onCoverSelected] 回调
 *
 * 在 Composable 中通过 `remember { CoverPickerState(...) }` 使用。
 *
 * @param initialUrl       初始选中封面 URL，默认为 [COVER_OPTIONS][0].url
 * @param onCoverSelected  确认时将选中 URL 传递给父组件的回调
 */
class CoverPickerState(
    initialUrl: String = COVER_OPTIONS[0].url,
    private val onCoverSelected: (String) -> Unit,
) {
    /**
     * 当前选中封面 URL
     *
     * 使用 `mutableStateOf` 委托，确保 Compose 能感知值变化并触发 Recomposition，
     * 从而使金色选中边框在用户点击后立即更新（R1 HIGH-01 修复）。
     */
    var selectedUrl: String by mutableStateOf(initialUrl)
        private set

    /** 当前选中封面在 [COVER_OPTIONS] 中的下标（-1 表示未找到） */
    val selectedIndex: Int
        get() = COVER_OPTIONS.indexOfFirst { it.url == selectedUrl }

    /**
     * 选中指定下标的封面。
     *
     * @param index [COVER_OPTIONS] 中的合法下标（0..7）
     */
    fun selectCover(index: Int) {
        selectedUrl = COVER_OPTIONS[index].url
    }

    /**
     * 确认选择，将 [selectedUrl] 通过 [onCoverSelected] 回调传递出去。
     */
    fun confirmSelection() {
        onCoverSelected(selectedUrl)
    }
}
