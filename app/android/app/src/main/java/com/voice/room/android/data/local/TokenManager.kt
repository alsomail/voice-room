package com.voice.room.android.data.local

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import com.voice.room.android.domain.local.ITokenManager
import kotlinx.coroutines.flow.firstOrNull
import kotlinx.coroutines.flow.map

/**
 * [ITokenManager] 的 DataStore 实现
 *
 * - 使用 [DataStore<Preferences>] 持久化 JWT Token
 * - 通过构造注入 [DataStore]，方便测试时使用内存 / 临时文件版本
 *
 * 创建方式（Application 级别）：
 * ```kotlin
 * val Context.authDataStore by preferencesDataStore(name = "auth")
 * val tokenManager = TokenManager(context.authDataStore)
 * ```
 */
class TokenManager(
    private val dataStore: DataStore<Preferences>
) : ITokenManager {

    companion object {
        private val TOKEN_KEY = stringPreferencesKey("jwt_token")
    }

    override suspend fun saveToken(token: String) {
        dataStore.edit { prefs ->
            prefs[TOKEN_KEY] = token
        }
    }

    override suspend fun getToken(): String? =
        dataStore.data
            .map { prefs -> prefs[TOKEN_KEY] }
            .firstOrNull()

    override suspend fun clearToken() {
        dataStore.edit { prefs ->
            prefs.remove(TOKEN_KEY)
        }
    }
}
