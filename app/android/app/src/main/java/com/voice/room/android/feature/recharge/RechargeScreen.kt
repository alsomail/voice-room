package com.voice.room.android.feature.recharge

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.theme.GoldButton
import com.voice.room.android.domain.payment.SkuItem
import com.voice.room.android.core.theme.MenaColors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RechargeScreen(
    container: AppContainer,
    onBack: () -> Unit,
    onNavigateToHistory: () -> Unit
) {
    val viewModel: RechargeViewModel = viewModel(
        factory = RechargeViewModel.factory(container.paymentRepository, container.walletRepository)
    )
    val state by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Diamond Recharge") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        },
        bottomBar = {
            if (state.selectedSku != null) {
                Surface(tonalElevation = 3.dp) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column {
                            Text("Balance", style = MaterialTheme.typography.labelSmall)
                            Text(
                                "${state.balance} 💎",
                                style = MaterialTheme.typography.titleLarge,
                                fontWeight = FontWeight.Bold,
                                color = MenaColors.Primary
                            )
                        }
                        Button(
                            onClick = { viewModel.createOrderAndPay() },
                            enabled = !state.isCreatingOrder,
                            colors = ButtonDefaults.buttonColors(containerColor = MenaColors.Primary)
                        ) {
                            Text(
                                if (state.isCreatingOrder) "Loading..." else "Buy ${state.selectedSku!!.displayPriceUsd}",
                                color = MenaColors.Background
                            )
                        }
                    }
                }
            }
        }
    ) { padding ->
        Column(modifier = Modifier.padding(padding)) {
            // Balance header
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                colors = CardDefaults.cardColors(containerColor = MenaColors.Surface)
            ) {
                Column(
                    modifier = Modifier.padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Text("Current Balance", style = MaterialTheme.typography.labelMedium)
                    Text(
                        "${state.balance} 💎",
                        fontSize = 36.sp,
                        fontWeight = FontWeight.Bold,
                        color = MenaColors.Primary
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    TextButton(onClick = onNavigateToHistory) {
                        Text("Recharge History →", color = MenaColors.Primary)
                    }
                }
            }

            // Error
            if (state.error != null) {
                Text(
                    state.error!!,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 16.dp)
                )
            }

            // SKU Grid
            if (state.isLoadingSkus) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator(color = MenaColors.Primary)
                }
            } else {
                LazyVerticalGrid(
                    columns = GridCells.Fixed(2),
                    contentPadding = PaddingValues(16.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    items(state.skus) { sku ->
                        SkuCard(
                            sku = sku,
                            isSelected = state.selectedSku?.skuId == sku.skuId,
                            onClick = { viewModel.selectSku(sku) }
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SkuCard(sku: SkuItem, isSelected: Boolean, onClick: () -> Unit) {
    val borderColor = if (isSelected) MenaColors.Primary else MenaColors.Surface
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .border(2.dp, borderColor, RoundedCornerShape(16.dp))
            .clickable { onClick() },
        colors = CardDefaults.cardColors(
            containerColor = if (isSelected) MenaColors.Surface.copy(alpha = 0.8f) else MenaColors.Surface
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Tag
            if (sku.tag != null) {
                Text(
                    sku.tag.uppercase(),
                    color = MenaColors.Primary,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier
                        .background(MenaColors.Primary.copy(alpha = 0.15f), RoundedCornerShape(4.dp))
                        .padding(horizontal = 8.dp, vertical = 2.dp)
                )
                Spacer(modifier = Modifier.height(8.dp))
            }

            Text(
                "${sku.diamonds}",
                fontSize = 28.sp,
                fontWeight = FontWeight.Bold,
                color = MenaColors.Primary
            )
            Text("💎", fontSize = 20.sp)
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                sku.displayPriceUsd,
                fontSize = 18.sp,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface
            )
            if (sku.displayPriceLocal != null) {
                Text(
                    sku.displayPriceLocal,
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
