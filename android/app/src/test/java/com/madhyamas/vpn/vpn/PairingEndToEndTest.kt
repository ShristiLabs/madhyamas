package com.madhyamas.vpn.vpn

import com.madhyamas.vpn.pairing.CredentialStore
import com.madhyamas.vpn.pairing.EnrollmentResult
import com.madhyamas.vpn.pairing.FakeAead
import com.madhyamas.vpn.pairing.HttpUrlEnrollmentClient
import com.madhyamas.vpn.pairing.InMemoryKeyValueStore
import com.madhyamas.vpn.pairing.MinimalHttpMock
import com.madhyamas.vpn.pairing.ProxyAuth
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.InputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.Base64
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit

/**
 * Definition-of-done probe (JVM level): a QR-style enrollment against a
 * mock enroll API produces a stored credential whose Proxy-Authorization
 * header arrives on the CONNECT the companion authors toward the proxy —
 * the full pairing -> injection chain without an emulator.
 */
class PairingEndToEndTest {

    @Test
    fun enrollmentResultIsStoredSealedAndInjectedIntoTheConnect() {
        // 1. Mock enroll API (issue #106 contract).
        val deviceKey = "mdy_dev_e2e0123456789abcdef0123456789ab"
        val api = MinimalHttpMock(
            200,
            """{"device":{"id":"e2e-1","name":"E2E Pixel"},"key":"$deviceKey"}"""
        ).start()
        try {
            // 2. Enrollment exchange (token -> long-lived key).
            val enrolled = HttpUrlEnrollmentClient()
                .enroll("http://127.0.0.1:${api.port}/api", "mdy_enroll_e2e")
            assertTrue("expected Success, got $enrolled", enrolled is EnrollmentResult.Success)
            val success = enrolled as EnrollmentResult.Success

            // 3. Sealed storage + read-back (the service's start-of-day read).
            val kv = InMemoryKeyValueStore()
            val store = CredentialStore(kv, FakeAead())
            store.store(success.deviceKey, success.deviceId, success.deviceName)
            val storedKey = store.deviceKey()
            assertEquals(deviceKey, storedKey)

            // 4. The proxy mock asserts the CONNECT carries OUR header.
            val proxy = ServerSocket(0, 1, InetAddress.getLoopbackAddress())
            val received: CompletableFuture<String> = CompletableFuture()
            Thread {
                proxy.accept().use { sock ->
                    sock.soTimeout = 5_000
                    received.complete(readUntilHeadersEnd(sock.getInputStream()))
                    sock.getOutputStream().write("HTTP/1.1 200 Connection Established\r\n\r\n".toByteArray())
                    Thread.sleep(200)
                }
            }.start()
            try {
                Socket(proxy.inetAddress, proxy.localPort).use { client ->
                    client.soTimeout = 5_000
                    val auth = ProxyAuth.basicHeaderValue(storedKey!!)
                    val result = ProxyTunnelHandshake.perform(
                        client.getOutputStream(), client.getInputStream(),
                        "93.184.216.34", 443, auth
                    )
                    assertTrue("expected Established, got $result", result is TunnelResult.Established)
                }
                val connect = received.get(5, TimeUnit.SECONDS)
                assertTrue(connect.startsWith("CONNECT 93.184.216.34:443 HTTP/1.1\r\n"))
                val expected = "Basic " + Base64.getEncoder()
                    .encodeToString("$deviceKey:".toByteArray(Charsets.UTF_8))
                assertTrue(
                    "CONNECT must carry the injected Proxy-Authorization",
                    connect.contains("Proxy-Authorization: $expected\r\n")
                )
            } finally {
                proxy.close()
            }
        } finally {
            api.stop()
        }
    }

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
