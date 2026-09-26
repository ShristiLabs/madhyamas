package com.madhyamas.vpn.pairing

import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * HttpUrlEnrollmentClient against a local mock of the enroll API
 * (issue #106 contract): POST {api}/devices/enroll {"token": "..."}.
 */
class EnrollmentClientTest {

    private var mock: MinimalHttpMock? = null

    private fun startMock(
        status: Int,
        responseBody: String =
            """{"device":{"id":"dev-1","name":"Pixel"},"key":"mdy_dev_0123456789abcdef"}""",
        capture: Boolean = true
    ): MinimalHttpMock = MinimalHttpMock(status, responseBody, capture)
        .start().also { mock = it }

    @After
    fun stopMock() {
        mock?.stop()
    }

    private fun client() = HttpUrlEnrollmentClient(connectTimeoutMs = 2_000, readTimeoutMs = 2_000)

    @Test
    fun happyPathParsesCredential() {
        val port = startMock(200).port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_tok")
        assertTrue("expected Success, got $result", result is EnrollmentResult.Success)
        val success = result as EnrollmentResult.Success
        assertEquals("mdy_dev_0123456789abcdef", success.deviceKey)
        assertEquals("dev-1", success.deviceId)
        assertEquals("Pixel", success.deviceName)
    }

    @Test
    fun postsToDevicesEnrollUnderTheApiBase() {
        val m = startMock(200)
        client().enroll("http://127.0.0.1:${m.port}/api", "mdy_enroll_x")
        assertEquals("POST /api/devices/enroll HTTP/1.1", m.lastRequestLine)
    }

    @Test
    fun requestCarriesExactlyTheTokenField() {
        val m = startMock(200)
        client().enroll("http://127.0.0.1:${m.port}/api", "mdy_enroll_tok_123")
        val json = JSONObject(m.lastBody!!)
        assertEquals(
            "enroll request must carry ONLY the token (server schema has no other field)",
            1,
            json.length()
        )
        assertEquals("mdy_enroll_tok_123", json.getString("token"))
        assertTrue(
            "Content-Type must be JSON, was ${m.lastContentType}",
            m.lastContentType!!.startsWith("application/json")
        )
    }

    @Test
    fun enrollUrlJoinsTrailingSlashVariants() {
        assertEquals(
            "http://h:3001/api/devices/enroll",
            client().enrollUrl("http://h:3001/api")
        )
        assertEquals(
            "http://h:3001/api/devices/enroll",
            client().enrollUrl("http://h:3001/api/")
        )
        assertEquals(
            "https://h.example/api/devices/enroll",
            client().enrollUrl("https://h.example/api")
        )
    }

    @Test
    fun status400MapsToMalformedToken() {
        val port = startMock(400, responseBody = "bad request").port
        val result = client().enroll("http://127.0.0.1:$port/api", "notatoken")
        assertEquals(
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.MALFORMED_TOKEN),
            result
        )
    }

    @Test
    fun status401MapsToInvalidToken() {
        val port = startMock(401, responseBody = "unauthorized").port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_used")
        assertEquals(
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.INVALID_TOKEN),
            result
        )
    }

    @Test
    fun status500MapsToServerError() {
        val port = startMock(500, responseBody = "boom").port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_x")
        assertEquals(
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.SERVER_ERROR),
            result
        )
    }

    @Test
    fun unreachableServerMapsToNetwork() {
        val m = startMock(200)
        val port = m.port
        m.stop()
        mock = null
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_x")
        assertEquals(
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.NETWORK),
            result
        )
    }

    @Test
    fun garbageJsonMapsToBadResponse() {
        val port = startMock(200, responseBody = "not json at all").port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_x")
        assertEquals(
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.BAD_RESPONSE),
            result
        )
    }

    @Test
    fun nonDeviceKeyInResponseMapsToBadResponse() {
        val port = startMock(
            200,
            responseBody = """{"device":{"id":"d"},"key":"not_a_device_key"}"""
        ).port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_x")
        assertEquals(
            "a 200 body must never store an unexpected credential",
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.BAD_RESPONSE),
            result
        )
    }

    @Test
    fun missingKeyInResponseMapsToBadResponse() {
        val port = startMock(200, responseBody = """{"device":{"id":"d"}}""").port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_x")
        assertEquals(
            EnrollmentResult.Failure(EnrollmentResult.Failure.Reason.BAD_RESPONSE),
            result
        )
    }

    @Test
    fun responseWithoutDeviceStillSucceedsWithNullMetadata() {
        val port = startMock(200, responseBody = """{"key":"mdy_dev_abc"}""").port
        val result = client().enroll("http://127.0.0.1:$port/api", "mdy_enroll_x")
        assertTrue(result is EnrollmentResult.Success)
        val success = result as EnrollmentResult.Success
        assertEquals("mdy_dev_abc", success.deviceKey)
        assertEquals(null, success.deviceId)
        assertEquals(null, success.deviceName)
    }
}
