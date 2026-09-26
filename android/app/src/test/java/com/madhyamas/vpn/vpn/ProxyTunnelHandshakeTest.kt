package com.madhyamas.vpn.vpn

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.io.OutputStream
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.TimeUnit

/**
 * The companion-authored CONNECT handshake (issue #111): request framing,
 * Proxy-Authorization injection, and status-line classification against a
 * real local socket mock.
 */
class ProxyTunnelHandshakeTest {

    // ---------- request construction ----------

    @Test
    fun buildRequestWithoutAuthHasExactFraming() {
        val request = ProxyTunnelHandshake.buildRequest("1.2.3.4", 443, null)
        assertEquals(
            "CONNECT 1.2.3.4:443 HTTP/1.1\r\n" +
                "Host: 1.2.3.4:443\r\n" +
                "Proxy-Connection: keep-alive\r\n" +
                "\r\n",
            request
        )
    }

    @Test
    fun buildRequestCarriesTheAuthorizationHeader() {
        val request = ProxyTunnelHandshake.buildRequest("93.184.216.34", 443, "Basic a2V5Ogo=")
        assertTrue(request.contains("Proxy-Authorization: Basic a2V5Ogo=\r\n"))
        // Header sits between Host and Proxy-Connection.
        val lines = request.split("\r\n")
        assertTrue(lines.indexOf("Host: 93.184.216.34:443") < lines.indexOf("Proxy-Authorization: Basic a2V5Ogo="))
        assertTrue(request.endsWith("Proxy-Connection: keep-alive\r\n\r\n"))
    }

    // ---------- status-line parsing ----------

    @Test
    fun statusCodeParsesVersionVariants() {
        assertEquals(200, ProxyTunnelHandshake.statusCode("HTTP/1.1 200 Connection Established"))
        assertEquals(407, ProxyTunnelHandshake.statusCode("HTTP/1.0 407 Proxy Authentication Required"))
        assertEquals(502, ProxyTunnelHandshake.statusCode("HTTP/1.1 502 Bad Gateway"))
        assertEquals(200, ProxyTunnelHandshake.statusCode("HTTP/1.1 200"))
    }

    @Test
    fun statusCodeRejectsGarbage() {
        assertNull(ProxyTunnelHandshake.statusCode("garbage"))
        assertNull(ProxyTunnelHandshake.statusCode("200 OK")) // no HTTP-version token
        assertNull(ProxyTunnelHandshake.statusCode(""))
    }

    // ---------- readLine ----------

    @Test
    fun readLineHandlesCrLfAndEof() {
        val input: InputStream = ByteArrayInputStream("first\r\nsecond\r\n".toByteArray())
        assertEquals("first", ProxyTunnelHandshake.readLine(input))
        assertEquals("second", ProxyTunnelHandshake.readLine(input))
        assertNull(ProxyTunnelHandshake.readLine(input))
    }

    @Test
    fun readLineReturnsPartialLineOnEof() {
        val input: InputStream = ByteArrayInputStream("partial".toByteArray())
        assertEquals("partial", ProxyTunnelHandshake.readLine(input))
    }

    // ---------- perform() over streams ----------

    @Test
    fun performEstablishesOn200AndConsumesHeadersExactly() {
        // Response headers followed by immediate tunnel payload: after
        // Established, the payload byte must still be readable (header
        // consumption must not over-read).
        val input: InputStream = ByteArrayInputStream(
            "HTTP/1.1 200 Connection Established\r\nX-Foo: bar\r\n\r\nZ".toByteArray()
        )
        val output = ByteArrayOutputStream()
        val result = ProxyTunnelHandshake.perform(output, input, "h", 443, null)
        assertTrue(result is TunnelResult.Established)
        assertEquals('Z'.code, input.read())
        assertTrue(output.toString().startsWith("CONNECT h:443 HTTP/1.1\r\n"))
    }

