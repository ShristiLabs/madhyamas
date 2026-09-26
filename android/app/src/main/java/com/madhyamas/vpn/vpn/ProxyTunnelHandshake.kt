package com.madhyamas.vpn.vpn

import java.io.IOException
import java.io.InputStream
import java.io.OutputStream

/**
 * Coarse per-connection outcome surfaced to the VPN service (and from
 * there to the UI's status card) — issue #111.
 */
enum class ConnectState {
    /** CONNECT succeeded — traffic is being captured. */
    OK,
    /** HTTP 407 — the proxy refused the device credential (revoked or wrong key). */
    REJECTED,
    /** Proxy answered with another non-200 status. */
    FAILED,
    /** Could not reach the proxy (connect/IO error). */
    UNREACHABLE,
    /** TLS handshake or certificate verification failed on the proxy connection. */
    TLS_ERROR
}

/** Result of performing the HTTP CONNECT handshake with the proxy. */
sealed class TunnelResult {
    data object Established : TunnelResult()
    data object Rejected : TunnelResult()
    data class Failed(val statusLine: String?) : TunnelResult()
    data class IoError(val message: String?) : TunnelResult()
}

/**
 * The proxy-side half of the tunnel setup, extracted from TcpRelay so the
 * protocol logic is unit-testable against a plain socket pair (pure JVM).
 *
 * The companion AUTHORS this CONNECT request itself (it re-originates the
 * app's TCP connections to the proxy), so the [proxyAuthorization] value —
 * built from the Keystore-stored device credential — is the only
 * Proxy-Authorization the proxy ever sees on this connection; credentials
 * an app might attach to its own (tunneled) requests never reach the
 * proxy's CONNECT parser.
 *
 * A 407 response maps to [TunnelResult.Rejected]: the caller closes the
 * connection and reports [ConnectState.REJECTED] — it never retries with
 * the same dead credential.
 */
object ProxyTunnelHandshake {

    fun buildRequest(dstHost: String, dstPort: Int, proxyAuthorization: String?): String =
        buildString {
            append("CONNECT $dstHost:$dstPort HTTP/1.1\r\n")
            append("Host: $dstHost:$dstPort\r\n")
            if (proxyAuthorization != null) {
                append("Proxy-Authorization: $proxyAuthorization\r\n")
            }
            append("Proxy-Connection: keep-alive\r\n")
            append("\r\n")
        }

    fun perform(
        output: OutputStream,
        input: InputStream,
        dstHost: String,
        dstPort: Int,
        proxyAuthorization: String?
    ): TunnelResult {
        return try {
            output.write(buildRequest(dstHost, dstPort, proxyAuthorization).toByteArray(Charsets.ISO_8859_1))
            output.flush()
            val statusLine = readLine(input) ?: return TunnelResult.IoError(null)
            when (statusCode(statusLine)) {
                200 -> {
                    consumeHeaders(input)
                    TunnelResult.Established
                }
                407 -> TunnelResult.Rejected
                else -> TunnelResult.Failed(statusLine)
            }
        } catch (e: IOException) {
            TunnelResult.IoError(e.message)
        }
    }

    /** `HTTP/1.1 200 Connection Established` -> 200; null when unparseable. */
    internal fun statusCode(statusLine: String): Int? {
        val parts = statusLine.trim().split(' ')
        if (parts.size < 2) return null
        return try {
            parts[1].toInt()
        } catch (e: NumberFormatException) {
            null
        }
    }

    /** Consume response headers up to the blank line (kept from TcpRelay). */
    private fun consumeHeaders(input: InputStream) {
        while (true) {
            val line = readLine(input) ?: break
            if (line.isEmpty()) break
        }
    }

    /**
     * Read a CRLF-terminated line from an InputStream (kept from TcpRelay,
     * including its lone-CR tolerance).
     */
    internal fun readLine(input: InputStream): String? {
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
