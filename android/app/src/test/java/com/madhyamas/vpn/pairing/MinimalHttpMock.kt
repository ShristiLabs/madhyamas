package com.madhyamas.vpn.pairing

import java.io.InputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket

/**
 * Minimal HTTP/1.1 mock over ServerSocket (com.sun.net.httpserver is not
 * on the android.jar compile classpath, so unit tests cannot use it).
 * Handles exactly one request per accepted connection, closes after the
 * response (Connection: close).
 */
class MinimalHttpMock(
    private val status: Int,
    private val responseBody: String,
    private val capture: Boolean = true
) {
    private val server = ServerSocket(0, 8, InetAddress.getLoopbackAddress())
    private val running = java.util.concurrent.atomic.AtomicBoolean(true)

    @Volatile var lastBody: String? = null
        private set
    @Volatile var lastContentType: String? = null
        private set
    @Volatile var lastRequestLine: String? = null
        private set

    private val thread = Thread {
        while (running.get()) {
            val client = try {
                server.accept()
            } catch (e: Exception) {
                break
            }
            handle(client)
        }
    }.apply { isDaemon = true }

    fun start() = apply { thread.start() }

    val port: Int get() = server.localPort

    fun stop() {
        running.set(false)
        try {
            server.close()
        } catch (ignored: Exception) {
        }
    }

    private fun handle(client: Socket) {
        client.use { sock ->
            try {
                sock.soTimeout = 5_000
                val input = sock.getInputStream()
                val requestLine = readLine(input) ?: return
                val headers = HashMap<String, String>()
                while (true) {
                    val line = readLine(input) ?: break
                    if (line.isEmpty()) break
                    val idx = line.indexOf(':')
                    if (idx > 0) {
                        headers[line.substring(0, idx).trim().lowercase()] = line.substring(idx + 1).trim()
                    }
                }
                val contentLength = headers["content-length"]?.toIntOrNull() ?: 0
                val body = if (contentLength > 0) {
                    val buf = ByteArray(contentLength)
                    var off = 0
                    while (off < contentLength) {
                        val n = input.read(buf, off, contentLength - off)
                        if (n == -1) break
                        off += n
                    }
                    String(buf, 0, off, Charsets.UTF_8)
                } else ""

                if (capture) {
                    lastRequestLine = requestLine
                    lastBody = body
                    lastContentType = headers["content-type"]
                }

                val bytes = responseBody.toByteArray(Charsets.UTF_8)
                val head = "HTTP/1.1 $status ${reason(status)}\r\n" +
                    "Content-Type: application/json\r\n" +
                    "Content-Length: ${bytes.size}\r\n" +
                    "Connection: close\r\n\r\n"
                sock.getOutputStream().write((head.toByteArray(Charsets.ISO_8859_1)) + bytes)
                sock.getOutputStream().flush()
            } catch (ignored: Exception) {
                // Client-side aborts (e.g. the connection-refused test tearing
                // down) must not kill the accept loop.
            }
        }
    }

    private fun reason(status: Int) = when (status) {
        200 -> "OK"
        400 -> "Bad Request"
        401 -> "Unauthorized"
        500 -> "Internal Server Error"
        else -> "Status"
    }

    companion object {
        internal fun readLine(input: InputStream): String? {
            val sb = StringBuilder()
            while (true) {
                val b = input.read()
                if (b == -1) return if (sb.isNotEmpty()) sb.toString() else null
                if (b == 0x0D) {
                    val next = input.read()
                    if (next == 0x0A) return sb.toString()
                    sb.append(b.toChar())
                    if (next != -1) sb.append(next.toChar())
                } else {
                    sb.append(b.toChar())
                }
            }
        }
    }
}
