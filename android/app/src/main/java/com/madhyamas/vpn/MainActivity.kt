package com.madhyamas.vpn

import android.content.Intent
import android.net.VpnService
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.madhyamas.vpn.config.ProxyConfig
import com.madhyamas.vpn.vpn.CertInstallActivity
import com.madhyamas.vpn.vpn.getInstalledApps
import com.madhyamas.vpn.vpn.AppInfo

class MainActivity : ComponentActivity() {

    private lateinit var vpnPermissionLauncher: ActivityResultLauncher<Intent>

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        vpnPermissionLauncher = registerForActivityResult(
            ActivityResultContracts.StartActivityForResult()
        ) { result ->
            if (result.resultCode == RESULT_OK) {
                viewModelStore.let {
                    // Re-fetch the ViewModel and start VPN
                    val vm = viewModel<MainViewModel>(this)
                    vm.startVpnAfterPermission()
                }
            }
        }

        setContent {
            MaterialTheme(
                colorScheme = lightColorScheme()
            ) {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    MadhyamasApp()
                }
            }
        }
    }

    @Composable
    private fun MadhyamasApp() {
        val viewModel: MainViewModel = viewModel()
        val config by viewModel.config.collectAsState()
        val status by viewModel.status.collectAsState()
        val stats by viewModel.stats.collectAsState()
        val error by viewModel.error.collectAsState()

        var showSettings by remember { mutableStateOf(false) }
        var showAppSelector by remember { mutableStateOf(false) }

        if (showSettings) {
            SettingsScreen(
                config = config,
                onProxyHostChange = viewModel::updateProxyHost,
                onProxyPortChange = viewModel::updateProxyPort,
                onApiHostChange = viewModel::updateApiHost,
                onApiPortChange = viewModel::updateApiPort,
                onBack = { showSettings = false }
            )
        } else if (showAppSelector) {
            AppSelectorScreen(
                selectedPackages = config.selectedPackages,
                onPackagesSelected = { packages ->
                    viewModel.updateSelectedPackages(packages)
                    showAppSelector = false
                },
                onBack = { showAppSelector = false }
            )
        } else {
            MainScreen(
                config = config,
                status = status,
                stats = stats,
                error = error,
                onToggleVpn = {
                    if (status == VpnStatus.CONNECTED || status == VpnStatus.CONNECTING) {
                        viewModel.stopVpn()
                    } else {
                        val prepareIntent = viewModel.startVpn()
                        if (prepareIntent != null) {
                            vpnPermissionLauncher.launch(prepareIntent)
                        }
                    }
                },
                onSettings = { showSettings = true },
                onAppSelector = { showAppSelector = true },
                onInstallCert = {
                    val intent = Intent(this@MainActivity, CertInstallActivity::class.java).apply {
                        putExtra(CertInstallActivity.EXTRA_API_HOST, config.apiHost)
                        putExtra(CertInstallActivity.EXTRA_API_PORT, config.apiPort)
                    }
                    startActivity(intent)
                }
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MainScreen(
    config: ProxyConfig,
    status: VpnStatus,
    stats: VpnStats,
    error: String?,
    onToggleVpn: () -> Unit,
    onSettings: () -> Unit,
    onAppSelector: () -> Unit,
    onInstallCert: () -> Unit
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Madhyamas VPN") },
                actions = {
                    IconButton(onClick = onSettings) {
                        Icon(Icons.Default.Settings, contentDescription = "Settings")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Status indicator
            val statusColor = when (status) {
                VpnStatus.CONNECTED -> MaterialTheme.colorScheme.primary
                VpnStatus.CONNECTING -> MaterialTheme.colorScheme.tertiary
                VpnStatus.ERROR -> MaterialTheme.colorScheme.error
                VpnStatus.DISCONNECTED -> MaterialTheme.colorScheme.outline
            }

            val statusText = when (status) {
                VpnStatus.CONNECTED -> "Connected"
                VpnStatus.CONNECTING -> "Connecting..."
                VpnStatus.ERROR -> "Error"
                VpnStatus.DISCONNECTED -> "Disconnected"
            }

            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = statusColor.copy(alpha = 0.1f))
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Icon(
                        imageVector = if (status == VpnStatus.CONNECTED)
                            Icons.Default.VpnKey else Icons.Default.VpnKeyOff,
                        contentDescription = null,
                        tint = statusColor,
                        modifier = Modifier.size(48.dp)
                    )
                    Spacer(Modifier.height(8.dp))
                    Text(statusText, style = MaterialTheme.typography.titleLarge)
                    Text(
                        "${config.proxyHost}:${config.proxyPort}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // VPN toggle button
            Button(
                onClick = onToggleVpn,
                modifier = Modifier.fillMaxWidth(),
                colors = if (status == VpnStatus.CONNECTED) {
                    ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                } else {
                    ButtonDefaults.buttonColors()
                }
            ) {
                Icon(
                    if (status == VpnStatus.CONNECTED) Icons.Default.Stop else Icons.Default.PlayArrow,
                    contentDescription = null
                )
                Spacer(Modifier.width(8.dp))
                Text(if (status == VpnStatus.CONNECTED) "Stop VPN" else "Start VPN")
            }

            Spacer(Modifier.height(16.dp))

            // Stats (only shown when connected)
            if (status == VpnStatus.CONNECTED) {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Statistics", style = MaterialTheme.typography.titleMedium)
                        Spacer(Modifier.height(8.dp))
                        StatRow("Total Connections", stats.totalConnections.toString())
                        StatRow("Active Connections", stats.activeConnections.toString())
                        StatRow("Bytes Sent", formatBytes(stats.bytesSent))
                        StatRow("Bytes Received", formatBytes(stats.bytesReceived))
                    }
                }
                Spacer(Modifier.height(16.dp))
            }

            // App selector
            OutlinedButton(
                onClick = onAppSelector,
                modifier = Modifier.fillMaxWidth()
            ) {
                Icon(Icons.Default.Apps, contentDescription = null)
                Spacer(Modifier.width(8.dp))
                Text(
                    if (config.selectedPackages.isEmpty())
                        "All Apps (tap to filter)"
                    else
                        "${config.selectedPackages.size} apps selected"
                )
            }

            Spacer(Modifier.height(8.dp))

            // Certificate installation
            OutlinedButton(
                onClick = onInstallCert,
                modifier = Modifier.fillMaxWidth()
            ) {
                Icon(Icons.Default.Security, contentDescription = null)
                Spacer(Modifier.width(8.dp))
                Text("Install CA Certificate")
            }

            // Error message
            if (error != null) {
                Spacer(Modifier.height(16.dp))
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.errorContainer
                    )
                ) {
                    Text(
                        error!!,
                        modifier = Modifier.padding(16.dp),
                        color = MaterialTheme.colorScheme.onErrorContainer
                    )
                }
            }

            Spacer(Modifier.weight(1f))

            // Info card
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant
                )
            ) {
                Text(
                    "Madhyamas VPN routes traffic from selected apps to the " +
                    "Madhyamas proxy for inspection. Install the CA certificate " +
                    "for HTTPS interception.\n\n" +
                    "For apps with certificate pinning, use Frida or APK " +
                    "patching in addition to this VPN.",
                    modifier = Modifier.padding(16.dp),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
private fun StatRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium)
        Text(value, style = MaterialTheme.typography.bodyMedium)
    }
}

