package com.voice.room.android.feature.room

/**
 * 麦克风权限检查接口（T-30044）
 *
 * 供 [RoomViewModel] 在收到 ForceTakeMic 广播时，
 * 检查并按需请求录音权限，无需依赖 Compose/Accompanist。
 *
 * - 生产实现：委托给 Android 系统权限 API（在 RoomScreen 传入）
 * - 测试实现：[FakeMicPermissionChecker]（可控 hasMicPermission 返回值 + 回调时机）
 */
interface IMicPermissionChecker {

    /**
     * 查询当前是否已授予 RECORD_AUDIO 权限。
     *
     * @return true = 已授权，false = 未授权
     */
    fun hasMicPermission(): Boolean

    /**
     * 发起权限请求，结果通过 [onResult] 回调返回。
     *
     * 注意：回调可能在任意线程被调用，ViewModel 内应用 [kotlinx.coroutines.launch] 包裹。
     *
     * @param onResult 权限授予结果回调（true = 用户批准，false = 用户拒绝）
     */
    fun requestMicPermission(onResult: (Boolean) -> Unit)
}

/**
 * 始终视为已授权的默认实现（生产 MVP 阶段 / 权限已授予时使用）。
 *
 * 不发起任何系统权限弹窗，直接回调 `onResult(true)`。
 */
class AlwaysGrantedMicPermissionChecker : IMicPermissionChecker {
    override fun hasMicPermission(): Boolean = true
    override fun requestMicPermission(onResult: (Boolean) -> Unit) = onResult(true)
}
