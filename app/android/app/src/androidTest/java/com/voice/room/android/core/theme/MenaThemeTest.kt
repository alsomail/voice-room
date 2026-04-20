package com.voice.room.android.core.theme

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.Locale

/**
 * Compose UI 测试 — MenaTheme (T-30018)
 *
 * MT-01: MenaTheme 内部 MaterialTheme.colorScheme.background == MenaColors.Background
 * MT-02: MenaTheme 内部 MaterialTheme.colorScheme.primary == MenaColors.Primary
 * MT-03: MenaTheme 应用 darkColorScheme（始终深色）
 * MT-04: MaterialTheme.typography 已应用（titleLarge.fontSize == 22.sp）
 * MT-05: MaterialTheme.shapes 已应用（large == RoundedCornerShape(16.dp)）
 * MT-06: 阿拉伯语 locale 下 LocalLayoutDirection.current == Rtl
 * MT-07: 英语 locale 下 LocalLayoutDirection.current == Ltr
 * MT-08: 嵌套 MenaTheme 不崩溃
 */
@RunWith(AndroidJUnit4::class)
class MenaThemeTest {

    @get:Rule
    val composeTestRule = createAndroidComposeRule<ComponentActivity>()

    // ─────────────────────────────────────────────
    // MT-01: background == MenaColors.Background
    // ─────────────────────────────────────────────

    @Test
    fun MT01_background_isMenaBackground() {
        var backgroundMatches = false
        composeTestRule.setContent {
            MenaTheme {
                backgroundMatches =
                    MaterialTheme.colorScheme.background == MenaColors.Background
                Text("MT01")
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(true, backgroundMatches)
    }

    // ─────────────────────────────────────────────
    // MT-02: primary == MenaColors.Primary
    // ─────────────────────────────────────────────

    @Test
    fun MT02_primary_isMenaPrimary() {
        var primaryMatches = false
        composeTestRule.setContent {
            MenaTheme {
                primaryMatches =
                    MaterialTheme.colorScheme.primary == MenaColors.Primary
                Text("MT02")
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(true, primaryMatches)
    }

    // ─────────────────────────────────────────────
    // MT-03: 始终使用 darkColorScheme（surface 应为深色）
    // ─────────────────────────────────────────────

    @Test
    fun MT03_alwaysDarkColorScheme() {
        var surfaceMatches = false
        composeTestRule.setContent {
            MenaTheme {
                surfaceMatches =
                    MaterialTheme.colorScheme.surface == MenaColors.Surface
                Text("MT03")
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(true, surfaceMatches)
    }

    // ─────────────────────────────────────────────
    // MT-04: titleLarge.fontSize == 22.sp
    // ─────────────────────────────────────────────

    @Test
    fun MT04_typography_titleLarge_22sp() {
        var fontSizeMatches = false
        composeTestRule.setContent {
            MenaTheme {
                fontSizeMatches =
                    MaterialTheme.typography.titleLarge.fontSize == 22.sp
                Text("MT04")
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(true, fontSizeMatches)
    }

    // ─────────────────────────────────────────────
    // MT-05: shapes.large == RoundedCornerShape(16.dp)
    // ─────────────────────────────────────────────

    @Test
    fun MT05_shapes_large_16dp() {
        var shapesMatch = false
        composeTestRule.setContent {
            MenaTheme {
                shapesMatch =
                    MaterialTheme.shapes.large == RoundedCornerShape(16.dp)
                Text("MT05")
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(true, shapesMatch)
    }

    // ─────────────────────────────────────────────
    // MT-06: 阿拉伯语 locale → RTL
    // ─────────────────────────────────────────────

    @Test
    fun MT06_arabicLocale_isRtl() {
        var direction: LayoutDirection? = null
        composeTestRule.setContent {
            val arabicConfig = LocalConfiguration.current.apply {
                setLocale(Locale("ar"))
            }
            CompositionLocalProvider(LocalConfiguration provides arabicConfig) {
                MenaTheme {
                    direction = LocalLayoutDirection.current
                    Text("MT06")
                }
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(LayoutDirection.Rtl, direction)
    }

    // ─────────────────────────────────────────────
    // MT-07: 英语 locale → LTR
    // ─────────────────────────────────────────────

    @Test
    fun MT07_englishLocale_isLtr() {
        var direction: LayoutDirection? = null
        composeTestRule.setContent {
            val englishConfig = LocalConfiguration.current.apply {
                setLocale(Locale("en"))
            }
            CompositionLocalProvider(LocalConfiguration provides englishConfig) {
                MenaTheme {
                    direction = LocalLayoutDirection.current
                    Text("MT07")
                }
            }
        }
        composeTestRule.waitForIdle()
        assertEquals(LayoutDirection.Ltr, direction)
    }

    // ─────────────────────────────────────────────
    // MT-08: 嵌套 MenaTheme 不崩溃
    // ─────────────────────────────────────────────

    @Test
    fun MT08_nestedMenaTheme_doesNotCrash() {
        composeTestRule.setContent {
            MenaTheme {
                MenaTheme {
                    Text("Nested MenaTheme works")
                }
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Nested MenaTheme works").assertIsDisplayed()
    }
}
