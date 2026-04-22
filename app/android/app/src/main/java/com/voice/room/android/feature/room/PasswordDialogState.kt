package com.voice.room.android.feature.room

/**
 * 密码输入弹窗状态密封类（T-30038）
 *
 * 状态流转：
 * null → Idle（openPasswordDialog）→ Verifying（verifyPassword）
 *   ├── Error(remainingAttempts)  （HTTP 40103）
 *   ├── Locked(remainingMinutes)  （HTTP 42910）
 *   └── null                      （成功 / dismiss）
 */
sealed class PasswordDialogState {

    /** 弹窗已打开，等待用户输入 */
    object Idle : PasswordDialogState()

    /** 正在请求验证接口，输入框不可操作 */
    object Verifying : PasswordDialogState()

    /**
     * 密码错误（HTTP 40103）
     *
     * @param remainingAttempts 剩余可重试次数
     */
    data class Error(val remainingAttempts: Int) : PasswordDialogState()

    /**
     * 账号已锁定（HTTP 42910）
     *
     * @param remainingMinutes 距解锁剩余分钟数
     */
    data class Locked(val remainingMinutes: Int) : PasswordDialogState()
}
