package com.voice.room.android.domain.room

/**
 * 密码验证失败（HTTP 40103）。
 *
 * 缺陷 #4 修复：移除中文 message。i18n 文案由 UI 层根据
 * `remainingAttempts` 自行格式化（参见 strings.xml 中的资源条目）。
 *
 * @param remainingAttempts 剩余可重试次数
 */
class PasswordWrongException(val remainingAttempts: Int) :
    Exception("PasswordWrong(remaining=$remainingAttempts)")

/**
 * 账号已被锁定（HTTP 42910）。
 *
 * 缺陷 #1 修复：服务端返回字段为 `locked_remaining_sec`（秒），不是
 * `remaining_minutes`，因此参数改为 [remainingSeconds]。UI 显示分钟时
 * 自行做 `ceil(secs / 60)` 转换。
 *
 * 缺陷 #4 修复：message 不再使用中文字面量；UI 通过 R.string.* 渲染。
 *
 * @param remainingSeconds 距解锁剩余秒数
 */
class PasswordLockedException(val remainingSeconds: Int) :
    Exception("PasswordLocked(remainingSec=$remainingSeconds)")

/**
 * 房间不存在（HTTP 40400）。
 *
 * 缺陷 #4 修复：移除中文 message，UI 通过 R.string.hall_room_not_found 显示。
 */
class RoomNotFoundException : Exception("RoomNotFound")
