package com.voice.room.android.feature.auth

import com.voice.room.android.common.FeatureDescriptor

object AuthFeature {
    val descriptor = FeatureDescriptor(
        title = "Auth Bootstrap",
        description = "Login, token refresh, and session binding will stay behind IAuthService."
    )
}
