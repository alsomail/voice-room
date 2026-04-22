package com.voice.room.android.feature.room.create

import androidx.compose.runtime.snapshots.SnapshotMutableState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — CoverPickerState Compose 感知性验证 (T-30037 Review R1 HIGH-01)
 *
 * 目标：验证 [CoverPickerState.selectedUrl] 使用 `mutableStateOf` 委托，
 * 确保 Compose 能感知属性变化并触发 Recomposition（金色边框随点击更新）。
 *
 * 说明：
 * - JVM 单元测试无法验证 Compose Recomposition 本身，
 *   但可通过反射检查 delegate 类型为 [SnapshotMutableState] 来验证正确声明。
 * - [SnapshotMutableState] 是 Compose Runtime 的核心接口，
 *   只有通过 `mutableStateOf()` 创建的状态才实现该接口。
 *
 * 测试用例：
 * - **HIGH01-01** `selectedUrl` 的委托必须是 `SnapshotMutableState<String>`（Compose 可感知）
 * - **HIGH01-02** `mutableStateOf` 委托在切换封面后值正确更新
 * - **HIGH01-03** 自定义初始 URL 的 `mutableStateOf` 委托初始值正确
 * - **HIGH01-04** 多次 `selectCover` 每次都更新 `mutableStateOf` 内部值
 * - **HIGH01-05** HIGH-02 集成验证：确认回调传出的 URL 与 `mutableStateOf` 当前值一致
 */
class CoverPickerStateComposeTest {

    // ─────────────────────────────────────────────
    // HIGH01-01: selectedUrl 委托类型必须是 SnapshotMutableState
    // ─────────────────────────────────────────────

    /**
     * 通过反射验证 [CoverPickerState.selectedUrl] 使用了 `mutableStateOf` 委托。
     *
     * Kotlin by-delegation 会在类中生成名为 `<propertyName>$delegate` 的合成字段。
     * `mutableStateOf()` 返回的对象实现 [SnapshotMutableState] 接口——
     * 这是 Compose 感知状态变化的必要条件。
     */
    @Test
    fun HIGH01_01_selectedUrl_delegateIsSnapshotMutableState() {
        val state = CoverPickerState(onCoverSelected = {})

        // 通过反射获取 Kotlin 委托属性的 backing delegate 字段
        val delegateField = CoverPickerState::class.java
            .getDeclaredField("selectedUrl\$delegate")
        delegateField.isAccessible = true
        val delegate = delegateField.get(state)

        assertNotNull(
            "selectedUrl\$delegate field should not be null",
            delegate
        )
        assertTrue(
            "selectedUrl must be backed by SnapshotMutableState<*> " +
                "(declare as: var selectedUrl by mutableStateOf(initialUrl)). " +
                "Actual delegate type: ${delegate?.javaClass?.name}",
            delegate is SnapshotMutableState<*>
        )
    }

    // ─────────────────────────────────────────────
    // HIGH01-02: mutableStateOf 委托在切换封面后值正确
    // ─────────────────────────────────────────────

    @Test
    fun HIGH01_02_mutableStateDelegate_updatesValueOnSelectCover() {
        val state = CoverPickerState(onCoverSelected = {})

        // 初始值是 COVER_OPTIONS[0].url
        assertEquals(COVER_OPTIONS[0].url, state.selectedUrl)

        // 切换到 index=3
        state.selectCover(3)
        assertEquals(
            "After selectCover(3), SnapshotMutableState value should update to COVER_OPTIONS[3].url",
            COVER_OPTIONS[3].url,
            state.selectedUrl
        )
    }

    // ─────────────────────────────────────────────
    // HIGH01-03: 自定义 initialUrl 时 mutableStateOf 初始值正确
    // ─────────────────────────────────────────────

    @Test
    fun HIGH01_03_customInitialUrl_mutableStateDelegateHoldsCorrectInitialValue() {
        val customInitialUrl = COVER_OPTIONS[5].url
        val state = CoverPickerState(
            initialUrl = customInitialUrl,
            onCoverSelected = {}
        )

        // 验证反射获取到的 delegate 其值等于 customInitialUrl
        val delegateField = CoverPickerState::class.java
            .getDeclaredField("selectedUrl\$delegate")
        delegateField.isAccessible = true

        @Suppress("UNCHECKED_CAST")
        val delegate = delegateField.get(state) as SnapshotMutableState<String>

        assertEquals(
            "SnapshotMutableState initial value should equal customInitialUrl",
            customInitialUrl,
            delegate.value
        )
    }

    // ─────────────────────────────────────────────
    // HIGH01-04: 多次切换 selectCover，mutableStateOf 每次更新
    // ─────────────────────────────────────────────

    @Test
    fun HIGH01_04_mutableStateDelegate_updatesOnEachSelectCover() {
        val state = CoverPickerState(onCoverSelected = {})

        val delegateField = CoverPickerState::class.java
            .getDeclaredField("selectedUrl\$delegate")
        delegateField.isAccessible = true

        @Suppress("UNCHECKED_CAST")
        val delegate = delegateField.get(state) as SnapshotMutableState<String>

        // 依次切换到 1, 5, 2，每次验证 delegate.value 同步更新
        listOf(1, 5, 2).forEach { index ->
            state.selectCover(index)
            assertEquals(
                "After selectCover($index), SnapshotMutableState.value should be COVER_OPTIONS[$index].url",
                COVER_OPTIONS[index].url,
                delegate.value
            )
        }
    }

    // ─────────────────────────────────────────────
    // HIGH01-05: 集成验证 — 确认回调与 mutableStateOf 值一致
    // ─────────────────────────────────────────────

    /**
     * HIGH-02 集成逻辑验证：
     * `CoverPickerBottomSheet` 通过 `onCoverSelected` 回调将 `selectedUrl` 传给
     * `viewModel.updateCoverUrl()`，本测试验证回调接收到的 URL 与 `selectedUrl` 一致。
     */
    @Test
    fun HIGH01_05_confirmSelection_callbackUrlMatchesMutableStateValue() {
        var callbackUrl: String? = null
        val state = CoverPickerState(onCoverSelected = { url -> callbackUrl = url })

        state.selectCover(6)
        val expectedUrl = state.selectedUrl   // 读取 mutableStateOf 的当前值

        state.confirmSelection()

        assertEquals(
            "onCoverSelected callback should receive the same URL as mutableStateOf.value",
            expectedUrl,
            callbackUrl
        )
    }
}
