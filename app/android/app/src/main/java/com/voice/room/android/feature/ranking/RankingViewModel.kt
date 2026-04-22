package com.voice.room.android.feature.ranking

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.domain.ranking.IRankingRepository
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * RankingViewModel — 魅力/财富榜页 ViewModel (T-30033)
 *
 * 职责：
 * - init 时加载默认魅力-日榜
 * - [selectType] 切换榜单类型，触发重新加载
 * - [selectPeriod] 切换榜单周期，触发重新加载
 * - [refresh] 下拉刷新：重新请求当前 type+period 的榜单
 * - 401 错误 → 发射 [RankingEvent.NavigateToLogin]
 *
 * @param rankingRepository 榜单 Repository
 */
class RankingViewModel(
    private val rankingRepository: IRankingRepository,
) : ViewModel() {

    // ─── State & Events ───────────────────────────────────────────────────────

    private val _uiState = MutableStateFlow(RankingUiState())
    val uiState: StateFlow<RankingUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<RankingEvent>()
    val events: SharedFlow<RankingEvent> = _events.asSharedFlow()

    // ─── Init ─────────────────────────────────────────────────────────────────

    init {
        loadRanking()
    }

    // ─── Public Actions ───────────────────────────────────────────────────────

    /**
     * 切换一级 Tab（魅力榜/财富榜），重新加载数据
     */
    fun selectType(type: RankingType) {
        if (_uiState.value.type == type) return
        _uiState.update { it.copy(type = type, error = null) }
        loadRanking()
    }

    /**
     * 切换二级 Tab（日榜/周榜），重新加载数据
     */
    fun selectPeriod(period: Period) {
        if (_uiState.value.period == period) return
        _uiState.update { it.copy(period = period, error = null) }
        loadRanking()
    }

    /**
     * 下拉刷新：重新请求当前 type+period 的榜单
     */
    fun refresh() {
        viewModelScope.launch {
            _uiState.update { it.copy(refreshing = true, error = null) }
            fetchRanking(
                type = _uiState.value.type.apiValue,
                period = _uiState.value.period.apiValue,
            )
        }
    }

    // ─── Private ─────────────────────────────────────────────────────────────

    private fun loadRanking() {
        viewModelScope.launch {
            _uiState.update { it.copy(loading = true, error = null) }
            fetchRanking(
                type = _uiState.value.type.apiValue,
                period = _uiState.value.period.apiValue,
            )
        }
    }

    private suspend fun fetchRanking(type: String, period: String) {
        rankingRepository.getRanking(type = type, period = period)
            .onSuccess { page ->
                _uiState.update {
                    it.copy(
                        items = page.items,
                        myRank = page.me,
                        loading = false,
                        refreshing = false,
                        error = null,
                    )
                }
            }
            .onFailure { e ->
                if (e is CancellationException) throw e
                if (e is ApiException && e.code == 401) {
                    _uiState.update { it.copy(loading = false, refreshing = false) }
                    _events.emit(RankingEvent.NavigateToLogin)
                } else {
                    _uiState.update {
                        it.copy(loading = false, refreshing = false, error = e.message)
                    }
                }
            }
    }

    // ─── Factory ──────────────────────────────────────────────────────────────

    companion object {
        fun factory(rankingRepository: IRankingRepository) =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T =
                    RankingViewModel(rankingRepository) as T
            }
    }
}
