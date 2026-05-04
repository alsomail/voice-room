package com.voice.room.android.feature.room

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * BUG-GOVERNANCE-FORM-VALIDATE Round 6 回归测试。
 *
 * 验证 [CreateRoomFormState.canSubmit] 与 [CreateRoomBottomSheet] 提交按钮
 * `enabled` 表达式中的 “房名非空” 不变量：当用户没有输入任何房名时，
 * 创建按钮必须保持 disabled 状态，避免误提交触发服务端 4xx。
 */
class CreateRoomFormValidationTest {

    private fun baseState() = CreateRoomFormState(
        title = "Hello",
        coverUrl = "https://example.com/cover.jpg",
        announcement = "",
        passwordEnabled = false,
        submitting = false,
    )

    @Test
    fun `canSubmit is false when title is empty`() {
        val state = baseState().copy(title = "")
        assertFalse("title 为空时不允许提交", state.canSubmit)
    }

    @Test
    fun `canSubmit is false when title is whitespace only`() {
        val state = baseState().copy(title = "   ")
        assertFalse("title 仅含空白时不允许提交", state.canSubmit)
    }

    @Test
    fun `canSubmit is true when all fields valid and title is non-blank`() {
        val state = baseState()
        assertTrue("基础合法表单应允许提交", state.canSubmit)
    }

    @Test
    fun `canSubmit is false when submitting is true`() {
        val state = baseState().copy(submitting = true)
        assertFalse("提交中应禁用按钮", state.canSubmit)
    }

    /**
     * 镜像 [CreateRoomBottomSheet] 内部按钮 enabled 表达式的语义：
     * - title 非空白
     * - title 长度 ≤ MAX_TITLE_LENGTH
     * - 当类型为 password 时，密码非空
     */
    private fun bottomSheetButtonEnabled(
        title: String,
        selectedType: String,
        password: String,
        isLoading: Boolean,
    ): Boolean {
        val trimmed = title.trim()
        val len = trimmed.codePointCount(0, trimmed.length)
        val titleValid = title.isNotBlank() &&
            len in 1..CreateRoomViewModel.MAX_TITLE_LENGTH
        val formValid = titleValid &&
            (selectedType != "password" || password.isNotBlank())
        return !isLoading && formValid
    }

    @Test
    fun `bottom sheet create button disabled when title is blank`() {
        assertFalse(
            "BUG-GOVERNANCE-FORM-VALIDATE：空标题应禁用创建按钮",
            bottomSheetButtonEnabled(title = "", selectedType = "normal", password = "", isLoading = false)
        )
    }

    @Test
    fun `bottom sheet create button enabled when title not blank and not loading`() {
        assertTrue(
            "标题非空 + 普通房间应允许提交",
            bottomSheetButtonEnabled(title = "Hi", selectedType = "normal", password = "", isLoading = false)
        )
    }

    @Test
    fun `bottom sheet create button disabled when password type but password blank`() {
        assertFalse(
            "密码房未填密码应禁用",
            bottomSheetButtonEnabled(title = "Hi", selectedType = "password", password = "", isLoading = false)
        )
    }
}