private fun formatBytes(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "${bytes / 1024} KB"
        bytes < 1024 * 1024 * 1024 -> "${bytes / (1024 * 1024)} MB"
        else -> "${bytes / (1024 * 1024 * 1024)} GB"
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SettingsScreen(
    config: ProxyConfig,
    onProxyHostChange: (String) -> Unit,
    onProxyPortChange: (Int) -> Unit,
    onApiHostChange: (String) -> Unit,
    onApiPortChange: (Int) -> Unit,
    onBack: () -> Unit
) {
    var proxyHost by remember(config.proxyHost) { mutableStateOf(config.proxyHost) }
    var proxyPort by remember(config.proxyPort) { mutableStateOf(config.proxyPort.toString()) }
    var apiHost by remember(config.apiHost) { mutableStateOf(config.apiHost) }
    var apiPort by remember(config.apiPort) { mutableStateOf(config.apiPort.toString()) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            Text("Proxy Configuration", style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.height(16.dp))

            OutlinedTextField(
                value = proxyHost,
                onValueChange = {
                    proxyHost = it
                    onProxyHostChange(it)
                },
                label = { Text("Proxy Host") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )
            Spacer(Modifier.height(8.dp))

            OutlinedTextField(
                value = proxyPort,
                onValueChange = {
                    proxyPort = it
                    it.toIntOrNull()?.let(onProxyPortChange)
                },
                label = { Text("Proxy Port") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )

            Spacer(Modifier.height(24.dp))

            Text("API Configuration", style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.height(16.dp))

            OutlinedTextField(
                value = apiHost,
                onValueChange = {
                    apiHost = it
                    onApiHostChange(it)
                },
                label = { Text("API Host") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )
            Spacer(Modifier.height(8.dp))

            OutlinedTextField(
                value = apiPort,
                onValueChange = {
                    apiPort = it
                    it.toIntOrNull()?.let(onApiPortChange)
                },
                label = { Text("API Port") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )

            Spacer(Modifier.height(24.dp))

            Text(
                "The proxy host/port is where the Madhyamas proxy listens " +
                "for HTTP CONNECT requests (default: 8888).\n\n" +
                "The API host/port is for the Madhyamas REST API, used to " +
                "download the CA certificate (default: 3001).",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AppSelectorScreen(
    selectedPackages: Set<String>,
    onPackagesSelected: (Set<String>) -> Unit,
    onBack: () -> Unit
) {
    val context = LocalContext.current
    var apps by remember { mutableStateOf<List<AppInfo>>(emptyList()) }
    var selected by remember(selectedPackages) { mutableStateOf(selectedPackages.toMutableSet()) }
    var loading by remember { mutableStateOf(true) }
    var searchQuery by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        loading = true
        apps = getInstalledApps(context, excludeSystem = true)
        loading = false
    }

    val filteredApps = remember(apps, searchQuery) {
        if (searchQuery.isBlank()) apps
        else apps.filter {
            it.label.contains(searchQuery, ignoreCase = true) ||
            it.packageName.contains(searchQuery, ignoreCase = true)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Select Apps") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    TextButton(onClick = { onPackagesSelected(selected) }) {
                        Text("Done")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Search bar
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { searchQuery = it },
                label = { Text("Search apps") },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                singleLine = true
            )

            // "All apps" option
            ListItem(
                headlineContent = { Text("All Apps") },
                supportingContent = { Text("Route traffic from all apps") },
                leadingContent = {
                    RadioButton(
                        selected = selected.isEmpty(),
                        onClick = { selected.clear() }
                    )
                },
                modifier = Modifier.fillMaxWidth()
            )
            HorizontalDivider()

            if (loading) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            } else {
                LazyColumn {
                    items(filteredApps) { app ->
                        ListItem(
                            headlineContent = { Text(app.label) },
                            supportingContent = {
                                Text(
                                    app.packageName,
                                    style = MaterialTheme.typography.bodySmall
                                )
                            },
                            leadingContent = {
                                Checkbox(
                                    checked = app.packageName in selected,
                                    onCheckedChange = { checked ->
                                        selected = selected.toMutableSet().apply {
                                            if (checked) add(app.packageName)
                                            else remove(app.packageName)
                                        }
                                    }
                                )
                            },
                            modifier = Modifier.fillMaxWidth()
                        )
                    }
                }
            }
        }
    }
}
