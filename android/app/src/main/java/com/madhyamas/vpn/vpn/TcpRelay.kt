package com.madhyamas.vpn.vpn

import android.net.VpnService
import android.util.Log
import java.io.IOException
import java.net.InetSocketAddress
import java.net.Socket
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Relays a single TCP connection between an Android app (via the VPN
 * tunnel) and the Madhyamas proxy (via HTTP CONNECT).
 *
 * Flow:
 *  1. Connect to the Madhyamas proxy at [proxyHost]:[proxyPort]
 *  2. Send: `CONNECT <dstIp>:<dstPort> HTTP/1.1\r\nHost: <dstIp>:<dstPort>\r\n\r\n`
 *  3. Wait for: `HTTP/1.1 200 Connection Established\r\n\r\n`
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
    private val vpnService: VpnService,
    private val onClose: (Int) -> Unit,
    private val onBytesSent: (Int) -> Unit,
    private val onBytesReceived: (Int) -> Unit
) {
    companion object {
        private const val TAG = "TcpRelay"
        private const val CONNECT_TIMEOUT_MS = 10_000
        private const val READ_TIMEOUT_MS = 30_000
    }

    private val running = AtomicBoolean(false)
    private var proxySocket: Socket? = null
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
        val socket = Socket()
        vpnService.protect(socket) // Prevent the socket from going through the VPN

        socket.connect(InetSocketAddress(proxyHost, proxyPort), CONNECT_TIMEOUT_MS)
        socket.soTimeout = READ_TIMEOUT_MS
        socket.tcpNoDelay = true

        proxySocket = socket
        proxyOutput = socket.getOutputStream()
        proxyInput = socket.getInputStream()

        // Send HTTP CONNECT request
        val connectReq = buildString {
            append("CONNECT $dstIp:$dstPort HTTP/1.1\r\n")
            append("Host: $dstIp:$dstPort\r\n")
            append("Proxy-Connection: keep-alive\r\n")
            append("\r\n")
        }
        proxyOutput?.write(connectReq.toByteArray())
        proxyOutput?.flush()

        // Read CONNECT response
        val response = readLine(proxyInput!!)
        if (response == null || !response.contains("200")) {
            throw IOException("Proxy CONNECT failed: $response")
        }

        // Consume remaining headers until empty line
        while (true) {
            val line = readLine(proxyInput!!) ?: break
            if (line.isEmpty()) break
        }

        running.set(true)
        Log.d(TAG, "Relay $id: CONNECT tunnel established to $dstIp:$dstPort via $proxyHost:$proxyPort")
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

    fun close() {
        if (!running.compareAndSet(true, false)) return
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

    /**
     * Read a line (terminated by \r\n) from an InputStream.
     */
    private fun readLine(input: java.io.InputStream): String? {
        val sb = StringBuilder()
        while (true) {
            val b = input.read()
            if (b == -1) return if (sb.isNotEmpty()) sb.toString() else null
            if (b == 0x0D) { // \r
                val next = input.read()
                if (next == 0x0A) { // \n
                    return sb.toString()
                }
                sb.append(b.toChar())
                if (next != -1) sb.append(next.toChar())
            } else {
                sb.append(b.toChar())
            }
        }
    }
}
