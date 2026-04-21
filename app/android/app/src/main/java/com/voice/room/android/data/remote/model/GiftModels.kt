package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 礼物列表 DTO (T-30028)
 *
 * 映射 `GET /api/v1/gifts/list` 响应中 data 数组的每个元素。
 */
data class GiftDto(
    @SerializedName("id")         val id: String,
    @SerializedName("code")       val code: String,
    @SerializedName("name")       val name: String,
    @SerializedName("icon_url")   val iconUrl: String,
    @SerializedName("price")      val price: Long,
    @SerializedName("sort_order") val sortOrder: Int,
    @SerializedName("tier")       val tier: Int,
)
