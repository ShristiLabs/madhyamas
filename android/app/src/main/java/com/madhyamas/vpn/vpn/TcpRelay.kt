package com.madhyamas.vpn.vpn

import android.net.VpnService
import android.util.Log
import java.io.IOException
import java.net.InetSocketAddress
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLException
import javax.net.ssl.SSLSocket

/**
 * Relays a single TCP connection between an Android app (via the VPN
 * tunnel) and the Madhyamas proxy (via HTTP CONNECT).
 *
 * Flow:
 *  1. Connect to the Madhyamas proxy at [proxyHost]:[proxyPort]
 *     (TLS-wrapped when [useTls] is set — QR payload tls=1, issue #110)
 *  2. Send the CONNECT request the companion AUTHORS itself, carrying
 *     `Proxy-Authorization` from the stored device credential when paired
 *     ([proxyAuthorization], issue #111). Because the companion
 *     re-originates the connection, an app's own proxy-auth headers never
 *     reach the proxy's CONNECT parser — the companion credential wins by
 *     construction.
 *  3. Wait for: `HTTP/1.1 200 Connection Established\r\n\r\n`. A 407 is
 *     reported as [ConnectState.REJECTED] and the connection is closed —
 *     never retried with the same credential.
 *  4. Relay data bidirectionally:
 *     - App → Proxy: data from VPN tunnel → proxy socket
 *     - Proxy → App: data from proxy socket → VPN tunnel
 *
 * Note: This implementation uses IP addresses for CONNECT. For proper
 * SNI/hostname support, a DNS resolver should be added to map IPs back
 * to hostnames (via a DNS cache or by intercepting DNS queries).
 */
