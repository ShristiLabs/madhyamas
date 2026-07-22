package com.madhyamas.vpn.vpn

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.net.HttpURLConnection
import java.net.URL

/**
 * Activity that downloads the Madhyamas CA certificate from the API
 * and launches the Android system certificate installer.
 *
 * The certificate is fetched from: http://<proxyHost>:<apiPort>/api/cert/ca
 * (returns PEM-encoded CA certificate)
 */
class CertInstallActivity : ComponentActivity() {

    companion object {
        private const val TAG = "CertInstall"
        const val EXTRA_API_HOST = "api_host"
        const val EXTRA_API_PORT = "api_port"
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val apiHost = intent.getStringExtra(EXTRA_API_HOST) ?: "127.0.0.1"
        val apiPort = intent.getIntExtra(EXTRA_API_PORT, 3001)

        setContent {
            CertInstallScreen(
                apiHost = apiHost,
                apiPort = apiPort,
                onInstall = { host, port ->
                    downloadAndInstallCert(host, port)
                },
                onFinish = { finish() }
            )
        }
    }

    private fun downloadAndInstallCert(host: String, port: Int) {
        lifecycleScope.launch {
            try {
                val certBytes = withContext(Dispatchers.IO) {
                    downloadCert(host, port)
                }
                if (certBytes != null) {
                    launchCertInstaller(certBytes)
                } else {
                    Toast.makeText(this@CertInstallActivity,
                        "Failed to download certificate", Toast.LENGTH_LONG).show()
                }
            } catch (e: Exception) {
                Log.e(TAG, "Certificate download failed", e)
                Toast.makeText(this@CertInstallActivity,
                    "Error: ${e.message}", Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun downloadCert(host: String, port: Int): ByteArray? {
        val url = URL("http://$host:$port/api/cert/ca")
        val conn = url.openConnection() as HttpURLConnection
        return try {
            conn.connectTimeout = 5000
            conn.readTimeout = 5000
            if (conn.responseCode == 200) {
                conn.inputStream.readBytes()
            } else {
                Log.e(TAG, "Certificate API returned ${conn.responseCode}")
                null
            }
        } finally {
            conn.disconnect()
        }
    }

    private fun launchCertInstaller(certBytes: ByteArray) {
        // Use the credentials install intent. The action string and extra keys
        // are not exposed as public constants on KeyChain, so they are hardcoded.
        val intent = Intent("android.credentials.INSTALL").apply {
            putExtra("name", "Madhyamas CA Certificate")
            putExtra("CERT", certBytes)
        }
        startActivity(intent)
    }
}

@Composable
fun CertInstallScreen(
    apiHost: String,
    apiPort: Int,
    onInstall: (String, Int) -> Unit,
    onFinish: () -> Unit
) {
    var installing by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            "Install Madhyamas CA Certificate",
            style = MaterialTheme.typography.headlineSmall
        )
        Spacer(Modifier.height(16.dp))
        Text(
            "Download the CA certificate from $apiHost:$apiPort and install it " +
            "into the Android system trust store.\n\n" +
            "After installation, apps that don't use certificate pinning " +
            "will trust the Madhyamas proxy for HTTPS interception.",
            style = MaterialTheme.typography.bodyMedium
        )
        Spacer(Modifier.height(24.dp))
        Row {
            Button(
                onClick = {
                    installing = true
                    onInstall(apiHost, apiPort)
                },
                enabled = !installing
            ) {
                Text(if (installing) "Installing..." else "Download & Install")
            }
            Spacer(Modifier.width(8.dp))
            OutlinedButton(onClick = onFinish) {
                Text("Cancel")
            }
        }
    }
}
