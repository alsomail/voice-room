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
     * 缺陷 #1 修复：协议契约字段为 `locked_remaining_sec`（秒），
     * 因此 UI 状态也改用秒为单位；显示分钟时由 UI 层做 `ceil(secs / 60)` 转换。
     *
     * @param remainingSeconds 距解锁剩余秒数
     */
    data class Locked(val remainingSeconds: Int) : PasswordDialogState()
}
