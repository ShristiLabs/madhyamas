package com.madhyamas.vpn

import android.app.Application
import android.content.Intent
import android.net.VpnService
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.madhyamas.vpn.config.ConfigManager
import com.madhyamas.vpn.config.ProxyConfig
import com.madhyamas.vpn.pairing.ConnectParseResult
import com.madhyamas.vpn.pairing.ConnectUriParser
import com.madhyamas.vpn.pairing.CredentialStore
import com.madhyamas.vpn.pairing.EnrollmentClient
import com.madhyamas.vpn.pairing.EnrollmentResult
import com.madhyamas.vpn.pairing.HttpUrlEnrollmentClient
import com.madhyamas.vpn.pairing.InstallationId
import com.madhyamas.vpn.pairing.KeystoreAead
import com.madhyamas.vpn.pairing.PrefsKeyValueStore
import com.madhyamas.vpn.vpn.ConnectState
import com.madhyamas.vpn.vpn.MadhyamasVpnService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

enum class VpnStatus { DISCONNECTED, CONNECTING, CONNECTED, ERROR }

data class VpnStats(
    val totalConnections: Int = 0,
    val activeConnections: Int = 0,
    val bytesSent: Long = 0,
    val bytesReceived: Long = 0
)

/**
 * Pairing snapshot for the UI's pairing card (issue #111): the state of
 * the credential half of the companion (the VPN toggle remains separate).
 */
data class PairingSnapshot(
    val paired: Boolean,
    val deviceName: String?,
    val deviceId: String?,
    val host: String,
    val port: Int,
    val tls: Boolean
)

class MainViewModel(application: Application) : AndroidViewModel(application) {

    private val configManager = ConfigManager(application)

    /** Injectable for tests; HttpURLConnection-based in production. */
    var enrollmentClient: EnrollmentClient = HttpUrlEnrollmentClient()

    private val prefsStore by lazy { PrefsKeyValueStore(application) }
    private val credentialStore by lazy { CredentialStore(prefsStore, KeystoreAead()) }

    private val _config = MutableStateFlow(ProxyConfig())
    val config: StateFlow<ProxyConfig> = _config.asStateFlow()

    private val _status = MutableStateFlow(VpnStatus.DISCONNECTED)
    val status: StateFlow<VpnStatus> = _status.asStateFlow()

    private val _stats = MutableStateFlow(VpnStats())
    val stats: StateFlow<VpnStats> = _stats.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _pairing = MutableStateFlow<PairingSnapshot?>(null)
    val pairing: StateFlow<PairingSnapshot?> = _pairing.asStateFlow()

    private val _enrolling = MutableStateFlow(false)
    val enrolling: StateFlow<Boolean> = _enrolling.asStateFlow()

    private val _pairingError = MutableStateFlow<String?>(null)
    val pairingError: StateFlow<String?> = _pairingError.asStateFlow()

    private val _lastConnectState = MutableStateFlow<ConnectState?>(null)
    val lastConnectState: StateFlow<ConnectState?> = _lastConnectState.asStateFlow()

    private val _authRejected = MutableStateFlow(false)
    val authRejected: StateFlow<Boolean> = _authRejected.asStateFlow()

