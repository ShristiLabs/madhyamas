package com.madhyamas.vpn.pairing

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Full validation matrix for the madhyamas://connect deep link (issue #111). */
class ConnectUriParserTest {

    private fun ok(link: String): ConnectPayload {
        val result = ConnectUriParser.parse(link)
        assertTrue("expected Ok for $link, got $result", result is ConnectParseResult.Ok)
        return (result as ConnectParseResult.Ok).payload
    }

    private fun err(link: String?): String {
        val result = ConnectUriParser.parse(link)
        assertTrue("expected Error for $link, got $result", result is ConnectParseResult.Error)
        return (result as ConnectParseResult.Error).reason
    }

    // ---------- happy paths ----------

    @Test
    fun parsesFullTokenLink() {
        val payload = ok(
            "madhyamas://connect?host=proxy.example.com&port=8888&tls=1" +
                "&token=mdy_enroll_0123456789abcdef&name=Hari%27s%20Pixel" +
                "&ca=http://proxy.example.com:3001/api/cert/ca" +
                "&api=http://proxy.example.com:3001/api"
        )
        assertEquals("proxy.example.com", payload.host)
        assertEquals(8888, payload.port)
        assertTrue(payload.tls)
        assertEquals("Hari's Pixel", payload.name)
        assertEquals("mdy_enroll_0123456789abcdef", payload.token)
        assertNull(payload.key)
        assertTrue(payload.isEnrollment)
        assertEquals("http://proxy.example.com:3001/api/cert/ca", payload.caUrl)
        assertEquals("http://proxy.example.com:3001/api", payload.apiUrl)
    }

    @Test
    fun parsesKeyLinkManualMode() {
        val payload = ok(
            "madhyamas://connect?host=10.0.0.5&port=18888&tls=0&key=mdy_dev_aaaaaaaaaaaaaaaa" +
                "&api=https://10.0.0.5:3001/api"
        )
        assertFalse(payload.tls)
        assertEquals("mdy_dev_aaaaaaaaaaaaaaaa", payload.key)
        assertNull(payload.token)
        assertFalse(payload.isEnrollment)
        assertNull(payload.name)
        assertNull(payload.caUrl)
    }

    @Test
    fun tlsDefaultsToFalseWhenAbsent() {
        val payload = ok("madhyamas://connect?host=h&port=8888&key=mdy_dev_k")
        assertFalse(payload.tls)
    }

    @Test
    fun emptyNameBecomesNull() {
        val payload = ok("madhyamas://connect?host=h&port=1&key=mdy_dev_k&name=%20%20")
        assertNull(payload.name)
    }

    @Test
    fun schemeAndLinkHostAreCaseInsensitive() {
        val payload = ok("MADHYAMAS://Connect?host=h&port=1&key=mdy_dev_k")
        assertEquals("h", payload.host)
    }

    @Test
    fun portBoundariesAccepted() {
        assertEquals(1, ok("madhyamas://connect?host=h&port=1&key=mdy_dev_k").port)
        assertEquals(65535, ok("madhyamas://connect?host=h&port=65535&key=mdy_dev_k").port)
    }

    // ---------- error arms ----------

    @Test
    fun rejectsEmptyLink() {
        assertTrue(err("").contains("Empty"))
    }

    @Test
    fun rejectsNullLink() {
        assertTrue(err(null).contains("Empty"))
    }

    @Test
    fun rejectsMalformedUri() {
        assertTrue(err("madhyamas://connect?host=%zz&port=1&key=mdy_dev_k").isEmpty().not())
    }

    @Test
    fun rejectsWrongScheme() {
        assertTrue(err("https://connect?host=h&port=1&key=mdy_dev_k").contains("Not a Madhyamas"))
    }

    @Test
    fun rejectsWrongLinkHost() {
        assertTrue(err("madhyamas://pair?host=h&port=1&key=mdy_dev_k").contains("Not a Madhyamas"))
    }

    @Test
    fun rejectsMissingHostParam() {
        assertTrue(err("madhyamas://connect?port=1&key=mdy_dev_k").contains("host"))
    }

    @Test
    fun rejectsBlankHostParam() {
        assertTrue(err("madhyamas://connect?host=%20&port=1&key=mdy_dev_k").contains("host"))
    }

    @Test
    fun rejectsMissingPort() {
        assertTrue(err("madhyamas://connect?host=h&key=mdy_dev_k").contains("port"))
    }

    @Test
    fun rejectsNonNumericPort() {
        assertTrue(err("madhyamas://connect?host=h&port=abc&key=mdy_dev_k").contains("port"))
    }

    @Test
    fun rejectsPortZero() {
        assertTrue(err("madhyamas://connect?host=h&port=0&key=mdy_dev_k").contains("range"))
    }

    @Test
    fun rejectsPortAboveRange() {
        assertTrue(err("madhyamas://connect?host=h&port=65536&key=mdy_dev_k").contains("range"))
    }

    @Test
    fun rejectsBadTlsValue() {
        assertTrue(err("madhyamas://connect?host=h&port=1&tls=2&key=mdy_dev_k").contains("tls"))
    }

    @Test
    fun rejectsBothTokenAndKey() {
        assertTrue(
            err("madhyamas://connect?host=h&port=1&token=mdy_enroll_a&key=mdy_dev_b&api=http://x/api")
                .contains("both")
        )
    }

    @Test
    fun rejectsNoCredential() {
        assertTrue(err("madhyamas://connect?host=h&port=1").contains("no credential"))
    }

    @Test
    fun rejectsWrongTokenPrefix() {
        assertTrue(
            err("madhyamas://connect?host=h&port=1&token=mdy_dev_a&api=http://x/api")
                .contains("token format")
        )
    }

    @Test
    fun rejectsWrongKeyPrefix() {
        assertTrue(err("madhyamas://connect?host=h&port=1&key=mdy_enroll_a").contains("key format"))
    }

    @Test
    fun rejectsTokenWithoutApi() {
        assertTrue(
            err("madhyamas://connect?host=h&port=1&token=mdy_enroll_a").contains("API URL")
        )
    }

    @Test
    fun rejectsNonHttpCa() {
        assertTrue(
            err(
                "madhyamas://connect?host=h&port=1&key=mdy_dev_k" +
                    "&ca=ftp://example.com/ca"
            ).contains("ca parameter")
        )
    }

    @Test
    fun rejectsNonHttpApi() {
        assertTrue(
            err(
                "madhyamas://connect?host=h&port=1&key=mdy_dev_k" +
                    "&api=ftp://example.com/api"
            ).contains("api parameter")
        )
    }

    // ---------- query decoding ----------

    @Test
    fun decodesUrlEncodedValues() {
        val params = ConnectUriParser.parseQuery("a=one%20two&b=x%26y&c")
        assertEquals("one two", params["a"])
        assertEquals("x&y", params["b"])
        assertNull(params["c"])
    }

    @Test
    fun parseQueryHandlesEmptyInput() {
        assertTrue(ConnectUriParser.parseQuery(null).isEmpty())
        assertTrue(ConnectUriParser.parseQuery("").isEmpty())
    }

    @Test
    fun parseNeverThrowsOnGarbage() {
        // Fuzz-ish inputs: none of these may throw.
        for (raw in listOf(
            "madhyamas://",
            "madhyamas://connect",
            "madhyamas://connect?",
            "::::",
            "madhyamas://connect?=&=&",
            "madhyamas://connect?token=",
            "madhyamas://connect?host=h&port=99999999999999999999&key=mdy_dev_k"
        )) {
            assertNotNull(ConnectUriParser.parse(raw))
        }
    }
}
