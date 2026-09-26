package com.madhyamas.vpn.pairing

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Developer smoke test for the deep-link parser (issue #111) — proves the
 * pure-JVM test harness compiles and runs without an emulator. The full
 * suite lives with the tester.
 */
class ConnectUriParserSmokeTest {

    private val tokenLink =
        "madhyamas://connect?host=proxy.example.com&port=8888&tls=1" +
            "&token=mdy_enroll_abcd1234&name=Hari%27s%20Pixel" +
            "&ca=http://proxy.example.com:3001/api/cert/ca" +
            "&api=http://proxy.example.com:3001/api"

    @Test
    fun parsesTokenLink() {
        val result = ConnectUriParser.parse(tokenLink)
        assertTrue(result is ConnectParseResult.Ok)
        val payload = (result as ConnectParseResult.Ok).payload
        assertEquals("proxy.example.com", payload.host)
        assertEquals(8888, payload.port)
        assertTrue(payload.tls)
        assertEquals("Hari's Pixel", payload.name)
        assertEquals("mdy_enroll_abcd1234", payload.token)
        assertNull(payload.key)
        assertTrue(payload.isEnrollment)
        assertEquals("http://proxy.example.com:3001/api", payload.apiUrl)
    }

    @Test
    fun parsesKeyLink() {
        val result = ConnectUriParser.parse(
            "madhyamas://connect?host=h&port=8888&tls=0&key=mdy_dev_0123456789abcdef"
        )
        val payload = (result as ConnectParseResult.Ok).payload
        assertFalse(payload.tls)
        assertEquals("mdy_dev_0123456789abcdef", payload.key)
        assertNull(payload.token)
        assertFalse(payload.isEnrollment)
    }

    @Test
    fun rejectsMissingHost() {
        val result = ConnectUriParser.parse("madhyamas://connect?port=8888&key=mdy_dev_x")
        assertTrue(result is ConnectParseResult.Error)
        assertTrue(
            (result as ConnectParseResult.Error).reason.contains("host")
        )
    }

    @Test
    fun rejectsTokenAndKeyTogether() {
        val result = ConnectUriParser.parse(
            "madhyamas://connect?host=h&port=1&token=mdy_enroll_a&key=mdy_dev_b"
        )
        assertNotNull((result as ConnectParseResult.Error).reason)
    }
}
