package com.madhyamas.vpn.vpn

import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import com.madhyamas.vpn.MainActivity
import com.madhyamas.vpn.MadhyamasApp
import com.madhyamas.vpn.R
import java.io.FileInputStream
import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

/**
 * VPN service that captures TCP traffic from selected apps and forwards
 * it to the Madhyamas proxy via HTTP CONNECT.
 *
 * Architecture:
 *  1. VpnService creates a TUN interface that captures IP packets
 *  2. We parse IP/TCP headers to extract source port and destination
 *  3. For each new TCP connection, we create a [TcpRelay] that:
 *     a. Connects to the Madhyamas proxy
 *     b. Sends an HTTP CONNECT request for the destination host:port
 *     c. Waits for "200 Connection Established"
 *     d. Relays data bidirectionally between the app and the proxy
 *  4. For apps that bypass the proxy (e.g. the proxy itself), we
 *     allowlist them by UID so their traffic passes through directly
 */
class MadhyamasVpnService : VpnService() {

    companion object {
        private const val TAG = "MadhyamasVpn"
        private const val NOTIFICATION_ID = 1
        private const val VPN_ADDRESS = "10.0.0.2"
        private const val VPN_ROUTE = "0.0.0.0"
        private const val VPN_MTU = 1500
        private const val BUFFER_SIZE = 32767

        // Intent actions
        const val ACTION_START = "com.madhyamas.vpn.START"
        const val ACTION_STOP = "com.madhyamas.vpn.STOP"
        const val ACTION_UPDATE_CONFIG = "com.madhyamas.vpn.UPDATE_CONFIG"

        // Intent extras
        const val EXTRA_PROXY_HOST = "proxy_host"
        const val EXTRA_PROXY_PORT = "proxy_port"
        const val EXTRA_ALLOWED_PACKAGES = "allowed_packages"
        const val EXTRA_DISALLOWED_PACKAGES = "disallowed_packages"

        // Singleton instance for UI to check status
        @Volatile
        var instance: MadhyamasVpnService? = null
            private set
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var relayThread: Thread? = null
    private val running = AtomicBoolean(false)
    private val activeRelays = ConcurrentHashMap<Int, TcpRelay>()
    private val connectionCounter = AtomicInteger(0)

    // Proxy configuration
    private var proxyHost: String = "127.0.0.1"
    private var proxyPort: Int = 8888

    // Package filtering
    private var allowedPackages: Set<String> = emptySet() // empty = all apps
    private var disallowedPackages: Set<String> = emptySet()

    // Stats
    @Volatile var totalConnections: Int = 0
        private set
    @Volatile var activeConnections: Int = 0
        private set
    @Volatile var bytesSent: Long = 0
        private set
    @Volatile var bytesReceived: Long = 0
        private set

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> {
                proxyHost = intent.getStringExtra(EXTRA_PROXY_HOST) ?: proxyHost
                proxyPort = intent.getIntExtra(EXTRA_PROXY_PORT, proxyPort)
                allowedPackages = intent.getStringArrayExtra(EXTRA_ALLOWED_PACKAGES)?.toSet() ?: emptySet()
                disallowedPackages = intent.getStringArrayExtra(EXTRA_DISALLOWED_PACKAGES)?.toSet() ?: emptySet()
                startVpn()
            }
            ACTION_STOP -> {
                stopVpn()
                stopSelf()
            }
            ACTION_UPDATE_CONFIG -> {
                proxyHost = intent.getStringExtra(EXTRA_PROXY_HOST) ?: proxyHost
                proxyPort = intent.getIntExtra(EXTRA_PROXY_PORT, proxyPort)
                allowedPackages = intent.getStringArrayExtra(EXTRA_ALLOWED_PACKAGES)?.toSet() ?: emptySet()
                disallowedPackages = intent.getStringArrayExtra(EXTRA_DISALLOWED_PACKAGES)?.toSet() ?: emptySet()
                // No need to restart VPN — relay reads config dynamically
            }
        }
        return START_STICKY
    }

    private fun startVpn() {
        if (running.get()) {
            Log.w(TAG, "VPN already running")
            return
        }

        startForeground()

        try {
            val builder = Builder()
                .setSession("Madhyamas VPN")
                .setMtu(VPN_MTU)
                .addAddress(VPN_ADDRESS, 32)
                .addRoute(VPN_ROUTE, 0)
                .setBlocking(true)

            // Per-app VPN: restrict to allowed packages or exclude disallowed
            val pm = packageManager
            if (allowedPackages.isNotEmpty()) {
                for (pkg in allowedPackages) {
                    try {
                        val uid = pm.getPackageUid(pkg, 0)
                        builder.addAllowedApplication(pkg)
                    } catch (e: Exception) {
                        Log.w(TAG, "Package not found: $pkg")
                    }
                }
            } else if (disallowedPackages.isNotEmpty()) {
                for (pkg in disallowedPackages) {
                    try {
                        builder.addDisallowedApplication(pkg)
                    } catch (e: Exception) {
                        Log.w(TAG, "Package not found: $pkg")
                    }
                }
            }

            // Always exclude self from VPN to avoid loops
            builder.addDisallowedApplication(packageName)

            vpnInterface = builder.establish()

            if (vpnInterface == null) {
                Log.e(TAG, "Failed to establish VPN interface (user may have revoked permission)")
                stopSelf()
                return
            }

            running.set(true)
            instance = this
            Log.i(TAG, "VPN started — proxy at $proxyHost:$proxyPort")

            // Start the packet processing thread
            relayThread = Thread({ processPackets() }, "MadhyamasVpn-Relay").apply {
                isDaemon = true
                start()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start VPN", e)
            stopSelf()
        }
    }

    /**
     * Main packet processing loop. Reads IP packets from the TUN interface,
     * parses TCP headers, and creates/manages [TcpRelay] instances for each
     * connection.
     */
    private fun processPackets() {
        val pfd = vpnInterface ?: return
        val input = FileInputStream(pfd.fileDescriptor)
        val buffer = ByteBuffer.allocate(BUFFER_SIZE)

        while (running.get()) {
            try {
                buffer.clear()
                val length = input.read(buffer.array())
                if (length <= 0) continue

                buffer.limit(length)

                // Parse IP header
                val ipVersion = (buffer.get(0).toInt() shr 4) and 0x0F
                if (ipVersion != 4) continue // Only IPv4 for now

                val ihl = (buffer.get(0).toInt() and 0x0F) * 4
                if (length < ihl + 20) continue // Too short for IP + TCP header

                val protocol = buffer.get(9).toInt() and 0xFF
                if (protocol != 6) continue // Only TCP (protocol 6)

                // Extract source and destination
                val srcIp = formatIp(buffer, 12)
                val dstIp = formatIp(buffer, 16)
                val srcPort = ((buffer.get(ihl).toInt() and 0xFF) shl 8) or (buffer.get(ihl + 1).toInt() and 0xFF)
                val dstPort = ((buffer.get(ihl + 2).toInt() and 0xFF) shl 8) or (buffer.get(ihl + 3).toInt() and 0xFF)

                val tcpFlags = buffer.get(ihl + 13).toInt() and 0xFF
                val isSyn = (tcpFlags and 0x02) != 0
                val isFin = (tcpFlags and 0x01) != 0
                val isRst = (tcpFlags and 0x04) != 0

                val connKey = srcPort // Source port uniquely identifies a connection

                when {
                    isSyn -> {
                        // New connection — create a relay
                        val relayId = connectionCounter.incrementAndGet()
                        val relay = TcpRelay(
                            id = relayId,
                            srcPort = connKey,
                            dstIp = dstIp,
                            dstPort = dstPort,
                            proxyHost = proxyHost,
                            proxyPort = proxyPort,
                            vpnService = this,
                            onClose = { id ->
                                activeRelays.remove(srcPort)
                                activeConnections = activeRelays.size
                            },
                            onBytesSent = { n -> bytesSent += n },
                            onBytesReceived = { n -> bytesReceived += n }
                        )
                        activeRelays[srcPort] = relay
                        totalConnections++
                        activeConnections = activeRelays.size
                        relay.start()

                        // Send SYN-ACK back to the app so it thinks the connection is established
                        writePacket(pfd, buffer.array(), length, isResponse = true, srcPort = connKey, dstPort = srcPort, flags = 0x12) // SYN-ACK
                    }
                    isFin || isRst -> {
                        // Connection closing
                        activeRelays[connKey]?.close()
                        activeRelays.remove(connKey)
                        activeConnections = activeRelays.size
                    }
                    else -> {
                        // Data packet — forward to the relay
                        val relay = activeRelays[connKey]
                        if (relay != null) {
                            val dataLen = length - ihl - 20 // Subtract IP + TCP headers
                            if (dataLen > 0) {
                                val data = ByteArray(dataLen)
                                System.arraycopy(buffer.array(), ihl + 20, data, 0, dataLen)
                                relay.writeToProxy(data)
                            }
                        }
                    }
                }
            } catch (e: Exception) {
                if (running.get()) {
                    Log.e(TAG, "Error processing packet", e)
                }
            }
        }
    }

    /**
     * Write a packet back to the TUN interface (response from proxy to app).
     */
    fun writeToVpn(data: ByteArray, srcPort: Int, dstPort: Int) {
        try {
            val pfd = vpnInterface ?: return
            val packet = buildTcpPacket(srcPort, dstPort, data)
            pfd.fileDescriptor.let { fd ->
                val out = java.io.FileOutputStream(fd)
                out.write(packet)
                out.flush()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error writing to VPN", e)
        }
    }

    /**
     * Build a minimal TCP/IP packet for response data.
     * This is a simplified implementation — in production, proper TCP
     * state tracking (sequence numbers, ACKs, windowing) is needed.
     */
    private fun buildTcpPacket(srcPort: Int, dstPort: Int, data: ByteArray): ByteArray {
        val ipHeaderLen = 20
        val tcpHeaderLen = 20
        val totalLen = ipHeaderLen + tcpHeaderLen + data.size

        val packet = ByteArray(totalLen)
        val buf = ByteBuffer.wrap(packet)

        // IP header
        buf.put(0x45.toByte()) // IPv4, IHL=5
        buf.put(0) // DSCP/ECN
        buf.putShort(totalLen.toShort()) // Total length
        buf.putShort(0) // Identification
        buf.putShort(0x4000.toShort()) // Flags: Don't fragment
        buf.put(64) // TTL
        buf.put(6) // Protocol: TCP
        buf.putShort(0) // Checksum (0 = let OS fill)
        // Source IP = VPN gateway (10.0.0.2)
        buf.put(byteArrayOf(10, 0, 0, 2))
        // Destination IP = app (10.0.0.1) — simplified
        buf.put(byteArrayOf(10, 0, 0, 1))

        // TCP header
        buf.putShort(srcPort.toShort())
        buf.putShort(dstPort.toShort())
        buf.putInt(0) // Sequence number (simplified)
        buf.putInt(0) // Ack number (simplified)
        buf.put((5 shl 4).toByte()) // Data offset = 5 (20 bytes)
        buf.put(0x18.toByte()) // Flags: PSH+ACK
        buf.putShort(0xFFFF.toShort()) // Window size
        buf.putShort(0) // Checksum (0 = let OS fill)
        buf.putShort(0) // Urgent pointer

        // Data
        buf.put(data)

        return packet
    }

    private fun writePacket(pfd: ParcelFileDescriptor, data: ByteArray, length: Int, isResponse: Boolean, srcPort: Int, dstPort: Int, flags: Int) {
        // Simplified — proper implementation would build TCP response packets
        // with correct sequence numbers and ACKs
    }

    private fun formatIp(buf: ByteBuffer, offset: Int): String {
        return "${buf.get(offset).toInt() and 0xFF}." +
               "${buf.get(offset + 1).toInt() and 0xFF}." +
               "${buf.get(offset + 2).toInt() and 0xFF}." +
               "${buf.get(offset + 3).toInt() and 0xFF}"
    }

    private fun startForeground() {
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification = NotificationCompat.Builder(this, MadhyamasApp.CHANNEL_VPN)
            .setContentTitle("Madhyamas VPN Active")
            .setContentText("Forwarding to $proxyHost:$proxyPort")
            .setSmallIcon(android.R.drawable.ic_menu_compass)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun stopVpn() {
        running.set(false)
        instance = null

        // Close all active relays
        for (relay in activeRelays.values) {
            relay.close()
        }
        activeRelays.clear()
        activeConnections = 0

        // Stop packet processing
        relayThread?.interrupt()
        relayThread = null

        // Close VPN interface
        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            Log.w(TAG, "Error closing VPN interface", e)
        }
        vpnInterface = null

        // Cancel notification
        getSystemService(NotificationManager::class.java)
            .cancel(NOTIFICATION_ID)

        Log.i(TAG, "VPN stopped")
    }

    override fun onDestroy() {
        stopVpn()
        super.onDestroy()
    }

    override fun onRevoke() {
        // Called when the user disables the VPN via system settings
        stopVpn()
        stopSelf()
    }
}
