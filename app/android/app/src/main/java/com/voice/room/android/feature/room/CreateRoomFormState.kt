package com.voice.room.android.feature.room

/**
 * 创建房间表单 UI 状态 (T-30036)
 *
 * 包含所有表单字段、提交状态和校验逻辑。
 * 由 [CreateRoomViewModel.formState] 持有，并通过 updateXxx() 方法更新。
 *
 * ### canSubmit 规则
 * - 房名非空且 ≤ 30 字符
 * - 公告 ≤ 200 字符
 * - 密码开关开启时：密码必须为 6 位纯数字
 * - 封面 URL 非空
 * - 未在提交中
 *
 * @param title           房间标题（1–30 字符）
 * @param coverUrl        封面图 URL（非空代表已选择封面）
 * @param category        房间分类，默认 [RoomCategory.CHAT]
 * @param announcement    公告（最多 200 字符，可为空）
 * @param passwordEnabled 是否开启密码房
 * @param password        密码（6 位纯数字，仅 [passwordEnabled]=true 时校验）
 * @param submitting      正在提交（true 时按钮禁用）
 * @param error           错误信息（校验失败或 API 失败）
 * @param navigatedRoomId 非 null 时代表创建成功，UI 应导航到该房间
 */
data class CreateRoomFormState(
    val title: String = "",
    val coverUrl: String = "",
    val category: RoomCategory = RoomCategory.CHAT,
    val announcement: String = "",
    val passwordEnabled: Boolean = false,
    val password: String = "",
    val submitting: Boolean = false,
    val error: String? = null,
    val navigatedRoomId: String? = null
) {
    /**
     * 所有表单字段合法 + 未在提交中 → true，否则 false。
     *
     * UI 层将提交按钮的 `enabled` 绑定到此属性。
     */
    val canSubmit: Boolean
        get() = title.isNotBlank()
             && title.length <= 30
             && announcement.length <= 200
             && (!passwordEnabled || password.matches(Regex("\\d{6}")))
             && coverUrl.isNotEmpty()
             && !submitting
}
