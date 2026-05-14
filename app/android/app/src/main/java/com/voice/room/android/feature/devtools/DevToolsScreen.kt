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

/**
 * DevToolsScreen — 开发者工具（仅 dev/staging） (T-30065)
 *
 * 入口：个人中心连击版本号 7 下。
 * UI 顶部黄条提示"仅限开发环境 / 不影响财务报表"。
 * production 环境不渲染（BuildConfig check）。
 * 使用 Retrofit PaymentApiService（带 AuthInterceptor）请求 mock API。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DevToolsScreen(
    container: AppContainer,
    onBack: () -> Unit
) {
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

            // Mock Recharge buttons via IPaymentRepository
            val paymentRepo = container.paymentRepository

            Button(
                onClick = {
                    scope.launch {
                        try {
                            // Use existing payment order flow (dev SKU)
                            paymentRepo.createOrder("diamond_300")
                                .onSuccess {
                                    statusMessage = "✅ Mock order created: ${it.orderId}"
                                }
                                .onFailure { e ->
                                    statusMessage = "❌ ${e.message}"
                                }
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
                Text("Create Dev Order (diamond_300)")
            }

            Button(
                onClick = {
                    scope.launch {
                        statusMessage = "Mock purchase flow — use BillingPort Fake"
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp)
                    .testTag("debug_mock_recharge_fail")
            ) {
                Text("Dev: Test Verify (with FakeBillingPort)")
            }

            Button(
                onClick = {
                    scope.launch {
                        paymentRepo.listSkus()
                            .onSuccess { skus ->
                                statusMessage = "✅ Loaded ${skus.size} SKUs: ${skus.map { it.skuId }}"
                            }
                            .onFailure { e ->
                                statusMessage = "❌ ${e.message}"
                            }
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp)
                    .testTag("debug_mock_recharge_pending")
            ) {
                Text("Dev: List SKUs")
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