    @Test
    fun performRejectsOn407() {
        val input: InputStream = ByteArrayInputStream(
            "HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic\r\n\r\n".toByteArray()
        )
        val result = ProxyTunnelHandshake.perform(
            ByteArrayOutputStream(), input, "h", 443, "Basic a2V5Ogo="
        )
        assertTrue(result is TunnelResult.Rejected)
    }

    @Test
    fun performFailsOnOtherStatus() {
        val input: InputStream = ByteArrayInputStream("HTTP/1.1 502 Bad Gateway\r\n\r\n".toByteArray())
        val result = ProxyTunnelHandshake.perform(ByteArrayOutputStream(), input, "h", 443, null)
        assertTrue(result is TunnelResult.Failed)
        assertEquals("HTTP/1.1 502 Bad Gateway", (result as TunnelResult.Failed).statusLine)
    }

    @Test
    fun performIoErrorOnImmediateClose() {
        val input: InputStream = ByteArrayInputStream(ByteArray(0))
        val result = ProxyTunnelHandshake.perform(ByteArrayOutputStream(), input, "h", 443, null)
        assertTrue(result is TunnelResult.IoError)
    }

    // ---------- perform() against a real socket mock ----------

    /** Accepts one connection, reads the CONNECT, replies, returns the request text. */
    private class MockProxy(private val response: String, private val thenWrite: ByteArray? = null) {
        val server = ServerSocket(0, 1, java.net.InetAddress.getLoopbackAddress())
        val received = java.util.concurrent.CompletableFuture<String>()
        private val thread = Thread {
            server.accept().use { sock ->
                sock.soTimeout = 5_000
                val text = readUntilHeadersEnd(sock.getInputStream())
                received.complete(text)
                sock.getOutputStream().write(response.toByteArray(Charsets.ISO_8859_1))
                if (thenWrite != null) sock.getOutputStream().write(thenWrite)
                sock.getOutputStream().flush()
                if (thenWrite == null) {
                    // Hold the socket briefly so the client can read the response.
                    Thread.sleep(300)
                }
            }
        }

        fun start() = apply { thread.start() }

        private fun readUntilHeadersEnd(input: InputStream): String {
            val sb = StringBuilder()
            while (!sb.endsWith("\r\n\r\n")) {
                val b = input.read()
                if (b == -1) break
                sb.append(b.toChar())
            }
            return sb.toString()
        }
    }

    @Test
    fun socketLevelEstablishedAndRelayPayloadFlows() {
        val mock = MockProxy(
            "HTTP/1.1 200 Connection Established\r\n\r\n",
            thenWrite = "PAYLOAD".toByteArray()
        ).start()
        Socket(mock.server.inetAddress, mock.server.localPort).use { client ->
            client.soTimeout = 5_000
            val result = ProxyTunnelHandshake.perform(
                client.getOutputStream(), client.getInputStream(), "h", 443, null
            )
            assertTrue("expected Established, got $result", result is TunnelResult.Established)
            // The bytes the proxy pushed after the 200 arrive post-handshake.
            val buf = ByteArray(7)
            var off = 0
            while (off < buf.size) {
                val n = client.getInputStream().read(buf, off, buf.size - off)
                if (n == -1) break
                off += n
            }
            assertEquals("PAYLOAD", String(buf, 0, off))
        }
        assertTrue(mock.received.get(5, TimeUnit.SECONDS).startsWith("CONNECT h:443 HTTP/1.1\r\n"))
    }

    @Test
    fun socketLevelRejectedOn407() {
        val mock = MockProxy("HTTP/1.1 407 Proxy Authentication Required\r\n\r\n").start()
        Socket(mock.server.inetAddress, mock.server.localPort).use { client ->
            client.soTimeout = 5_000
            val result = ProxyTunnelHandshake.perform(
                client.getOutputStream(), client.getInputStream(), "h", 443, "Basic a2V5Ogo="
            )
            assertTrue(result is TunnelResult.Rejected)
        }
        assertTrue(mock.received.get(5, TimeUnit.SECONDS).contains("Proxy-Authorization: Basic a2V5Ogo="))
    }
}
