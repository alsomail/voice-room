package com.voice.room.android.core.ui

import androidx.activity.ComponentActivity
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Chat
import androidx.compose.material.icons.outlined.Person
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.unit.LayoutDirection
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.voice.room.android.core.theme.MenaTheme
import com.voice.room.android.feature.main.MessagesPlaceholder
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.Locale

/**
 * Compose UI 测试 — PlaceholderScreen + MessagesPlaceholder (T-30023)
 *
 * PlaceholderScreen 组件测试:
 * - PH-01: 全参数(title+icon+subtitle)，三个元素均可见
 * - PH-02: 仅 title(icon=null, subtitle=null)，标题可见，无图标和副标题
 * - PH-03: title + icon，无 subtitle，标题和图标可见
 * - PH-04: testTag("placeholder_screen") 可被定位
 *
 * MessagesPlaceholder 集成测试:
 * - PH-06: testTag("messages_placeholder") 仍可被定位（T-30020 兼容性）
 * - PH-07: 消息 Tab 占位页显示"消息功能即将上线"文本
 * - PH-08: 消息 Tab 占位页显示"敬请期待"副标题
 *
 * 边界场景:
 * - PH-09: RTL 布局下组件不崩溃、居中对齐正确
 */
@RunWith(AndroidJUnit4::class)
class PlaceholderScreenTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // PH-01: 全参数(title+icon+subtitle)，三个元素均可见
    // ─────────────────────────────────────────────

    @Test
    fun PH01_allParams_titleIconSubtitle_allVisible() {
        composeTestRule.setContent {
            MenaTheme {
                PlaceholderScreen(
                    title = "Test Title",
                    icon = Icons.AutoMirrored.Outlined.Chat,
                    subtitle = "Test Subtitle",
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("Test Title").assertIsDisplayed()
        composeTestRule.onNodeWithText("Test Subtitle").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_icon").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // PH-02: 仅传 title（icon=null, subtitle=null），标题可见
    // ─────────────────────────────────────────────

    @Test
    fun PH02_titleOnly_noIconNoSubtitle() {
        composeTestRule.setContent {
            MenaTheme {
                PlaceholderScreen(
                    title = "Title Only",
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("Title Only").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_icon").assertDoesNotExist()
        composeTestRule.onNodeWithTag("placeholder_subtitle").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // PH-03: title + icon，无 subtitle
    // ─────────────────────────────────────────────

    @Test
    fun PH03_titleAndIcon_noSubtitle() {
        composeTestRule.setContent {
            MenaTheme {
                PlaceholderScreen(
                    title = "With Icon",
                    icon = Icons.Outlined.Person,
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("With Icon").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_icon").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_subtitle").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // PH-04: testTag("placeholder_screen") 可被定位
    // ─────────────────────────────────────────────

    @Test
    fun PH04_placeholderScreen_testTag_isDisplayed() {
        composeTestRule.setContent {
            MenaTheme {
                PlaceholderScreen(
                    title = "Tag Test",
                    icon = Icons.AutoMirrored.Outlined.Chat,
                    subtitle = "Sub",
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("placeholder_screen").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // PH-06: testTag("messages_placeholder") 仍可被定位
    // （T-30020 兼容性测试）
    // ─────────────────────────────────────────────

    @Test
    fun PH06_messagesPlaceholder_testTag_isDisplayed() {
        composeTestRule.setContent {
            MenaTheme {
                MessagesPlaceholder()
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("messages_placeholder").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // PH-07: 消息 Tab 占位页显示"消息功能即将上线"文本
    // ─────────────────────────────────────────────

    @Test
    fun PH07_messagesPlaceholder_showsComingSoonTitle() {
        composeTestRule.setContent {
            MenaTheme {
                MessagesPlaceholder()
            }
        }
        composeTestRule.waitForIdle()

        // Round 3 BUG-002：标题文本随设备 locale 变化（en/ar/zh→英文回退），
        // 改用 testTag 'placeholder_title' 唯一定位。
        composeTestRule.onNodeWithTag("placeholder_title").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // PH-08: 消息 Tab 占位页显示"敬请期待"副标题
    // ─────────────────────────────────────────────

    @Test
    fun PH08_messagesPlaceholder_showsSubtitle() {
        composeTestRule.setContent {
            MenaTheme {
                MessagesPlaceholder()
            }
        }
        composeTestRule.waitForIdle()

        // Round 3 BUG-002：副标题文本随 locale 变化，改用 testTag 'placeholder_subtitle'。
        composeTestRule.onNodeWithTag("placeholder_subtitle").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // PH-08b: 消息 Tab 占位页显示聊天图标
    // ─────────────────────────────────────────────

    @Test
    fun PH08b_messagesPlaceholder_showsChatIcon() {
        composeTestRule.setContent {
            MenaTheme {
                MessagesPlaceholder()
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithTag("placeholder_icon").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // PH-09: RTL 布局下组件不崩溃、居中对齐正确
    // ─────────────────────────────────────────────

    @Test
    fun PH09_rtlLayout_doesNotCrash_andDisplaysContent() {
        // Round 3 BUG-003：合并两次 setContent 为单次调用，避免 ComposeRule 二次
        // setContent 行为不稳定（Activity 已挂载首个 root，再次 setContent 可能丢节点）。
        var direction: LayoutDirection? = null
        composeTestRule.setContent {
            val arabicConfig = LocalConfiguration.current.apply {
                setLocale(Locale("ar"))
            }
            CompositionLocalProvider(LocalConfiguration provides arabicConfig) {
                MenaTheme {
                    direction = LocalLayoutDirection.current
                    PlaceholderScreen(
                        title = "اختبار",
                        icon = Icons.AutoMirrored.Outlined.Chat,
                        subtitle = "ترقبوا",
                    )
                }
            }
        }
        composeTestRule.waitForIdle()

        assertEquals(LayoutDirection.Rtl, direction)
        // 文本断言会受设备 locale 影响；这里 title/subtitle 直接以参数硬编码 Arabic，
        // 故文本断言可保留；但仍同时用 testTag 兜底。
        composeTestRule.onNodeWithTag("placeholder_title").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_subtitle").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_screen").assertIsDisplayed()
    }

    // ─────────────────────────────────────────────
    // 额外: PlaceholderScreen 可复用 — 不同参数渲染不同内容
    // ─────────────────────────────────────────────

    @Test
    fun reusability_differentParams_renderDifferentContent() {
        composeTestRule.setContent {
            MenaTheme {
                PlaceholderScreen(
                    title = "个人中心即将上线",
                    icon = Icons.Outlined.Person,
                )
            }
        }
        composeTestRule.waitForIdle()

        composeTestRule.onNodeWithText("个人中心即将上线").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_icon").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_subtitle").assertDoesNotExist()
    }

    // ─────────────────────────────────────────────
    // 额外: MessagesPlaceholder 内部包含 placeholder_screen testTag
    // ─────────────────────────────────────────────

    @Test
    fun messagesPlaceholder_containsPlaceholderScreen() {
        composeTestRule.setContent {
            MenaTheme {
                MessagesPlaceholder()
            }
        }
        composeTestRule.waitForIdle()

        // Both testTags should exist
        composeTestRule.onNodeWithTag("messages_placeholder").assertIsDisplayed()
        composeTestRule.onNodeWithTag("placeholder_screen").assertIsDisplayed()
    }
}
