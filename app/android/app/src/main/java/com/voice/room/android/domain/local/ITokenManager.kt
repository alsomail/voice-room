package com.voice.room.android.domain.local

/**
 * JWT Token 存储接口（Domain 层契约）
 *
 * 生产实现基于 DataStore Preferences；测试时注入 Fake 实现，无需 Android Context。
 */
interface ITokenManager {

    /** 持久化保存 JWT Token */
    suspend fun saveToken(token: String)

    /** 读取已保存的 JWT Token；未登录时返回 null */
    suspend fun getToken(): String?

    /** 清除 Token（登出时调用） */
    suspend fun clearToken()
}
