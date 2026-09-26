package com.madhyamas.vpn.pairing

import org.json.JSONObject
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL

/**
 * Outcome of redeeming an enrollment token (issue #111). On success the
 * server returns the long-lived `mdy_dev_` key exactly once.
 */
sealed class EnrollmentResult {
    data class Success(
        val deviceId: String?,
        val deviceName: String?,
        val deviceKey: String
    ) : EnrollmentResult()

    data class Failure(val reason: Reason) : EnrollmentResult() {
        enum class Reason {
            /** Server rejected the token: unknown, expired, already used, or revoked (indistinguishable by design). */
            INVALID_TOKEN,
            /** Token not shaped like an enrollment credential (HTTP 400). */
            MALFORMED_TOKEN,
            /** Any other non-200 server response. */
            SERVER_ERROR,
            /** Connection/timeout failure. */
            NETWORK,
            /** 200 with an unparseable body or a key that is not mdy_dev_-shaped. */
            BAD_RESPONSE
        }
    }
}

/**
 * Redeems the QR's single-use `mdy_enroll_` token via the public enroll
 * endpoint from issue #106. The request body carries ONLY the token — the
 * server schema (`EnrollDeviceRequest`) has no other field (no
 * install_uuid; recorded as a follow-up).
 */
interface EnrollmentClient {
    fun enroll(apiBaseUrl: String, token: String): EnrollmentResult
}

/**
 * [EnrollmentClient] over [HttpURLConnection] — the app's existing HTTP
 * convention (see CertInstallActivity); no new runtime dependencies.
 * Follows the `api` base URL from the QR payload verbatim (http or https).
 */
class HttpUrlEnrollmentClient(
    private val connectTimeoutMs: Int = 10_000,
    private val readTimeoutMs: Int = 20_000
) : EnrollmentClient {

    override fun enroll(apiBaseUrl: String, token: String): EnrollmentResult {
        val url = try {
            URL(enrollUrl(apiBaseUrl))
        } catch (e: Exception) {
            return EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.BAD_RESPONSE)
        }
        var conn: HttpURLConnection? = null
        return try {
            conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "POST"
                connectTimeout = connectTimeoutMs
                readTimeout = readTimeoutMs
                doOutput = true
                setRequestProperty("Content-Type", "application/json")
                setRequestProperty("Accept", "application/json")
            }
            val body = JSONObject().put("token", token).toString()
            conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            when (conn.responseCode) {
                200 -> parseResponse(conn.inputStream.readBytes().toString(Charsets.UTF_8))
                400 -> EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.MALFORMED_TOKEN)
                401 -> EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.INVALID_TOKEN)
                else -> EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.SERVER_ERROR)
            }
        } catch (e: IOException) {
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.NETWORK)
        } catch (e: Exception) {
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.NETWORK)
        } finally {
            conn?.disconnect()
        }
    }

    /** `http://host:3001/api` -> `http://host:3001/api/devices/enroll`. */
    internal fun enrollUrl(apiBaseUrl: String): String =
        apiBaseUrl.trim().trimEnd('/') + "/devices/enroll"

    /**
     * Parses the `DeviceWithKey` response: `{"device":{"id","name",...},"key":"mdy_dev_..."}`.
     * A 200 body whose key is not mdy_dev_-shaped is treated as a bad
     * response (defensive — never store an unexpected credential).
     */
    internal fun parseResponse(body: String): EnrollmentResult {
        val json = try {
            JSONObject(body)
        } catch (e: Exception) {
            return EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.BAD_RESPONSE)
        }
        val key = json.optString("key")
        if (!key.startsWith(ConnectUriParser.DEVICE_KEY_PREFIX)) {
            return EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.BAD_RESPONSE)
        }
        val device = json.optJSONObject("device")
        val deviceId = device?.optString("id")?.takeIf { it.isNotBlank() }
        val deviceName = device?.optString("name")?.takeIf { it.isNotBlank() }
        return EnrollmentResult.Success(
            deviceId = deviceId,
            deviceName = deviceName,
            deviceKey = key
        )
    }
}
