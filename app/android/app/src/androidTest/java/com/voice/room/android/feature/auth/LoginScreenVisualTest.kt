package com.voice.room.android.feature.auth

import androidx.activity.ComponentActivity
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.core.theme.MenaTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Compose UI 测试 — LoginScreen 视觉升级 (T-30021)
 *
 * 视觉验收：
 * - VU-01: 渐变背景（不再使用白色 Surface）
 * - VU-02/03: GoldOutlinedTextField 替代 PhoneInput / CodeInput
 * - VU-04/05: GoldButton 替代 CountdownButton / 登录按钮
 * - VU-06/07: 标题颜色
 *
 * 功能回归：
 * - RG-02: 手机号输入仍仅接受数字、最多 9 位
 * - RG-03: 验证码输入仅接受数字、最多 6 位
 * - RG-04: 发送验证码按钮禁用/启用逻辑
 * - RG-05: 登录按钮禁用/启用逻辑
 * - RG-06: RTL 布局支持
 *
 * 边界用例：
 * - EC-01: 空状态 MenaTheme 下正常渲染
 * - EC-02: 倒计时状态 MenaTheme 下正常渲染
 * - EC-03: GoldButton(enabled=false) 不可点击
 */
@RunWith(AndroidJUnit4::class)
class LoginScreenVisualTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ═══════════════════════════════════════════════
    // VU-06: 主标题 "Voice Room" 可见（MenaTheme 下）
    // ═══════════════════════════════════════════════

    @Test
    fun VU06_mainTitle_isDisplayedInMenaTheme() {
        composeTestRule.setContent {
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
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Voice Room").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // VU-07: 副标题 "تسجيل الدخول" 可见（MenaTheme 下）
    // ═══════════════════════════════════════════════

    @Test
    fun VU07_subtitle_isDisplayedInMenaTheme() {
        composeTestRule.setContent {
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
        composeTestRule.waitForIdle()
        // 副标题 "تسجيل الدخول" 在标题区域可见
        composeTestRule.onAllNodes(hasText("تسجيل الدخول"))[0].assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // VU-02: PhoneInput 渲染 GoldOutlinedTextField
    //        验证通过 placeholder "5XXXXXXXX" 可见确认组件存在
    // ═══════════════════════════════════════════════

    @Test
    fun VU02_phoneInput_displaysPlaceholder() {
        composeTestRule.setContent {
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
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("5XXXXXXXX").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // VU-03: CodeInput 渲染 GoldOutlinedTextField
    //        验证通过 placeholder "------" 可见确认组件存在
    // ═══════════════════════════════════════════════

    @Test
    fun VU03_codeInput_displaysPlaceholder() {
        composeTestRule.setContent {
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
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("------").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // VU-04: CountdownButton 使用 GoldButton
    //        验证通过文本 "إرسال رمز التحقق" 可见确认组件存在
    // ═══════════════════════════════════════════════

    @Test
    fun VU04_countdownButton_displaysText() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(phoneNumber = "501234567"),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("إرسال رمز التحقق").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // VU-05: 登录按钮使用 GoldButton
    //        验证通过按钮文本可见确认组件存在
    // ═══════════════════════════════════════════════

    @Test
    fun VU05_loginButton_displaysText() {
        composeTestRule.setContent {
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
        composeTestRule.waitForIdle()
        // 登录按钮文字 "تسجيل الدخول" 可见（与副标题文字相同，取按钮节点）
        composeTestRule.onAllNodes(hasText("تسجيل الدخول"))[1].assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // RG-02: 手机号输入仍仅接受数字、最多 9 位
    // ═══════════════════════════════════════════════

    @Test
    fun RG02_phoneInput_acceptsOnlyDigits_max9() {
        var lastPhone = ""
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(),
                    onPhoneNumberChanged = { lastPhone = it },
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()

        // 输入混合字符 "abc123def456789012"
        composeTestRule.onNodeWithText("رقم الهاتف").performTextInput("abc123def456789012")
        composeTestRule.waitForIdle()

        // 过滤后应只保留数字，最多 9 位 → "123456789"
        assertEquals("123456789", lastPhone)
    }

    // ═══════════════════════════════════════════════
    // RG-03: 验证码输入仅接受数字、最多 6 位
    // ═══════════════════════════════════════════════

    @Test
    fun RG03_codeInput_acceptsOnlyDigits_max6() {
        var lastCode = ""
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = { lastCode = it },
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()

        // 输入混合字符 "abc1234567890"
        composeTestRule.onNodeWithText("رمز التحقق").performTextInput("abc1234567890")
        composeTestRule.waitForIdle()

        // 过滤后应只保留数字，最多 6 位 → "123456"
        assertEquals("123456", lastCode)
    }

    // ═══════════════════════════════════════════════
    // RG-04: 发送验证码按钮禁用/启用逻辑不变
    // ═══════════════════════════════════════════════

    @Test
    fun RG04_sendButton_disabled_whenPhoneEmpty() {
        var sendClicked = false
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(phoneNumber = ""),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = { sendClicked = true },
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()

        // 发送按钮应可见但 disabled
        val sendBtn = composeTestRule.onNodeWithText("إرسال رمز التحقق")
        sendBtn.assertIsDisplayed()
        sendBtn.assertIsNotEnabled()
        sendBtn.performClick()
        composeTestRule.waitForIdle()
        assertFalse("Send button click should be ignored when disabled", sendClicked)
    }

    @Test
    fun RG04_sendButton_enabled_whenPhoneValid() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(phoneNumber = "501234567", countdownSeconds = 0),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("إرسال رمز التحقق").assertIsEnabled()
    }

    @Test
    fun RG04_sendButton_disabled_duringCountdown() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(phoneNumber = "501234567", countdownSeconds = 30),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        // 倒计时中显示 "30s"
        composeTestRule.onNodeWithText("30s").assertIsDisplayed()
        composeTestRule.onNodeWithText("30s").assertIsNotEnabled()
    }

    // ═══════════════════════════════════════════════
    // RG-05: 登录按钮禁用/启用逻辑不变
    // ═══════════════════════════════════════════════

    @Test
    fun RG05_loginButton_disabled_whenCodeMissing() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(
                        phoneNumber = "501234567",
                        verificationCode = ""
                    ),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        // 登录按钮（第二个 "تسجيل الدخول"）应 disabled
        composeTestRule.onAllNodes(hasText("تسجيل الدخول"))[1].assertIsNotEnabled()
    }

    @Test
    fun RG05_loginButton_enabled_whenPhoneAndCodeValid() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(
                        phoneNumber = "501234567",
                        verificationCode = "123456"
                    ),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        // 登录按钮（第二个 "تسجيل الدخول"）应 enabled
        composeTestRule.onAllNodes(hasText("تسجيل الدخول"))[1].assertIsEnabled()
    }

    @Test
    fun RG05_loginButton_clickTriggersCallback() {
        var loginClicked = false
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(
                        phoneNumber = "501234567",
                        verificationCode = "123456"
                    ),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = { loginClicked = true }
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onAllNodes(hasText("تسجيل الدخول"))[1].performClick()
        composeTestRule.waitForIdle()
        assertTrue("Login button click should trigger callback", loginClicked)
    }

    // ═══════════════════════════════════════════════
    // RG-06: RTL 布局支持
    // ═══════════════════════════════════════════════

    @Test
    fun RG06_rtlLayout_rendersWithoutCrash() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(isRtlLayout = true),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Voice Room").assertIsDisplayed()
    }

    @Test
    fun RG06_ltrLayout_rendersWithoutCrash() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(isRtlLayout = false),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Voice Room").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // EC-01: 空状态 MenaTheme 下正常渲染
    // ═══════════════════════════════════════════════

    @Test
    fun EC01_emptyState_rendersInMenaTheme() {
        composeTestRule.setContent {
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
        composeTestRule.waitForIdle()
        // 所有核心元素都可见
        composeTestRule.onNodeWithText("🎙️").assertIsDisplayed()
        composeTestRule.onNodeWithText("Voice Room").assertIsDisplayed()
        composeTestRule.onNodeWithText("+966").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // EC-02: 倒计时状态 MenaTheme 下正常渲染
    // ═══════════════════════════════════════════════

    @Test
    fun EC02_countdownState_rendersInMenaTheme() {
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(
                        phoneNumber = "501234567",
                        countdownSeconds = 42
                    ),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        // 倒计时标签可见
        composeTestRule.onNodeWithText("42s").assertIsDisplayed()
    }

    // ═══════════════════════════════════════════════
    // EC-03: GoldButton(enabled=false) 不可点击
    // ═══════════════════════════════════════════════

    @Test
    fun EC03_disabledLoginButton_notClickable() {
        var loginClicked = false
        composeTestRule.setContent {
            MenaTheme {
                LoginScreenContent(
                    uiState = LoginUiState(
                        phoneNumber = "501234567",
                        verificationCode = ""  // 不满足 6 位 → disabled
                    ),
                    onPhoneNumberChanged = {},
                    onVerificationCodeChanged = {},
                    onSendCode = {},
                    onLogin = { loginClicked = true }
                )
            }
        }
        composeTestRule.waitForIdle()
        val loginBtn = composeTestRule.onAllNodes(hasText("تسجيل الدخول"))[1]
        loginBtn.assertIsNotEnabled()
        loginBtn.performClick()
        composeTestRule.waitForIdle()
        assertFalse("Disabled login button should not trigger callback", loginClicked)
    }
}
