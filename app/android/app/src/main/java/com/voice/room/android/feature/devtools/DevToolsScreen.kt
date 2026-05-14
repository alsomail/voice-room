package com.voice.room.android.feature.devtools

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.BuildConfig
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.MenaColors
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaType

/**
 * DevToolsScreen — 开发者工具（仅 dev/staging） (T-30065)
 *
 * 入口：个人中心连击版本号 7 下。
 * UI 顶部黄条提示"仅限开发环境 / 不影响财务报表"。
 * production 环境不渲染（BuildConfig check）。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DevToolsScreen(
    container: AppContainer,
    onBack: () -> Unit
) {
    // Safety: production 环境不渲染
    if (BuildConfig.APP_ENVIRONMENT == "production") {
        onBack()
        return
    }

    val scope = rememberCoroutineScope()
    var statusMessage by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Dev Tools") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
        ) {
            // Warning banner
            Surface(
                color = MenaColors.Primary.copy(alpha = 0.2f),
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    "DEV ONLY — Does not affect production data",
                    modifier = Modifier.padding(12.dp),
                    color = MenaColors.Primary,
                    style = MaterialTheme.typography.labelMedium
                )
            }

            Spacer(Modifier.height(16.dp))

            // Mock Recharge — Success
            Button(
                onClick = {
                    scope.launch {
                        try {
                            // Call T-00055 mock API
                            val httpClient = okhttp3.OkHttpClient()
                            val request = okhttp3.Request.Builder()
                                .url("${BuildConfig.API_BASE_URL}/v1/_dev/mock_recharge")
                                .post(
                                    okhttp3.RequestBody.create(
                                        "application/json".toMediaType(),
                                        """{"user_id":"dev-user","sku_id":"diamond_300","force_outcome":"success"}"""
                                    )
                                )
                                .build()
                            httpClient.newCall(request).execute()
                            statusMessage = "✅ Mock recharge SUCCESS"
                        } catch (e: Exception) {
                            statusMessage = "❌ ${e.message}"
                        }
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp)
                    .testTag("debug_mock_recharge_success")
            ) {
                Text("Mock Recharge — Success")
            }

            // Mock Recharge — Fail
            Button(
                onClick = {
                    scope.launch {
                        try {
                            val httpClient = okhttp3.OkHttpClient()
                            val request = okhttp3.Request.Builder()
                                .url("${BuildConfig.API_BASE_URL}/v1/_dev/mock_recharge")
                                .post(
                                    okhttp3.RequestBody.create(
                                        "application/json".toMediaType(),
                                        """{"user_id":"dev-user","sku_id":"diamond_300","force_outcome":"fail"}"""
                                    )
                                )
                                .build()
                            httpClient.newCall(request).execute()
                            statusMessage = "✅ Mock recharge FAIL (as expected)"
                        } catch (e: Exception) {
                            statusMessage = "❌ ${e.message}"
                        }
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp)
                    .testTag("debug_mock_recharge_fail")
            ) {
                Text("Mock Recharge — Fail")
            }

            // Mock Recharge — Pending
            Button(
                onClick = {
                    scope.launch {
                        try {
                            val httpClient = okhttp3.OkHttpClient()
                            val request = okhttp3.Request.Builder()
                                .url("${BuildConfig.API_BASE_URL}/v1/_dev/mock_recharge")
                                .post(
                                    okhttp3.RequestBody.create(
                                        "application/json".toMediaType(),
                                        """{"user_id":"dev-user","sku_id":"diamond_300","force_outcome":"pending"}"""
                                    )
                                )
                                .build()
                            httpClient.newCall(request).execute()
                            statusMessage = "✅ Mock recharge PENDING"
                        } catch (e: Exception) {
                            statusMessage = "❌ ${e.message}"
                        }
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp)
                    .testTag("debug_mock_recharge_pending")
            ) {
                Text("Mock Recharge — PENDING")
            }

            if (statusMessage != null) {
                Spacer(Modifier.height(16.dp))
                Text(
                    statusMessage!!,
                    modifier = Modifier.padding(horizontal = 16.dp),
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }
    }
}