    init {
        // Stable installation identity (kept locally; the server's enroll
        // schema has no field for it — recorded as a follow-up).
        InstallationId.getOrCreate(prefsStore)

        // Load saved config + pairing state
        viewModelScope.launch {
            configManager.configFlow.collect { cfg ->
                _config.value = cfg
                refreshPairing(cfg)
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
                    _lastConnectState.value = service.lastConnectState
                    _authRejected.value = service.authRejected
                } else {
                    if (_status.value == VpnStatus.CONNECTED) {
                        _status.value = VpnStatus.DISCONNECTED
                    }
                    _lastConnectState.value = null
                    _authRejected.value = false
                }
                kotlinx.coroutines.delay(1000)
            }
        }
    }

    private fun refreshPairing(cfg: ProxyConfig = _config.value) {
        _pairing.value = PairingSnapshot(
            paired = credentialStore.isEnrolled(),
            deviceName = credentialStore.deviceName() ?: cfg.deviceName,
            deviceId = credentialStore.deviceId(),
            host = cfg.proxyHost,
            port = cfg.proxyPort,
            tls = cfg.useTls
        )
    }

    /**
     * Entry point for the `madhyamas://connect` deep link (QR scan).
     * Malformed links surface a clear error and change nothing; valid
     * links pre-fill the proxy config and either store the manual `key`
     * directly or redeem `token` via the enroll API (the exchange).
     */
    fun handleDeepLink(raw: String?) {
        if (raw.isNullOrBlank()) return
        when (val parsed = ConnectUriParser.parse(raw)) {
            is ConnectParseResult.Error -> {
                _pairingError.value = parsed.reason
            }
            is ConnectParseResult.Ok -> {
                val payload = parsed.payload
                _pairingError.value = null
                viewModelScope.launch {
                    configManager.updateProxyHost(payload.host)
                    configManager.updateProxyPort(payload.port)
                    configManager.updateUseTls(payload.tls)
                    configManager.updateApiBaseUrl(payload.apiUrl)
                    configManager.updateDeviceName(payload.name)

                    if (payload.key != null) {
                        // Manual mode: the QR carried the standing credential.
                        storeCredential(payload.key, deviceId = null, deviceName = payload.name)
                    } else if (payload.token != null) {
                        // Enrollment exchange: redeem the single-use token.
                        val api = payload.apiUrl ?: run {
                            _pairingError.value = "Token links must include the API URL"
                            return@launch
                        }
                        _enrolling.value = true
                        val result = withContext(Dispatchers.IO) {
                            enrollmentClient.enroll(api, payload.token)
                        }
                        _enrolling.value = false
                        when (result) {
                            is EnrollmentResult.Success -> storeCredential(
                                result.deviceKey,
                                deviceId = result.deviceId,
                                deviceName = result.deviceName ?: payload.name
                            )
                            is EnrollmentResult.Failure ->
                                _pairingError.value = describeEnrollmentFailure(result.reason)
                        }
                    }
                }
            }
        }
    }

    private fun storeCredential(deviceKey: String, deviceId: String?, deviceName: String?) {
        credentialStore.store(deviceKey, deviceId, deviceName)
        refreshPairing()
    }

    private fun describeEnrollmentFailure(reason: EnrollmentResult.Failure.Reason): String =
        when (reason) {
            EnrollmentResult.Failure.Reason.MALFORMED_TOKEN ->
                "The enrollment token is malformed"
            EnrollmentResult.Failure.Reason.INVALID_TOKEN ->
                "Enrollment token invalid or expired — it may already have been used. Generate a new QR code."
            EnrollmentResult.Failure.Reason.SERVER_ERROR ->
                "The Madhyamas server rejected the enrollment request"
            EnrollmentResult.Failure.Reason.NETWORK ->
                "Could not reach the Madhyamas server — check the network and host"
            EnrollmentResult.Failure.Reason.BAD_RESPONSE ->
                "Unexpected response from the Madhyamas server"
        }

    /** Forget the pairing: clears the Keystore-sealed credential and metadata. */
    fun forget() {
        // Stop a running VPN first: the service holds the
        // Proxy-Authorization header built at start and must not keep
        // authenticating with the credential being wiped here.
        if (MadhyamasVpnService.instance != null) {
            stopVpn()
        }
        credentialStore.clear()
        viewModelScope.launch {
            configManager.updateDeviceName(null)
            configManager.updateApiBaseUrl(null)
        }
        _pairingError.value = null
        refreshPairing()
    }

    fun dismissPairingError() {
        _pairingError.value = null
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
        context.startService(buildStartIntent(cfg))
        _status.value = VpnStatus.CONNECTING
        return null
    }

    fun startVpnAfterPermission() {
        val context = getApplication<Application>()
        context.startService(buildStartIntent(_config.value))
    }

    private fun buildStartIntent(cfg: ProxyConfig): Intent =
        Intent(getApplication<Application>(), MadhyamasVpnService::class.java).apply {
            action = MadhyamasVpnService.ACTION_START
            putExtra(MadhyamasVpnService.EXTRA_PROXY_HOST, cfg.proxyHost)
            putExtra(MadhyamasVpnService.EXTRA_PROXY_PORT, cfg.proxyPort)
            putExtra(MadhyamasVpnService.EXTRA_USE_TLS, cfg.useTls)
            putExtra(
                MadhyamasVpnService.EXTRA_ALLOWED_PACKAGES,
                cfg.selectedPackages.toTypedArray()
            )
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
