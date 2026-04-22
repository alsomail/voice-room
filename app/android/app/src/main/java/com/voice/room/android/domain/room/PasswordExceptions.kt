package com.voice.room.android.domain.room

/**
 * 密码验证失败（HTTP 40103）
 *
 * @param remainingAttempts 剩余可重试次数
 */
class PasswordWrongException(val remainingAttempts: Int) :
    Exception("密码错误，剩余 $remainingAttempts 次机会")

/**
 * 账号已被锁定（HTTP 42910）
 *
 * @param remainingMinutes 距解锁剩余分钟数
 */
class PasswordLockedException(val remainingMinutes: Int) :
    Exception("已被锁定，$remainingMinutes 分钟后重试")

/**
 * 房间不存在（HTTP 40400）
 */
class RoomNotFoundException : Exception("房间不存在")
