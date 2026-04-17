package com.voice.room.android.feature.room

import com.voice.room.android.common.FeatureDescriptor

object RoomFeature {
    val descriptor = FeatureDescriptor(
        title = "Room Skeleton",
        description = "Room state remains server-authoritative; the client only renders placeholders and receives authority later."
    )
}
