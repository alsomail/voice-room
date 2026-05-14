package com.voice.room.android.feature.noble

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.domain.nobility.NobleTier
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun NobleCenterScreen(
    container: AppContainer,
    onBack: () -> Unit
) {
    val viewModel: NobleCenterViewModel = viewModel(
        factory = NobleCenterViewModel.factory(container.nobilityRepository)
    )
    val state by viewModel.uiState.collectAsStateWithLifecycle()

    LaunchedEffect(Unit) { viewModel.loadMyNoble() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Noble Center") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(modifier = Modifier.padding(padding)) {
            if (state.isLoadingTiers) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(color = MenaColors.Primary)
                }
            } else if (state.error != null) {
                Text(state.error!!, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(16.dp))
            } else if (state.tiers.isNotEmpty()) {
                // Current noble status
                if (state.currentNoble != null) {
                    Card(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        colors = CardDefaults.cardColors(containerColor = MenaColors.Surface)
                    ) {
                        Column(modifier = Modifier.padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("Current Noble", style = MaterialTheme.typography.labelMedium)
                            NobleBadge(tierLevel = state.currentNoble!!.level, userId = "self")
                            Spacer(Modifier.height(4.dp))
                            Text(
                                state.currentNoble!!.tierName,
                                fontWeight = FontWeight.Bold,
                                fontSize = 18.sp,
                                color = MenaColors.Primary
                            )
                            Text("Expires: ${state.currentNoble!!.expireAt}", style = MaterialTheme.typography.labelSmall)
                        }
                    }
                }

                // Tier pager
                val pagerState = rememberPagerState(
                    initialPage = state.selectedTierIndex,
                    pageCount = { state.tiers.size }
                )

                HorizontalPager(
                    state = pagerState,
                    modifier = Modifier.fillMaxWidth().weight(1f).testTag("noble_tier_pager")
                ) { page ->
                    NobleTierCard(
                        tier = state.tiers[page],
                        isOwned = state.currentNoble?.tierId == state.tiers[page].tierId,
                        onPurchase = { viewModel.purchase(autoRenew = false) }
                    )
                }

                // Page indicator
                Row(
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                    horizontalArrangement = Arrangement.Center
                ) {
                    repeat(state.tiers.size) { idx ->
                        Box(
                            modifier = Modifier
                                .padding(4.dp)
                                .size(if (idx == pagerState.currentPage) 10.dp else 6.dp)
                                .clip(RoundedCornerShape(50))
                                .background(
                                    if (idx == pagerState.currentPage) MenaColors.Primary
                                    else MenaColors.Primary.copy(alpha = 0.3f)
                                )
                        )
                    }
                }

                // Purchase button
                Button(
                    onClick = { viewModel.purchase(autoRenew = true) },
                    enabled = !state.isLoadingPurchase,
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MenaColors.Primary)
                ) {
                    Text(
                        if (state.isLoadingPurchase) "Processing..." else "Purchase",
                        color = MenaColors.Background,
                        fontWeight = FontWeight.Bold
                    )
                }
            }
        }
    }
}

@Composable
private fun NobleTierCard(tier: NobleTier, isOwned: Boolean, onPurchase: () -> Unit) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 24.dp, vertical = 8.dp)
            .testTag("noble_tier_card_${tier.tierId}"),
        colors = CardDefaults.cardColors(containerColor = MenaColors.Surface)
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            NobleBadge(tierLevel = tier.level, userId = tier.tierId)
            Spacer(Modifier.height(12.dp))
            Text(tier.nameEn, fontWeight = FontWeight.Bold, fontSize = 20.sp, color = MenaColors.Primary)
            Text(tier.nameAr, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(16.dp))
            Text("${tier.monthlyDiamonds} 💎 / month", fontWeight = FontWeight.SemiBold)
            Text("$${tier.monthlyUsd} USD", style = MaterialTheme.typography.bodySmall)
            Spacer(Modifier.height(16.dp))
            if (isOwned) {
                Text("OWNED", color = MenaColors.Primary, fontWeight = FontWeight.Bold, fontSize = 16.sp)
            } else {
                Button(onClick = onPurchase, colors = ButtonDefaults.buttonColors(containerColor = MenaColors.Primary)) {
                    Text("Get", color = MenaColors.Background)
                }
            }
        }
    }
}
