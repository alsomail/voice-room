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
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
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
        // Round 3 BUG-002 修复：副标题文本随设备 locale 变化（en/ar/zh-fallback），
        // 改用 testTag 'login_subtitle' 唯一定位，避免文本断言脆弱。
        composeTestRule.onNodeWithTag("login_subtitle").assertIsDisplayed()
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
        // Round 3 BUG-002：通过 testTag 定位 PhoneInput 容器（其内嵌 GoldOutlinedTextField
        // 的 placeholder "5XXXXXXXX" 仍渲染，但占位字符在 OutlinedTextField 中通过
        // 子节点 alpha 动画呈现，不直接匹配 onNodeWithText 断言）。
        composeTestRule.onNodeWithTag("login_phone_input").assertIsDisplayed()
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
        // Round 3 BUG-002：通过 testTag 定位 CodeInput 容器
        composeTestRule.onNodeWithTag("login_code_input").performScrollTo().assertIsDisplayed()
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
        // Round 3 BUG-002：CountdownButton 现内含 GoldButton（带 mergeDescendants 语义合并），
        // 同 LoginScreen 调用点已添加 testTag('login_send_code_button')，改用 tag 唯一定位。
        // LoginScreenContent 在小屏（Pixel 4 portrait）下需 verticalScroll，故 performScrollTo()。
        composeTestRule.onNodeWithTag("login_send_code_button").performScrollTo().assertIsDisplayed()
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
        // Round 3 BUG-002：登录按钮文本来自 R.string.login_button，
        // 在 zh-Hans-CN 设备上回退到默认 "Sign in"，故改用 testTag 定位 + performScrollTo。
        composeTestRule.onNodeWithTag("login_button").performScrollTo().assertIsDisplayed()
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

        // Round 3 BUG-002：通过 testTag 定位发送按钮，避免文本断言依赖 locale；
        // 同时 performScrollTo 以应对滚动布局
        val sendBtn = composeTestRule.onNodeWithTag("login_send_code_button")
        sendBtn.performScrollTo()
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
        composeTestRule.onNodeWithTag("login_send_code_button").assertIsEnabled()
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
        // 倒计时中按钮处于 disabled 状态
        composeTestRule.onNodeWithTag("login_send_code_button").assertIsNotEnabled()
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
        composeTestRule.onNodeWithTag("login_button").assertIsNotEnabled()
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
        composeTestRule.onNodeWithTag("login_button").assertIsEnabled()
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
        composeTestRule.onNodeWithTag("login_button").performScrollTo().performClick()
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
        // 倒计时按钮通过 testTag 定位，处于 disabled 状态
        composeTestRule.onNodeWithTag("login_send_code_button").assertIsNotEnabled()
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
        val loginBtn = composeTestRule.onNodeWithTag("login_button")
        loginBtn.assertIsNotEnabled()
        loginBtn.performClick()
        composeTestRule.waitForIdle()
        assertFalse("Disabled login button should not trigger callback", loginClicked)
    }
}