class TcpRelay(
    private val id: Int,
    private val srcPort: Int,
    private val dstIp: String,
    private val dstPort: Int,
    private val proxyHost: String,
    private val proxyPort: Int,
    private val useTls: Boolean = false,
    private val proxyAuthorization: String? = null,
    private val vpnService: VpnService,
    private val onClose: (Int) -> Unit,
    private val onBytesSent: (Int) -> Unit,
    private val onBytesReceived: (Int) -> Unit,
    private val onConnectResult: (ConnectState) -> Unit = {}
) {
    companion object {
        private const val TAG = "TcpRelay"
        private const val CONNECT_TIMEOUT_MS = 10_000
        private const val READ_TIMEOUT_MS = 30_000
    }

    private val running = AtomicBoolean(false)
    private val closed = AtomicBoolean(false)
    private var proxySocket: java.net.Socket? = null
    private var proxyOutput: java.io.OutputStream? = null
    private var proxyInput: java.io.InputStream? = null
    private var relayThread: Thread? = null

    fun start() {
        Thread {
            try {
                connectToProxy()
                startProxyToVpnRelay()
            } catch (e: Exception) {
                Log.e(TAG, "Relay $id failed: ${e.message}")
                close()
            }
        }.also { relayThread = it }.start()
    }

    /**
     * Connect to the Madhyamas proxy and establish a CONNECT tunnel.
     */
    private fun connectToProxy() {
        val socket = openProxySocket()
        socket.soTimeout = READ_TIMEOUT_MS

        val input = socket.getInputStream()
        val output = socket.getOutputStream()
        proxySocket = socket
        proxyInput = input
        proxyOutput = output

        when (val result = ProxyTunnelHandshake.perform(
            output, input, dstIp, dstPort, proxyAuthorization
        )) {
            TunnelResult.Established -> {
                running.set(true)
                onConnectResult(ConnectState.OK)
                Log.d(TAG, "Relay $id: CONNECT tunnel established to $dstIp:$dstPort via $proxyHost:$proxyPort")
            }
            TunnelResult.Rejected -> {
                onConnectResult(ConnectState.REJECTED)
                throw IOException("Proxy rejected the device credential (HTTP 407)")
            }
            is TunnelResult.Failed -> {
                onConnectResult(ConnectState.FAILED)
                throw IOException("Proxy CONNECT failed: ${result.statusLine}")
            }
            is TunnelResult.IoError -> {
                onConnectResult(ConnectState.UNREACHABLE)
                throw IOException("Proxy connection error: ${result.message}")
            }
        }
    }

    /**
     * Opens the socket to the proxy: plain TCP, or TLS when the QR payload
     * carried tls=1. The socket is protected from the VPN before
     * connecting. TLS failures (handshake or hostname verification) are
     * reported as [ConnectState.TLS_ERROR] and never fall back to
     * plaintext.
     */
    private fun openProxySocket(): java.net.Socket {
        val socket = ProxySockets.create(useTls)
        vpnService.protect(socket) // Prevent the socket from going through the VPN
        socket.tcpNoDelay = true
        try {
            socket.connect(InetSocketAddress(proxyHost, proxyPort), CONNECT_TIMEOUT_MS)
        } catch (e: SSLException) {
            reportTlsFailure(socket, e)
        } catch (e: IOException) {
            onConnectResult(ConnectState.UNREACHABLE)
            throw e
        }
        if (socket is SSLSocket) {
            try {
                socket.startHandshake()
            } catch (e: SSLException) {
                reportTlsFailure(socket, e)
            }
            if (!HttpsURLConnection.getDefaultHostnameVerifier().verify(proxyHost, socket.session)) {
                reportTlsFailure(
                    socket,
                    IOException("TLS certificate for $proxyHost failed hostname verification")
                )
            }
        }
        return socket
    }

    private fun reportTlsFailure(socket: java.net.Socket, e: Exception): Nothing {
        onConnectResult(ConnectState.TLS_ERROR)
        try {
            socket.close()
        } catch (ignored: Exception) {
        }
        throw IOException("TLS connection to $proxyHost:$proxyPort failed: ${e.message}", e)
    }

    /**
     * Relay data from proxy → VPN (app direction).
     * The app → proxy direction is handled by [writeToProxy] called from
     * the VPN packet processing thread.
     */
    private fun startProxyToVpnRelay() {
        val buffer = ByteArray(8192)
        while (running.get()) {
            try {
                val read = proxyInput?.read(buffer) ?: -1
                if (read <= 0) {
                    Log.d(TAG, "Relay $id: proxy closed connection")
                    break
                }
                onBytesReceived(read)
                val data = buffer.copyOfRange(0, read)
                // Write back to VPN tunnel — the VPN service will encapsulate
                // this into a TCP packet and send it to the app
                (vpnService as? MadhyamasVpnService)?.writeToVpn(data, dstPort, srcPort)
            } catch (e: IOException) {
                if (running.get()) {
                    Log.d(TAG, "Relay $id: read error: ${e.message}")
                }
                break
            }
        }
        close()
    }

    /**
     * Write data from the app (via VPN) to the proxy.
     * Called from the VPN packet processing thread.
     */
    fun writeToProxy(data: ByteArray) {
        try {
            if (running.get()) {
                proxyOutput?.write(data)
                proxyOutput?.flush()
                onBytesSent(data.size)
            }
        } catch (e: IOException) {
            Log.d(TAG, "Relay $id: write error: ${e.message}")
            close()
        }
    }

    /**
     * Idempotent teardown: always runs exactly once per relay, including
     * on pre-establishment failures (407/TLS/unreachable) where `running`
     * was never set — the proxy socket must be closed and [onClose] must
     * fire so the service drops the relay from its map.
     */
    fun close() {
        if (!closed.compareAndSet(false, true)) return
        running.set(false)
        try {
            proxyInput?.close()
        } catch (e: Exception) {}
        try {
            proxyOutput?.close()
        } catch (e: Exception) {}
        try {
            proxySocket?.close()
        } catch (e: Exception) {}
        onClose(id)
    }
}
