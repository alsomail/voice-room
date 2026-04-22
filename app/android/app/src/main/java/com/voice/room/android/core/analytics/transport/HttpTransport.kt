package com.voice.room.android.core.analytics.transport

import com.google.gson.Gson
import com.voice.room.android.core.analytics.queue.EventQueueEntity
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
 */
class HttpTransport(
    private val httpClient: OkHttpClient,
    private val endpoint: String,
    private val gson: Gson = Gson()
) : Transport {

    private val mediaType = "application/json; charset=utf-8".toMediaType()

    override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> {
        return try {
            val payload = batch.map { entity ->
                mapOf(
                    "event_name" to entity.eventName,
                    "properties" to entity.propertiesJson,
                    "session_id" to entity.sessionId,
                    "client_ts" to entity.clientTs
                )
            }
            val json = gson.toJson(mapOf("events" to payload))
            val body = json.toRequestBody(mediaType)
            val request = Request.Builder()
                .url(endpoint)
                .post(body)
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
