package com.madhyamas.vpn.pairing

import java.net.URI
import java.net.URLDecoder

/**
 * Parsed `madhyamas://connect` QR payload (issue #111).
 *
 * Payload shape (built by the web Devices panel, issue #106):
 * `madhyamas://connect?host=H&port=P&tls=0|1&token=mdy_enroll_...` or
 * `&key=mdy_dev_...`, plus `&name=`, `&ca=`, `&api=` (URL-encoded).
 *
 * Exactly one of [token] (enrollment exchange) and [key] (manual mode)
 * is non-null.
 */
data class ConnectPayload(
    val host: String,
    val port: Int,
    val tls: Boolean,
    val name: String?,
    val caUrl: String?,
    val apiUrl: String?,
    val token: String?,
    val key: String?
) {
    val isEnrollment: Boolean get() = token != null
}

sealed class ConnectParseResult {
    data class Ok(val payload: ConnectPayload) : ConnectParseResult()
    data class Error(val reason: String) : ConnectParseResult()
}

/**
 * Pure-JVM parser for the `madhyamas://connect` deep link.
 *
 * Malformed or incomplete links produce [ConnectParseResult.Error] with a
 * user-presentable reason — never an exception, never a crash. Validation
 * mirrors the server contract from issues #104/#106/#110:
 *  - `host` non-blank, `port` in 1..65535, `tls` strictly "0" or "1"
 *  - exactly one credential: `token` (`mdy_enroll_` prefix, redeemed via
 *    the enroll API) or `key` (`mdy_dev_` prefix, stored directly)
 *  - token links must carry `api` (the base URL of the enroll endpoint)
 *  - `ca`/`api`, when present, must be http(s) URLs
 */
object ConnectUriParser {

    const val SCHEME = "madhyamas"
    const val LINK_HOST = "connect"

    const val ENROLLMENT_TOKEN_PREFIX = "mdy_enroll_"
    const val DEVICE_KEY_PREFIX = "mdy_dev_"

    fun parse(raw: String?): ConnectParseResult {
        if (raw.isNullOrBlank()) {
            return ConnectParseResult.Error("Empty link")
        }
        val uri = try {
            URI(raw)
        } catch (e: Exception) {
            return ConnectParseResult.Error("Malformed link")
        }
        if (!uri.scheme.equals(SCHEME, ignoreCase = true)) {
            return ConnectParseResult.Error("Not a Madhyamas connect link")
        }
        if (uri.host?.equals(LINK_HOST, ignoreCase = true) != true) {
            return ConnectParseResult.Error("Not a Madhyamas connect link")
        }

        val params = parseQuery(uri.rawQuery)

        val host = params["host"]?.trim().orEmpty()
        if (host.isEmpty()) {
            return ConnectParseResult.Error("Link is missing the proxy host")
        }

        val port = params["port"]?.let { it.trim().toIntOrNull() }
            ?: return ConnectParseResult.Error("Link is missing the proxy port")
        if (port !in 1..65535) {
            return ConnectParseResult.Error("Proxy port out of range (1-65535)")
        }

        val tls = when (params["tls"] ?: "0") {
            "0" -> false
            "1" -> true
            else -> return ConnectParseResult.Error("tls must be 0 or 1")
        }

        val token = params["token"]?.trim()?.takeIf { it.isNotEmpty() }
        val key = params["key"]?.trim()?.takeIf { it.isNotEmpty() }
        when {
            token != null && key != null ->
                return ConnectParseResult.Error("Link carries both a token and a key")
            token == null && key == null ->
                return ConnectParseResult.Error("Link carries no credential (token or key)")
            token != null && !token.startsWith(ENROLLMENT_TOKEN_PREFIX) ->
                return ConnectParseResult.Error("Unexpected enrollment token format")
            key != null && !key.startsWith(DEVICE_KEY_PREFIX) ->
                return ConnectParseResult.Error("Unexpected device key format")
        }

        // A token link must say where to redeem it.
        if (token != null && params["api"].isNullOrBlank()) {
            return ConnectParseResult.Error("Token links must include the API URL")
        }

        val name = params["name"]?.trim()?.takeIf { it.isNotEmpty() }
        val caUrl = params["ca"]?.trim()?.takeIf { it.isNotEmpty() }?.let {
            if (!isHttpUrl(it)) return ConnectParseResult.Error("ca parameter is not an http(s) URL")
            it
        }
        val apiUrl = params["api"]?.trim()?.takeIf { it.isNotEmpty() }?.let {
            if (!isHttpUrl(it)) return ConnectParseResult.Error("api parameter is not an http(s) URL")
            it
        }

        return ConnectParseResult.Ok(
            ConnectPayload(
                host = host,
                port = port,
                tls = tls,
                name = name,
                caUrl = caUrl,
                apiUrl = apiUrl,
                token = token,
                key = key
            )
        )
    }

    internal fun parseQuery(rawQuery: String?): Map<String, String> {
        if (rawQuery.isNullOrEmpty()) return emptyMap()
        val params = LinkedHashMap<String, String>()
        for (pair in rawQuery.split('&')) {
            if (pair.isEmpty()) continue
            val eq = pair.indexOf('=')
            if (eq <= 0) continue
            val name = pair.substring(0, eq)
            val value = try {
                URLDecoder.decode(pair.substring(eq + 1), "UTF-8")
            } catch (e: Exception) {
                pair.substring(eq + 1)
            }
            params[name] = value
        }
        return params
    }

    private fun isHttpUrl(value: String): Boolean {
        val parsed = try {
            URI(value)
        } catch (e: Exception) {
            return false
        }
        return parsed.scheme == "http" || parsed.scheme == "https"
    }
}
