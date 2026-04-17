package com.voice.room.android.data.wallet

import com.voice.room.android.domain.wallet.IWalletRepository

class DebugWalletRepository : IWalletRepository {
    override fun walletPreviewLabel(): String = "Wallet module reserved"
}
