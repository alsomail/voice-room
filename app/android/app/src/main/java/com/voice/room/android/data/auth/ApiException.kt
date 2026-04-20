package com.voice.room.android.data.auth

/**
 * 服务端业务错误（code ≠ 0）或 HTTP 4xx 错误码的领域异常
 *
 * @param code    对应 protocol.md §1.4 的错误码（如 40103 = 验证码错误）
 * @param message 服务端返回的英文错误描述
 */
class ApiException(val code: Int, message: String) : Exception(message)
