package com.voice.room.android.feature.room

/**
 * 测试用 [IMicPermissionChecker] Fake 实现（T-30044）
 *
 * 允许在单元测试中精确控制：
 * - [hasMicPermission] 的返回值（[hasPermission] 字段）
 * - [requestMicPermission] 回调的触发时机（[grantPermission] / [denyPermission]）
 *
 * 用法示例：
 * ```kotlin
 * val fakeChecker = FakeMicPermissionChecker(hasPermission = false)
 * // 模拟拒绝权限
 * fakeChecker.denyPermission()
 * // 验证 MicLeave 已发出
 * assertTrue(fakeWsClient.sentMessages.any { "LeaveMic" in it })
 * ```
 */
class FakeMicPermissionChecker(
    /** 控制 [hasMicPermission] 的返回值 */
    var hasPermission: Boolean = true,
) : IMicPermissionChecker {

    /** [requestMicPermission] 被调用的次数 */
    var requestCallCount: Int = 0

    /** 最近一次 [requestMicPermission] 保存的回调（null = 尚未被调用） */
    private var pendingCallback: ((Boolean) -> Unit)? = null

    override fun hasMicPermission(): Boolean = hasPermission

    override fun requestMicPermission(onResult: (Boolean) -> Unit) {
        requestCallCount++
        pendingCallback = onResult
    }

    /**
     * 模拟用户授予权限，触发 [pendingCallback] 回调 `true`。
     *
     * @throws IllegalStateException 若 [requestMicPermission] 尚未被调用
     */
    fun grantPermission() {
        checkNotNull(pendingCallback) { "requestMicPermission() has not been called yet" }
        pendingCallback?.invoke(true)
        pendingCallback = null
    }

    /**
     * 模拟用户拒绝权限，触发 [pendingCallback] 回调 `false`。
     *
     * @throws IllegalStateException 若 [requestMicPermission] 尚未被调用
     */
    fun denyPermission() {
        checkNotNull(pendingCallback) { "requestMicPermission() has not been called yet" }
        pendingCallback?.invoke(false)
        pendingCallback = null
    }

    /** 是否有待处理的权限回调 */
    val hasPendingCallback: Boolean get() = pendingCallback != null
}
