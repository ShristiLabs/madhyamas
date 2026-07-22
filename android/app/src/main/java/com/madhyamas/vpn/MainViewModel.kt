package com.madhyamas.vpn

import android.app.Application
import android.content.Intent
import android.net.VpnService
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.madhyamas.vpn.config.ConfigManager
import com.madhyamas.vpn.config.ProxyConfig
import com.madhyamas.vpn.vpn.MadhyamasVpnService
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

enum class VpnStatus { DISCONNECTED, CONNECTING, CONNECTED, ERROR }

data class VpnStats(
    val totalConnections: Int = 0,
    val activeConnections: Int = 0,
    val bytesSent: Long = 0,
    val bytesReceived: Long = 0
)

class MainViewModel(application: Application) : AndroidViewModel(application) {

    private val configManager = ConfigManager(application)

    private val _config = MutableStateFlow(ProxyConfig())
    val config: StateFlow<ProxyConfig> = _config.asStateFlow()

    private val _status = MutableStateFlow(VpnStatus.DISCONNECTED)
    val status: StateFlow<VpnStatus> = _status.asStateFlow()

    private val _stats = MutableStateFlow(VpnStats())
    val stats: StateFlow<VpnStats> = _stats.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    init {
        // Load saved config
        viewModelScope.launch {
            configManager.configFlow.collect { cfg ->
                _config.value = cfg
            }
        }

        // Poll stats when connected
        viewModelScope.launch {
            while (true) {
                val service = MadhyamasVpnService.instance
                if (service != null) {
                    _status.value = VpnStatus.CONNECTED
                    _stats.value = VpnStats(
                        totalConnections = service.totalConnections,
                        activeConnections = service.activeConnections,
                        bytesSent = service.bytesSent,
                        bytesReceived = service.bytesReceived
                    )
                } else {
                    if (_status.value == VpnStatus.CONNECTED) {
                        _status.value = VpnStatus.DISCONNECTED
                    }
                }
                kotlinx.coroutines.delay(1000)
            }
        }
    }

    fun updateProxyHost(host: String) {
        viewModelScope.launch {
            configManager.updateProxyHost(host)
        }
    }

    fun updateProxyPort(port: Int) {
        viewModelScope.launch {
            configManager.updateProxyPort(port)
        }
    }

    fun updateApiHost(host: String) {
        viewModelScope.launch {
            configManager.updateApiHost(host)
        }
    }

    fun updateApiPort(port: Int) {
        viewModelScope.launch {
            configManager.updateApiPort(port)
        }
    }

    fun updateSelectedPackages(packages: Set<String>) {
        viewModelScope.launch {
            configManager.updateSelectedPackages(packages)
        }
    }

    fun updateExcludeSystemApps(exclude: Boolean) {
        viewModelScope.launch {
            configManager.updateExcludeSystemApps(exclude)
        }
    }

    /**
     * Start the VPN service. Returns the intent for requesting VPN
     * permission if needed (the activity must call startActivityForResult).
     */
    fun startVpn(): Intent? {
        val context = getApplication<Application>()
        val cfg = _config.value

        // Check if VPN permission has been granted
        val prepareIntent = VpnService.prepare(context)
        if (prepareIntent != null) {
            _status.value = VpnStatus.CONNECTING
            return prepareIntent // Activity must launch this and call startVpn again on result
        }

        // Permission already granted — start the service
        val intent = Intent(context, MadhyamasVpnService::class.java).apply {
            action = MadhyamasVpnService.ACTION_START
            putExtra(MadhyamasVpnService.EXTRA_PROXY_HOST, cfg.proxyHost)
            putExtra(MadhyamasVpnService.EXTRA_PROXY_PORT, cfg.proxyPort)
            putExtra(
                MadhyamasVpnService.EXTRA_ALLOWED_PACKAGES,
                cfg.selectedPackages.toTypedArray()
            )
        }
        context.startService(intent)
        _status.value = VpnStatus.CONNECTING
        return null
    }

    fun startVpnAfterPermission() {
        val context = getApplication<Application>()
        val cfg = _config.value
        val intent = Intent(context, MadhyamasVpnService::class.java).apply {
            action = MadhyamasVpnService.ACTION_START
            putExtra(MadhyamasVpnService.EXTRA_PROXY_HOST, cfg.proxyHost)
            putExtra(MadhyamasVpnService.EXTRA_PROXY_PORT, cfg.proxyPort)
            putExtra(
                MadhyamasVpnService.EXTRA_ALLOWED_PACKAGES,
                cfg.selectedPackages.toTypedArray()
            )
        }
        context.startService(intent)
    }

    fun stopVpn() {
        val context = getApplication<Application>()
        val intent = Intent(context, MadhyamasVpnService::class.java).apply {
            action = MadhyamasVpnService.ACTION_STOP
        }
        context.startService(intent)
        _status.value = VpnStatus.DISCONNECTED
        _stats.value = VpnStats()
    }
}
