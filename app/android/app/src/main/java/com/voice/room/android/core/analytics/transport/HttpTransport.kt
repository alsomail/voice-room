package com.voice.room.android.core.analytics.transport

import com.google.gson.Gson
import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.analytics.wire.EventWire
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

/**
 * HTTP fallback 事件上报实现（T-30035）
 *
 * WS 离线时使用此传输，直接 POST 到 analytics 端点。
 * 端点来自 BuildConfig.ANALYTICS_ENDPOINT。
 *
 * R1 修复（缺陷 1）：请求体严格遵循服务端 `EventInput` schema，使用 [EventWire] 单一事实源；
 * 顶层为 `{ "events": [...] }`，每个事件包含独立公共字段（device_id 必填）；
 * properties 为对象而非字符串，避免 JSONB 类型污染。
 */
class HttpTransport(
    private val httpClient: OkHttpClient,
    private val endpoint: String,
    private val gson: Gson = Gson()
) : Transport {

    private val mediaType = "application/json; charset=utf-8".toMediaType()

    override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> {
        return try {
            val body = EventWire.toHttpBody(batch, gson)
            val json = gson.toJson(body)
            val requestBody = json.toRequestBody(mediaType)
            val request = Request.Builder()
                .url(endpoint)
                .post(requestBody)
                .build()

            withContext(Dispatchers.IO) {
                val response = httpClient.newCall(request).execute()
                response.use {
                    if (it.isSuccessful) {
                        Result.success(SendOutcome(batch.map { e -> e.id }))
                    } else {
                        Result.failure(HttpException(it.code, it.message))
                    }
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    class HttpException(val code: Int, message: String) : Exception("HTTP $code: $message")
}
