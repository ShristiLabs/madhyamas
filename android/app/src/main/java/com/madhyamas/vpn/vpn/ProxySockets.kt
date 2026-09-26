package com.madhyamas.vpn.vpn

import java.net.Socket
import javax.net.ssl.SSLSocketFactory

/**
 * Creates the (unconnected) socket used to reach the Madhyamas proxy.
 *
 * When the QR payload carried `tls=1` (issue #110's TLS-wrapped listener),
 * the proxy connection is an SSLSocket from the platform default factory.
 * On Android the default factory is NetworkSecurityConfig-aware, so both
 * system and user-installed CAs are trusted anchors. Hostname verification
 * against the configured proxy host happens in TcpRelay after the
 * handshake; a TLS failure is surfaced as [ConnectState.TLS_ERROR] — the
 * tunnel never silently falls back to plaintext.
 *
 * Note: the QR's `ca=` URL is the Madhyamas MITM CA (for HTTPS
 * interception of the *monitored* apps) — it is NOT the listener
 * certificate; listener TLS uses a normal server certificate verified
 * against the platform trust store.
 */
object ProxySockets {

    fun create(useTls: Boolean): Socket {
        return if (useTls) {
            (SSLSocketFactory.getDefault() as SSLSocketFactory).createSocket()
        } else {
            Socket()
        }
    }
}
