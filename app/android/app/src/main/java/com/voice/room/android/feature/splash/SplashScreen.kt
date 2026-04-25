package com.voice.room.android.feature.splash

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.voice.room.android.BuildConfig
import com.voice.room.android.R
import com.voice.room.android.core.theme.MenaColors
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Splash 启动页 Composable
 *
 * - 深色背景 (#1A1A2E)
 * - 居中 Logo 缩放 0.5→1.0 + alpha 0→1（800ms EaseOut）
 * - 底部版本号
 * - 动画完成后延迟 500ms，调用 SplashViewModel.checkAuth()
 * - 通过 LaunchedEffect 监听 navEvent 驱动导航
 *
 * testTag 协议：
 * - splash_screen — 整个 Splash 容器
 * - splash_logo   — Logo 图片
 * - splash_version — 版本号文字
 */
@Composable
fun SplashScreen(
    onNavigateToMain: () -> Unit,
    onNavigateToLogin: () -> Unit,
    splashViewModel: SplashViewModel = viewModel()
) {
    // ── 动画状态 ──────────────────────────────────────
    val scale = remember { Animatable(0.5f) }
    val alpha = remember { Animatable(0f) }

    // ── Logo 缩放 + 淡入动画，完成后 delay 500ms 再 checkAuth ──
    LaunchedEffect(Unit) {
        // 并行启动 scale 和 alpha 动画
        val scaleJob = launch {
            scale.animateTo(
                targetValue = 1f,
                animationSpec = tween(
                    durationMillis = 800,
                    easing = EaseOut
                )
            )
        }
        val alphaJob = launch {
            alpha.animateTo(
                targetValue = 1f,
                animationSpec = tween(
                    durationMillis = 800,
                    easing = EaseOut
                )
            )
        }
        scaleJob.join()
        alphaJob.join()
        delay(500L) // 总停留 ~1.3s
        splashViewModel.checkAuth()
    }

    // ── 监听导航事件 ──────────────────────────────────
    LaunchedEffect(Unit) {
        splashViewModel.navEvent.collect { event ->
            when (event) {
                is SplashNavEvent.NavigateToMain -> onNavigateToMain()
                is SplashNavEvent.NavigateToLogin -> onNavigateToLogin()
            }
        }
    }

    // ── UI ────────────────────────────────────────────
    Surface(
        modifier = Modifier
            .fillMaxSize()
            .testTag("splash_screen"),
        color = MenaColors.Background
    ) {
        Box(
            modifier = Modifier.fillMaxSize(),
            contentAlignment = Alignment.Center
        ) {
            // Logo — 居中，金色，120dp
            Image(
                painter = painterResource(id = R.drawable.ic_logo),
                contentDescription = stringResource(id = R.string.splash_logo_description),
                modifier = Modifier
                    .size(120.dp)
                    .scale(scale.value)
                    .alpha(alpha.value)
                    .testTag("splash_logo"),
                colorFilter = ColorFilter.tint(MenaColors.Primary)
            )

            // 版本号 — 底部居中
            Text(
                text = "v${BuildConfig.VERSION_NAME}",
                style = MaterialTheme.typography.bodySmall,
                color = MenaColors.OnBackground,
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = 32.dp)
                    .testTag("splash_version")
            )
        }
    }
}
