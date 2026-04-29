package com.voice.room.android.feature.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.voice.room.android.R
import com.voice.room.android.core.theme.GoldButton
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.core.theme.MenaTheme
import com.voice.room.android.feature.auth.components.CodeInput
import com.voice.room.android.feature.auth.components.CountdownButton
import com.voice.room.android.feature.auth.components.PhoneInput

/**
 * 登录页入口 Composable
 *
 * RTL 支持：当 [LoginUiState.isRtlLayout] 为 true 时（沙特默认），
 * 通过 [CompositionLocalProvider] 将 [LocalLayoutDirection] 设为 [LayoutDirection.Rtl]，
 * 所有子组件自动遵循 RTL 排列。
 *
 * @param onLoginSuccess 登录成功回调（由外部导航层传入）
 * @param viewModel      由 [viewModel] 工厂自动提供，测试时可注入 fake
 */
@Composable
fun LoginScreen(
    onLoginSuccess: () -> Unit = {},
    loginViewModel: LoginViewModel = viewModel()
) {
    val uiState by loginViewModel.uiState.collectAsState()

    // 缺陷 #8 修复：监听 ViewModel 的 navEvent 流，登录成功 → 调用 onLoginSuccess
    LaunchedEffect(Unit) {
        loginViewModel.navEvent.collect { event ->
            if (event is NavEvent.NavigateToHall) {
                onLoginSuccess()
            }
        }
    }

    LoginScreenContent(
        uiState = uiState,
        onPhoneNumberChanged = loginViewModel::onPhoneNumberChanged,
        onVerificationCodeChanged = loginViewModel::onVerificationCodeChanged,
        onSendCode = loginViewModel::onSendCode,
        // 缺陷 #8 修复：调用真实登录方法（保存 JWT + 发射 NavigateToHall），
        // 不再 stub 直跳；导航由上方 LaunchedEffect 在 navEvent 触发后执行。
        onLogin = loginViewModel::onLogin,
    )
}

/**
 * 纯 Stateless Composable – 易于 Preview 与测试。
 *
 * RTL 逻辑：
 * - [uiState.isRtlLayout] == true  → CompositionLocalProvider 注入 LayoutDirection.Rtl
 * - [uiState.isRtlLayout] == false → 使用系统默认方向（LTR）
 */
@Composable
fun LoginScreenContent(
    uiState: LoginUiState,
    onPhoneNumberChanged: (String) -> Unit,
    onVerificationCodeChanged: (String) -> Unit,
    onSendCode: () -> Unit,
    onLogin: () -> Unit,
    modifier: Modifier = Modifier
) {
    val layoutDirection = if (uiState.isRtlLayout) LayoutDirection.Rtl else LayoutDirection.Ltr

    CompositionLocalProvider(LocalLayoutDirection provides layoutDirection) {
        Box(
            modifier = modifier
                .fillMaxSize()
                .background(
                    brush = Brush.verticalGradient(
                        colors = listOf(MenaColors.Background, MenaColors.Surface)
                    )
                )
        ) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 24.dp, vertical = 32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {

                // ── 品牌 Logo / 标题区域 ──────────────────────────
                Spacer(modifier = Modifier.height(40.dp))

                Text(
                    text = stringResource(id = R.string.app_brand_emoji),
                    style = MaterialTheme.typography.displayLarge,
                    textAlign = TextAlign.Center
                )

                Text(
                    text = stringResource(id = R.string.app_brand_name),
                    style = MaterialTheme.typography.headlineLarge,
                    fontWeight = FontWeight.Bold,
                    color = MenaColors.OnBackground,
                    textAlign = TextAlign.Center
                )

                Text(
                    text = stringResource(id = R.string.login_subtitle),
                    style = MaterialTheme.typography.titleMedium,
                    color = MenaColors.Primary,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.testTag("login_subtitle"),
                )

                Spacer(modifier = Modifier.height(24.dp))

                // ── 手机号输入区域 ────────────────────────────────
                PhoneInput(
                    phoneNumber = uiState.phoneNumber,
                    onPhoneNumberChanged = onPhoneNumberChanged,
                    countryCode = uiState.defaultCountryCode,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("login_phone_input")
                )

                // ── 发送验证码按钮（含倒计时）─────────────────────
                CountdownButton(
                    isEnabled = uiState.isSendButtonEnabled,
                    isCountingDown = uiState.isCountingDown,
                    countdownLabel = uiState.countdownLabel,
                    onSendCode = onSendCode,
                    modifier = Modifier.testTag("login_send_code_button"),
                )

                Spacer(modifier = Modifier.height(8.dp))

                // ── 验证码输入区域 ────────────────────────────────
                CodeInput(
                    code = uiState.verificationCode,
                    onCodeChanged = onVerificationCodeChanged,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("login_code_input"),
                )

                Spacer(modifier = Modifier.height(8.dp))

                // ── 登录按钮 ──────────────────────────────────────
                GoldButton(
                    text = stringResource(id = R.string.login_button),
                    onClick = onLogin,
                    enabled = uiState.isLoginButtonEnabled,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(52.dp)
                        .testTag("login_button")
                )
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Compose Previews
// ─────────────────────────────────────────────────────────────────────────────

@Preview(
    name = "Login Screen – RTL (Arabic)",
    showBackground = true,
    locale = "ar"
)
@Composable
private fun LoginScreenRtlPreview() {
    MenaTheme {
        LoginScreenContent(
            uiState = LoginUiState(
                phoneNumber = "501234567",
                isRtlLayout = true
            ),
            onPhoneNumberChanged = {},
            onVerificationCodeChanged = {},
            onSendCode = {},
            onLogin = {}
        )
    }
}

@Preview(
    name = "Login Screen – Empty (RTL)",
    showBackground = true
)
@Composable
private fun LoginScreenEmptyPreview() {
    MenaTheme {
        LoginScreenContent(
            uiState = LoginUiState(),
            onPhoneNumberChanged = {},
            onVerificationCodeChanged = {},
            onSendCode = {},
            onLogin = {}
        )
    }
}

@Preview(
    name = "Login Screen – Countdown Active",
    showBackground = true
)
@Composable
private fun LoginScreenCountdownPreview() {
    MenaTheme {
        LoginScreenContent(
            uiState = LoginUiState(
                phoneNumber = "501234567",
                countdownSeconds = 42,
                isRtlLayout = true
            ),
            onPhoneNumberChanged = {},
            onVerificationCodeChanged = {},
            onSendCode = {},
            onLogin = {}
        )
    }
}

@Preview(
    name = "Login Screen – Ready to Login",
    showBackground = true
)
@Composable
private fun LoginScreenReadyPreview() {
    MenaTheme {
        LoginScreenContent(
            uiState = LoginUiState(
                phoneNumber = "501234567",
                verificationCode = "123456",
                countdownSeconds = 0,
                isRtlLayout = true
            ),
            onPhoneNumberChanged = {},
            onVerificationCodeChanged = {},
            onSendCode = {},
            onLogin = {}
        )
    }
}
